//! Top-level pipeline orchestration.
//!
//! Glues [PIPELINE-DISCOVER-FILES], [PIPELINE-NORMALIZE-AST],
//! [PIPELINE-FINGERPRINT-MERKLE], [PIPELINE-CLUSTER-EXACT], and
//! [PIPELINE-RANK-WORST-FIRST] together behind a single entry point used by
//! the CLI and (later) the MCP/LSP daemon. Exclusion policy
//! ([EXCLUSION-CONFIG]) is applied here: `exclude` filters discovery,
//! `report_hide` flows into the renderer so hidden-only clusters are
//! omitted.

use std::{collections::HashMap, fs, path::Path};

use crate::{
    ast::NormalizedNode,
    cluster::build_ranked_fused_clusters,
    config::ExclusionConfig,
    discover::{discover_files, DiscoveredFile},
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
    /// Optional explicit path to a `.codededup.toml` config. When
    /// `None`, the pipeline looks for one in `root` and falls back to
    /// the empty config when absent ([EXCLUSION-CONFIG]).
    pub config_path: Option<std::path::PathBuf>,
}

/// Runs the full analysis pipeline and returns a rendered report.
///
/// # Errors
///
/// Returns [`CoreError::Io`] when a discovered source file cannot be read,
/// [`CoreError::ConfigParse`] / [`CoreError::ConfigPattern`] when the
/// exclusion config is malformed, and propagates any [`CoreError`] from
/// the language parser.
pub fn run(config: &PipelineConfig) -> Result<Report, CoreError> {
    let exclusion = load_exclusion_config(config)?;
    let parsers: Vec<Box<dyn LanguageParser>> = default_parsers();
    let extension_to_language = build_extension_map(&parsers);
    let discovery = discover_files(&config.root, &extension_to_language, &exclusion);

    tracing::info!(
        file_count = discovery.files.len(),
        lang_count = parsers.len(),
        config_source = %exclusion.source_path().display(),
        "file discovery complete",
    );

    let (all_fingerprints, all_trees) = fingerprint_corpus(&discovery.files, &parsers, config)?;

    tracing::info!(
        fingerprint_count = all_fingerprints.len(),
        "fingerprinting complete",
    );

    let signatures = build_signatures(&all_fingerprints, &all_trees);
    let lsh_pairs = band_collisions(&signatures);
    let pairs = candidate_pairs(&all_fingerprints, &signatures, &lsh_pairs);
    tracing::info!(
        signature_count = signatures.len(),
        lsh_pair_count = lsh_pairs.len(),
        candidate_pair_count = pairs.len(),
        "LSH + candidate union complete",
    );

    let fused_clusters = cluster_by_transitive_closure(&pairs);
    let clusters = build_ranked_fused_clusters(&all_fingerprints, &fused_clusters);
    tracing::info!(cluster_count = clusters.len(), "clustering complete");

    let file_languages = build_file_language_map(&discovery.files);
    Ok(render_report(
        &clusters,
        &discovery.registry,
        &file_languages,
        discovery.files.len(),
        config.min_nodes,
        &config.root,
        &exclusion,
    ))
}

/// Builds a `FileId → language_id` map so the renderer can apply
/// per-language `report_hide` overlays ([EXCLUSION-CONFIG]).
fn build_file_language_map(
    files: &[DiscoveredFile],
) -> HashMap<crate::state::FileId, &'static str> {
    let mut out: HashMap<crate::state::FileId, &'static str> = HashMap::new();
    for file in files {
        let _previous = out.insert(file.file_id, file.language);
    }
    out
}

/// Resolves `config.config_path` (explicit override) or falls back to
/// `DEFAULT_CONFIG_FILENAME` in the scan root.
fn load_exclusion_config(config: &PipelineConfig) -> Result<ExclusionConfig, CoreError> {
    if let Some(explicit) = &config.config_path {
        return ExclusionConfig::load(explicit);
    }
    ExclusionConfig::discover(&config.root)
}

/// Parses every discovered file and collects its structural + sibling
/// fingerprints plus the normalised tree kept for token extraction.
fn fingerprint_corpus(
    files: &[DiscoveredFile],
    parsers: &[Box<dyn LanguageParser>],
    config: &PipelineConfig,
) -> Result<(Vec<Fingerprint>, Vec<NormalizedNode>), CoreError> {
    let min_nodes_usize = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    let mut all_fingerprints: Vec<Fingerprint> = Vec::new();
    let mut all_trees: Vec<NormalizedNode> = Vec::with_capacity(files.len());
    for discovered in files {
        let Some(parser) = parser_for_language(parsers, discovered.language) else {
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
    Ok((all_fingerprints, all_trees))
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
/// of files is small compared to the number of fingerprints.
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
/// small to produce any). Every slot saturates at `u64::MAX`.
fn default_signature() -> Signature {
    [u64::MAX; crate::lsh::SIGNATURE_LEN]
}

/// Reads a source file into bytes.
fn read_source(path: &Path) -> Result<Vec<u8>, CoreError> {
    fs::read(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Returns the registered language parsers in a stable order
/// (implements [PIPELINE-LANG-TRAIT]).
fn default_parsers() -> Vec<Box<dyn LanguageParser>> {
    vec![
        Box::new(CSharpParser::new()),
        Box::new(RustParser::new()),
        Box::new(PythonParser::new()),
    ]
}

/// Builds a lowercase-extension → language-id lookup from the parser
/// registry. Returning the language id (not a parser index) lets
/// [`discover_files`] check [`ExclusionConfig`] before the parser is
/// selected.
fn build_extension_map(parsers: &[Box<dyn LanguageParser>]) -> HashMap<String, &'static str> {
    let mut out: HashMap<String, &'static str> = HashMap::new();
    for parser in parsers {
        for extension in parser.file_extensions() {
            let _previous = out.insert((*extension).to_lowercase(), parser.id());
        }
    }
    out
}

/// Returns the parser whose `id()` matches `language`.
fn parser_for_language<'a>(
    parsers: &'a [Box<dyn LanguageParser>],
    language: &str,
) -> Option<&'a dyn LanguageParser> {
    parsers
        .iter()
        .find(|parser| parser.id() == language)
        .map(|boxed| &**boxed)
}
