//! Top-level pipeline orchestration.
//!
//! Glues [PIPELINE-DISCOVER-FILES], [PIPELINE-NORMALIZE-AST],
//! [PIPELINE-FINGERPRINT-MERKLE], [PIPELINE-CLUSTER-EXACT], and
//! [PIPELINE-RANK-WORST-FIRST] together behind a single entry point used by
//! the CLI and (later) the MCP/LSP daemon.

use std::{fs, path::Path};

use crate::{
    cluster::build_ranked_clusters,
    discover::discover_files,
    error::CoreError,
    fingerprint::collect_fingerprints,
    lang::{csharp::CSharpParser, LanguageParser},
    report::{render_report, Report},
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
    let parser = CSharpParser::new();
    let accepted_extensions: Vec<&str> = parser.file_extensions().to_vec();
    let discovery = discover_files(&config.root, &accepted_extensions);

    tracing::info!(
        file_count = discovery.files.len(),
        lang = parser.id(),
        "file discovery complete",
    );

    let mut all_fingerprints = Vec::new();
    let min_nodes_usize = usize::try_from(config.min_nodes).unwrap_or(usize::MAX);
    for discovered in &discovery.files {
        let source = read_source(&discovered.path)?;
        let normalised = parser.parse_and_normalize(&source, discovered.file_id)?;
        let mut fingerprints = collect_fingerprints(&normalised, min_nodes_usize);
        all_fingerprints.append(&mut fingerprints);
    }

    tracing::info!(
        fingerprint_count = all_fingerprints.len(),
        "fingerprinting complete",
    );

    let clusters = build_ranked_clusters(all_fingerprints);
    tracing::info!(cluster_count = clusters.len(), "clustering complete");

    Ok(render_report(
        &clusters,
        &discovery.registry,
        discovery.files.len(),
        config.min_nodes,
        &config.root,
    ))
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
