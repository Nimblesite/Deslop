//! Free-function helpers shared by [`super::session::AnalysisSession`].
//!
//! Split out of `session.rs` purely to keep that file within the
//! 500-line budget mandated by `CLAUDE.md`. Nothing here is a public
//! surface — every helper is `pub(super)` so the module boundary
//! preserves encapsulation.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    cluster::encode_short_id,
    embedding::{EmbeddingMode, EmbeddingProvider, OllamaModelInfo, StubProvider},
    fingerprint::collect_fingerprints,
    lang::LanguageParser,
    pipeline::{EmbeddingSettings, PipelineSession},
    report::{Report, ReportCluster},
    sibling::collect_sibling_fingerprints,
    state::FileRegistry,
};

use super::{
    errors::LiveError,
    session::EmbeddingProgressReporter,
    wire::{EmbeddingModelInfo, EmbeddingPhase, EmbeddingProgress},
};

const LIVE_EMBEDDING_BATCH_SLEEP: Duration = Duration::from_millis(10);

// [LIVE-EMBEDDING-CONSENT] Live embedding refreshes yield between
// provider batches so the editor/agent loop keeps breathing.
pub(super) fn live_batch_yield(mode: EmbeddingMode) -> Option<Duration> {
    if matches!(mode, EmbeddingMode::Off) {
        None
    } else {
        Some(LIVE_EMBEDDING_BATCH_SLEEP)
    }
}

// [LIVE-EMBEDDING-CONSENT] Progress is observable while the selected
// model refresh is still below the current report generation.
pub(super) fn report_running_progress(
    reporter: &Option<EmbeddingProgressReporter>,
    provider_id: &str,
    model_id: &str,
    done: usize,
    total: u64,
) {
    if let Some(reporter) = reporter {
        reporter(EmbeddingProgress {
            phase: EmbeddingPhase::Running,
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            done: u64::try_from(done).unwrap_or(u64::MAX).min(total),
            total,
            message: None,
        });
    }
}

/// Calls [`PipelineSession::initialise`] with the supplied embedding
/// provider. Extracted so [`super::session::AnalysisSession::new`]
/// stays under the 20-line function budget.
pub(super) fn initialise_pipeline(
    root: PathBuf,
    min_nodes: u32,
    incremental: bool,
    config_path: Option<PathBuf>,
    mode: EmbeddingMode,
    provider: &dyn EmbeddingProvider,
) -> Result<(PipelineSession, Report), LiveError> {
    let embedding = EmbeddingSettings {
        mode,
        provider: Some(provider),
        batch_yield: None,
        progress: None,
    };
    Ok(PipelineSession::initialise(
        root,
        min_nodes,
        incremental,
        config_path,
        embedding,
    )?)
}

/// Truncates `clusters` to `max_results` when supplied.
pub(super) fn truncate(clusters: &mut Vec<ReportCluster>, max_results: Option<usize>) {
    if let Some(cap) = max_results {
        clusters.truncate(cap);
    }
}

/// Returns `true` when at least one occurrence in `cluster` lives in
/// `path`.
pub(super) fn cluster_touches_path(cluster: &ReportCluster, path: &Path) -> bool {
    cluster
        .occurrences
        .iter()
        .any(|occurrence| occurrence_path_matches(occurrence.path.as_path(), path))
}

/// Returns `true` when at least one occurrence overlaps the given
/// byte range in `path`.
pub(super) fn cluster_overlaps_range(
    cluster: &ReportCluster,
    path: &Path,
    start_byte: usize,
    end_byte: usize,
) -> bool {
    cluster.occurrences.iter().any(|occurrence| {
        occurrence_path_matches(occurrence.path.as_path(), path)
            && ranges_overlap(
                occurrence.start_byte,
                occurrence.end_byte,
                start_byte,
                end_byte,
            )
    })
}

/// Returns the smallest start byte across the occurrences of
/// `cluster` that live in `path`.
pub(super) fn earliest_byte_for_path(cluster: &ReportCluster, path: &Path) -> usize {
    cluster
        .occurrences
        .iter()
        .filter(|occurrence| occurrence_path_matches(occurrence.path.as_path(), path))
        .map(|occurrence| occurrence.start_byte)
        .min()
        .unwrap_or(usize::MAX)
}

/// Compares an occurrence path against a query path, accepting either
/// a full match or a suffix match (so workspace-relative queries find
/// absolute occurrence paths).
fn occurrence_path_matches(occurrence: &Path, query: &Path) -> bool {
    occurrence == query || occurrence.ends_with(query) || query.ends_with(occurrence)
}

/// Inclusive/exclusive byte-range overlap test.
fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start < right_end && right_start < left_end
}

/// Parses `snippet` and returns the structural hashes for every
/// subtree that meets `min_nodes`.
pub(super) fn parse_and_hash_snippet(
    parser: &dyn LanguageParser,
    snippet: &str,
    min_nodes: u32,
) -> Result<Vec<[u8; 32]>, LiveError> {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("__snippet__"));
    let normalised = parser
        .parse_and_normalize(snippet.as_bytes(), file_id)
        .map_err(|source| LiveError::UnparseableInput {
            path: None,
            start_byte: 0,
            end_byte: snippet.len(),
            message: source.to_string(),
        })?;
    let min_usize = usize::try_from(min_nodes).unwrap_or(usize::MAX);
    let mut hashes: Vec<[u8; 32]> = collect_fingerprints(&normalised, min_usize)
        .into_iter()
        .map(|fingerprint| fingerprint.hash)
        .collect();
    hashes.extend(
        collect_sibling_fingerprints(&normalised, min_usize)
            .into_iter()
            .map(|fingerprint| fingerprint.hash),
    );
    Ok(hashes)
}

/// Returns `true` when `cluster.id` matches any of the provided
/// snippet hashes when projected through the same short-id encoding
/// used by [`crate::cluster`].
pub(super) fn cluster_matches_any_hash(
    cluster: &ReportCluster,
    snippet_hashes: &[[u8; 32]],
) -> bool {
    snippet_hashes
        .iter()
        .any(|hash| encode_short_id(*hash) == cluster.id)
}

/// Returns the hard-coded model info for the built-in stub provider.
pub(super) fn stub_model_info() -> EmbeddingModelInfo {
    let dimensions = StubProvider::new()
        .embed("")
        .map(|vector| vector.len())
        .ok();
    EmbeddingModelInfo {
        provider_id: crate::embedding::STUB_PROVIDER_ID.to_owned(),
        model_id: "blake3-stub".to_owned(),
        model_version: Some("v1".to_owned()),
        dimensions,
        recommended: false,
        reachable: true,
    }
}

/// Translates the Ollama tag list into [`EmbeddingModelInfo`] entries.
pub(super) fn append_ollama_models(
    out: &mut Vec<EmbeddingModelInfo>,
    models: Vec<OllamaModelInfo>,
) {
    for entry in models {
        out.push(EmbeddingModelInfo {
            provider_id: crate::embedding::DEFAULT_PROVIDER_ID.to_owned(),
            model_id: entry.bare_id,
            model_version: Some(entry.digest),
            dimensions: None,
            recommended: entry.is_embedding_model,
            reachable: true,
        });
    }
}
