//! Batch pipeline entry point used by the CLI.

use std::{collections::HashMap, path::Path};

use crate::{
    cluster::build_ranked_fused_clusters,
    config::ExclusionConfig,
    discover::{discover_files, DiscoveredFile},
    error::CoreError,
    lsh::band_collisions,
    pair::{candidate_pairs, cluster_by_transitive_closure},
    render::render_ast_dump,
    report::{render_report, Report, ReportInputs},
    state::FileId,
};

use super::{
    config::PipelineConfig,
    corpus::{build_extension_map, default_parsers, fingerprint_corpus, read_source},
    embedding_pass::run_embedding_pass,
    signatures::build_signatures,
};

/// Runs the full analysis pipeline and returns a rendered report.
///
/// # Errors
///
/// Returns [`CoreError::Io`] when a discovered source file cannot be read,
/// [`CoreError::ConfigParse`] / [`CoreError::ConfigPattern`] when the
/// exclusion config is malformed, and propagates any [`CoreError`] from
/// the language parser.
pub fn run(config: &PipelineConfig<'_>) -> Result<Report, CoreError> {
    let exclusion = load_exclusion_config(config)?;
    let parsers = default_parsers();
    let extension_to_language = build_extension_map(&parsers);
    let discovery = discover_files(&config.root, &extension_to_language, &exclusion);

    tracing::info!(
        file_count = discovery.files.len(),
        lang_count = parsers.len(),
        config_source = %exclusion.source_path().display(),
        "file discovery complete",
    );

    let corpus = fingerprint_corpus(&discovery.files, &parsers, config)?;

    tracing::info!(
        fingerprint_count = corpus.fingerprints.len(),
        files_cache_hit = corpus.cache_stats.hits,
        files_cache_miss = corpus.cache_stats.misses,
        incremental = config.incremental,
        "fingerprinting complete",
    );

    let signatures = build_signatures(&corpus.fingerprints, &corpus.trees);
    let lsh_pairs = band_collisions(&signatures);

    let embedding_outcome = run_embedding_pass(config, &corpus)?;
    let pairs = candidate_pairs(
        &corpus.fingerprints,
        &signatures,
        &lsh_pairs,
        &embedding_outcome.pairs,
    );
    tracing::info!(
        signature_count = signatures.len(),
        lsh_pair_count = lsh_pairs.len(),
        embedding_pair_count = embedding_outcome.pairs.len(),
        candidate_pair_count = pairs.len(),
        "LSH + candidate union complete",
    );

    let fused_clusters = cluster_by_transitive_closure(&pairs);
    let clusters = build_ranked_fused_clusters(&corpus.fingerprints, &fused_clusters);
    tracing::info!(cluster_count = clusters.len(), "clustering complete");

    let file_languages = build_file_language_map(&discovery.files);
    Ok(render_report(ReportInputs {
        clusters: &clusters,
        registry: &discovery.registry,
        file_languages: &file_languages,
        files_analysed: discovery.files.len(),
        min_nodes: config.min_nodes,
        scan_root: &config.root,
        exclusion: &exclusion,
        embedding_provenance: embedding_outcome.provenance,
        cache_stats: corpus.cache_stats,
    }))
}

/// Parses `path` with the matching language parser and returns a
/// deterministic text dump of the normalised AST ([PIPELINE-NORMALIZE-AST]).
/// Used by the CLI's `--debug-ast` flag — developer tool, not part
/// of the analysis pipeline.
///
/// # Errors
///
/// - [`CoreError::Io`] if the file cannot be read.
/// - [`CoreError::UnsupportedExtension`] if no registered parser
///   claims the file's extension.
/// - Any parser error forwarded from
///   [`crate::lang::LanguageParser::parse_and_normalize`].
pub fn debug_ast_dump(path: &Path) -> Result<String, CoreError> {
    let parsers = default_parsers();
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_lowercase)
        .unwrap_or_default();
    let Some(parser) = parsers
        .iter()
        .find(|p| p.file_extensions().iter().any(|e| *e == extension))
    else {
        return Err(CoreError::UnsupportedExtension {
            path: path.to_path_buf(),
        });
    };
    let source = read_source(path)?;
    let mut registry = crate::state::FileRegistry::new();
    let file_id = registry.register(path.to_path_buf());
    let tree = parser.parse_and_normalize(&source, file_id)?;
    Ok(render_ast_dump(&tree))
}

/// Builds a `FileId → language_id` map so the renderer can apply
/// per-language `report_hide` overlays ([EXCLUSION-CONFIG]).
fn build_file_language_map(files: &[DiscoveredFile]) -> HashMap<FileId, &'static str> {
    let mut out: HashMap<FileId, &'static str> = HashMap::new();
    for file in files {
        let _previous = out.insert(file.file_id, file.language);
    }
    out
}

/// Resolves `config.config_path` (explicit override) or falls back to
/// `DEFAULT_CONFIG_FILENAME` in the scan root.
fn load_exclusion_config(config: &PipelineConfig<'_>) -> Result<ExclusionConfig, CoreError> {
    if let Some(explicit) = &config.config_path {
        return ExclusionConfig::load(explicit);
    }
    ExclusionConfig::discover(&config.root)
}
