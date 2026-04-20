//! Top-level pipeline orchestration.
//!
//! Glues [PIPELINE-DISCOVER-FILES], [PIPELINE-NORMALIZE-AST],
//! [PIPELINE-FINGERPRINT-MERKLE], [PIPELINE-CLUSTER-EXACT], and
//! [PIPELINE-RANK-WORST-FIRST] together behind a single entry point used by
//! the CLI and (later) the MCP/LSP daemon.

use std::{fs, path::Path};

use crate::{
    ast::NormalizedNode,
    cluster::build_ranked_fused_clusters,
    discover::discover_files,
    error::CoreError,
    fingerprint::{collect_fingerprints, Fingerprint},
    lang::{csharp::CSharpParser, python::PythonParser, rust_lang::RustParser, LanguageParser},
    lsh::{band_collisions, minhash_signature, Signature},
    pair::{candidate_pairs, cluster_by_transitive_closure},
    report::{render_report, Report},
    sibling::collect_sibling_fingerprints,
    tokens::{kgrams, token_stream_for_fingerprint, KGRAM_WIDTH},
};

/// Configuration for a single pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Root directory to analyse.
    pub root: std::path::PathBuf,
    /// Minimum AST subtree node count to consider a clone candidate
    /// ([DECISION-MIN-NODES]).
    pub min_nodes: u32,
}

/// Runs the full analysis pipeline and returns a rendered report.
///
/// # Errors
///
/// Returns [`CoreError::Io`] when a discovered source file cannot be read,
/// and propagates any [`CoreError`] from the language parser.
pub fn run(config: &PipelineConfig) -> Result<Report, CoreError> {
    let parsers: Vec<Box<dyn LanguageParser>> = default_parsers();
    let accepted_extensions: Vec<&str> = parsers
        .iter()
        .flat_map(|parser| parser.file_extensions().iter().copied())
        .collect();
    let discovery = discover_files(&config.root, &accepted_extensions);

    tracing::info!(
        file_count = discovery.files.len(),
        lang_count = parsers.len(),
        "file discovery complete",
    );

    let min_nodes_usize = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let mut all_fingerprints: Vec<Fingerprint> = Vec::new();
    let mut all_trees: Vec<NormalizedNode> = Vec::with_capacity(discovery.files.len());
    for discovered in &discovery.files {
        let Some(parser) = parser_for_extension(&parsers, &discovered.extension) else {
            continue;
        };
        let source = read_source(&discovered.path)?;
        let normalised = parser.parse_and_normalize(&source, discovered.file_id)?;
        all_fingerprints.append(&mut collect_fingerprints(&normalised, min_nodes_usize));
        all_fingerprints.append(&mut collect_sibling_fingerprints(
            &normalised,
            min_nodes_usize,
        ));
        all_trees.push(normalised);
    }

    tracing::info!(
        fingerprint_count = all_fingerprints.len(),
        "fingerprinting complete",
    );

    let signatures = build_signatures(&all_fingerprints, &all_trees);
    tracing::info!(
        signature_count = signatures.len(),
        "minhash signatures computed",
    );

    let lsh_pairs = band_collisions(&signatures);
    let pairs = candidate_pairs(&all_fingerprints, &signatures, &lsh_pairs);
    tracing::info!(
        lsh_pair_count = lsh_pairs.len(),
        candidate_pair_count = pairs.len(),
        "LSH + candidate union complete",
    );

    let fused_clusters = cluster_by_transitive_closure(&pairs);
    let clusters = build_ranked_fused_clusters(&all_fingerprints, &fused_clusters);
    tracing::info!(cluster_count = clusters.len(), "clustering complete");

    Ok(render_report(
        &clusters,
        &discovery.registry,
        discovery.files.len(),
        config.min_nodes,
        &config.root,
    ))
}

/// Computes a `MinHash` signature per fingerprint. Each signature is
/// generated from k-grams of the normalised token stream of the fingerprint's
/// subtree — token Jaccard then acts as the Type-3 recall signal per
/// [DECISION-TYPE3-TWO-PASS].
fn build_signatures(fingerprints: &[Fingerprint], trees: &[NormalizedNode]) -> Vec<Signature> {
    let mut signatures: Vec<Signature> = Vec::with_capacity(fingerprints.len());
    for fingerprint in fingerprints {
        let signature = tree_for_file(trees, fingerprint)
            .and_then(|root| token_stream_for_fingerprint(root, fingerprint))
            .map_or_else(default_signature, |tokens| signature_for_tokens(&tokens));
        signatures.push(signature);
    }
    signatures
}

/// Returns the normalised AST root for `fingerprint`'s file by scanning
/// the per-run tree list. O(n) per lookup; acceptable because the number
/// of files is small compared to the number of fingerprints, and the
/// signature pass iterates fingerprints linearly regardless.
fn tree_for_file<'a>(
    trees: &'a [NormalizedNode],
    fingerprint: &Fingerprint,
) -> Option<&'a NormalizedNode> {
    trees
        .iter()
        .find(|tree| tree.file_id == fingerprint.file_id)
}

/// Produces a signature from a prepared token stream using the configured
/// k-gram width.
fn signature_for_tokens(tokens: &[&'static str]) -> Signature {
    let grams = kgrams(tokens, KGRAM_WIDTH);
    let gram_slices: Vec<&[&'static str]> = grams.into_iter().collect();
    minhash_signature(&gram_slices)
}

/// Default signature used when no k-grams are available (subtree too
/// small to produce any). Every slot saturates at `u64::MAX` so it agrees
/// only with other equally-empty subtrees and therefore does not generate
/// spurious LSH collisions with real content.
fn default_signature() -> Signature {
    [u64::MAX; crate::lsh::SIGNATURE_LEN]
}

/// Reads a source file into bytes. Separate function so fingerprinting
/// callers can refine it later (memory mapping, caching, etc.) without
/// rewriting the orchestrator.
fn read_source(path: &Path) -> Result<Vec<u8>, CoreError> {
    fs::read(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Returns the registered language parsers in a stable order. Adding a
/// language is one entry here plus a `LanguageParser` impl in
/// [`crate::lang`] — nothing else changes (implements
/// [PIPELINE-LANG-TRAIT]).
fn default_parsers() -> Vec<Box<dyn LanguageParser>> {
    vec![
        Box::new(CSharpParser::new()),
        Box::new(RustParser::new()),
        Box::new(PythonParser::new()),
    ]
}

/// Routes a discovered file extension to the first parser that accepts
/// it. Extensions are compared case-insensitively to match the
/// lowercasing in [`crate::discover`].
fn parser_for_extension<'a>(
    parsers: &'a [Box<dyn LanguageParser>],
    extension: &str,
) -> Option<&'a dyn LanguageParser> {
    parsers
        .iter()
        .find(|parser| {
            parser
                .file_extensions()
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
        .map(|boxed| &**boxed)
}
