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

/// Delay inserted between live embedding batches.
const LIVE_EMBEDDING_BATCH_SLEEP: Duration = Duration::from_millis(10);

/// [LIVE-EMBEDDING-CONSENT] Returns the live embedding batch-yield
/// delay for modes that actually call an embedding provider.
pub(super) fn live_batch_yield(mode: EmbeddingMode) -> Option<Duration> {
    if matches!(mode, EmbeddingMode::Off) {
        None
    } else {
        Some(LIVE_EMBEDDING_BATCH_SLEEP)
    }
}

/// [LIVE-EMBEDDING-CONSENT] Reports running progress while selected
/// model refresh work is still below the current report generation.
pub(super) fn report_running_progress(
    reporter: Option<&EmbeddingProgressReporter>,
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

/// Collapses ranked range hits to one cluster per physical span.
pub(super) fn collapse_overlapping_clusters_for_range(
    clusters: Vec<ReportCluster>,
    path: &Path,
    start_byte: usize,
    end_byte: usize,
) -> Vec<ReportCluster> {
    let mut kept = Vec::with_capacity(clusters.len());
    let mut kept_ranges = Vec::new();
    for cluster in clusters {
        let Some((start, end)) = cluster_overlap_range(&cluster, path, start_byte, end_byte) else {
            continue;
        };
        if kept_ranges
            .iter()
            .any(|(left, right)| ranges_overlap(*left, *right, start, end))
        {
            continue;
        }
        kept_ranges.push((start, end));
        kept.push(cluster);
    }
    kept
}

/// Returns the widest occurrence in `cluster` that overlaps the query.
fn cluster_overlap_range(
    cluster: &ReportCluster,
    path: &Path,
    start_byte: usize,
    end_byte: usize,
) -> Option<(usize, usize)> {
    cluster
        .occurrences
        .iter()
        .filter(|occurrence| occurrence_path_matches(occurrence.path.as_path(), path))
        .filter(|occurrence| {
            ranges_overlap(
                occurrence.start_byte,
                occurrence.end_byte,
                start_byte,
                end_byte,
            )
        })
        .max_by_key(|occurrence| occurrence.end_byte.saturating_sub(occurrence.start_byte))
        .map(|occurrence| (occurrence.start_byte, occurrence.end_byte))
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

/// [LIVE-CACHE-SEED] Default name of the cache file the LSP writes
/// after every analysis pass. The MCP and any cache-seed startup path
/// read from this same file.
pub(super) const STATE_FILE_NAME: &str = "live-report.json";

/// [LIVE-STATE-FILE] Writes `bytes` to `{dir}/live-report.json` via an
/// atomic tmp-then-rename so readers never see a partial file.
pub(super) fn atomic_write_json(dir: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = dir.join("live-report.json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, dir.join(STATE_FILE_NAME))
}

/// [LIVE-STATE-FILE] Atomically writes `report` to
/// `{root}/.deslop-cache/live-report.json`. Best-effort: failures are
/// logged at `warn` and never propagated.
pub(super) fn persist_state_file(root: &Path, report: &Report, generation: u64) {
    let cache_dir = root.join(crate::embedding::cache::DEFAULT_CACHE_DIR_NAME);
    if let Err(error) = std::fs::create_dir_all(&cache_dir) {
        tracing::warn!(%error, "state_file_dir_create_failed");
        return;
    }
    match serde_json::to_vec(report) {
        Err(error) => tracing::warn!(%error, "state_file_serialize_failed"),
        Ok(bytes) => {
            if let Err(error) = atomic_write_json(&cache_dir, &bytes) {
                tracing::warn!(%error, "state_file_atomic_write_failed");
            } else {
                tracing::info!(generation, "state_file_written");
            }
        }
    }
}

/// [LIVE-CACHE-SEED] Best-effort load of `{root}/.deslop-cache/live-report.json`.
/// Returns `None` for a missing file (cold start) and for any parse or
/// I/O failure (the caller falls back to running a fresh full pass).
pub(super) fn try_load_cached_report(root: &Path) -> Option<Report> {
    let path = root
        .join(crate::embedding::cache::DEFAULT_CACHE_DIR_NAME)
        .join(STATE_FILE_NAME);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(%error, path = %path.display(), "cached_report_read_failed");
            }
            return None;
        }
    };
    match serde_json::from_slice::<Report>(&bytes) {
        Ok(mut report) => {
            normalize_cache_seed_stats(&mut report);
            Some(report)
        }
        Err(error) => {
            tracing::warn!(%error, path = %path.display(), "cached_report_parse_failed");
            delete_incompatible_cached_report(&path);
            None
        }
    }
}

fn delete_incompatible_cached_report(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => tracing::warn!(path = %path.display(), "cached_report_incompatible_deleted"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            tracing::warn!(%error, path = %path.display(), "cached_report_delete_failed");
        }
    }
}

/// Folds any cache misses recorded during the cold pass into the hits
/// counter when seeding from a cached report. Cache-seeded sessions
/// surface as "all hits" to the editor — the misses were paid by the
/// CLI run that wrote the cache, not by this session.
fn normalize_cache_seed_stats(report: &mut Report) {
    if report.cache_stats.misses == 0 {
        return;
    }
    report.cache_stats.hits = report
        .cache_stats
        .hits
        .saturating_add(report.cache_stats.misses);
    report.cache_stats.misses = 0;
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

#[cfg(test)]
mod tests {
    use super::try_load_cached_report;

    #[test]
    fn incompatible_cached_report_is_deleted_when_cache_seed_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache_dir = temp
            .path()
            .join(crate::embedding::cache::DEFAULT_CACHE_DIR_NAME);
        std::fs::create_dir_all(&cache_dir).expect("cache dir");
        let state_path = cache_dir.join(super::STATE_FILE_NAME);
        std::fs::write(&state_path, br#"{"tool_version":"stale","clusters":[]}"#)
            .expect("write incompatible state");

        assert!(
            state_path.exists(),
            "test setup must start with an incompatible state file"
        );
        assert!(
            try_load_cached_report(temp.path()).is_none(),
            "incompatible cached state must not seed a live session"
        );
        assert!(
            !state_path.exists(),
            "incompatible cached state must be deleted so startup runs a fresh scan"
        );
    }
}
