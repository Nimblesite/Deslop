//! Live analysis session ([LIVE-STATE]).
//!
//! Owns one [`PipelineSession`], the latest rendered [`Report`]
//! snapshot, the monotonic generation counter, and the active embedding
//! provider. The session is the single live struct in the live module
//! — nothing else holds mutable analysis state.

mod embedding;
mod init;
mod queries;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

pub use queries::{report_for_file_in, report_for_range_in};

use super::{
    cache_seed_key::CacheSeedKey,
    errors::LiveError,
    freshness::FreshnessTracker,
    session_helpers::{live_batch_yield, persist_state_file},
    watcher::{publish_exclusion, LiveExclusion},
    wire::{EmbeddingProgress, SessionConfig},
};
use crate::{
    delta::ReportDelta,
    embedding::{EmbeddingMode, EmbeddingProvider},
    pipeline::{language_for_path, EmbeddingSettings, OpenBuffers, PipelineSession},
    report::Report,
};

/// Sink invoked around a `set_embedding_model` swap so transports can
/// forward the progress onto `deslop/embeddingProgress`. The reporter
/// is `Send + Sync` so it survives being moved into a tokio handler.
pub type EmbeddingProgressReporter = Arc<dyn Fn(EmbeddingProgress) + Send + Sync>;

/// Live analysis session. Wraps [`PipelineSession`] with the live-only
/// metadata documented in [LIVE-STATE].
pub struct AnalysisSession {
    /// Workspace root pinned at construction. Mirrored on the session
    /// so query methods work in the cache-seed window before the
    /// background pipeline lands ([LIVE-CACHE-SEED]).
    root: PathBuf,
    /// Subtree-size floor pinned at construction.
    min_nodes: u32,
    /// Underlying analysis state. `None` between cache-seed
    /// construction and the background full-pass install.
    pipeline: Option<PipelineSession>,
    /// File-change paths queued while `pipeline` was `None`. Replayed
    /// in [`Self::install_pipeline`] when the background pass commits.
    pending_changes: Vec<PathBuf>,
    /// Atomic snapshot of the current report.
    latest_report: Arc<Report>,
    /// Lock-free mirror of [`Self::latest_report`] ([LSP-NON-INTERFERENCE]).
    /// The LSP backend reads its additive editor surfaces (diagnostics,
    /// code lens) from this handle without ever awaiting the session
    /// mutex — which a long `apply_changes` pass can hold for seconds on
    /// a churning monorepo. Republished by [`Self::publish_report`] in
    /// lock-step with `latest_report`.
    report_snapshot: Arc<RwLock<Arc<Report>>>,
    /// Lock-free copy of the open buffers [`Self::report_snapshot`] analysed
    /// ([LIVE-OPEN-BUFFERS]), so byte offsets read from that report are
    /// resolved against the text they describe.
    analysed_buffers: Arc<RwLock<Arc<OpenBuffers>>>,
    /// Monotonic generation counter.
    generation: u64,
    /// Active embedding provider.
    embedding_provider: Arc<dyn EmbeddingProvider>,
    /// Currently-active embedding mode.
    embedding_mode: EmbeddingMode,
    /// Whether the session was created with the incremental cache on.
    incremental: bool,
    /// Optional explicit exclusion config path supplied at
    /// construction.
    config_path: Option<PathBuf>,
    /// Optional sink invoked on set-model progress events.
    embedding_progress_reporter: Option<EmbeddingProgressReporter>,
    /// Monotonic id for queued embedding refreshes.
    embedding_refresh_revision: u64,
    /// Exclusion policy shared with the live watcher. Republished
    /// whenever the pipeline resolves or reloads its config so the
    /// watcher's ingest gate and the corpus definition never disagree
    /// ([CONFIG-EXCLUDE-DEPENDENCIES]).
    exclusion: LiveExclusion,
    /// Mtime ledger consulted before every IPC read
    /// ([LIVE-CLUSTER-OFFSET-FRESHNESS]). Refreshed
    /// after every analysis pass so a stale-mtime read forces a
    /// synchronous `apply_changes` ahead of serving the response.
    freshness: FreshnessTracker,
    /// Text of every file an editor holds open ([LIVE-OPEN-BUFFERS]).
    /// A change pass reads these instead of the disk, so the report
    /// describes what the user sees, saved or not. Maintained by the
    /// `*_buffer` methods in `session_buffers.rs`.
    pub(super) buffers: OpenBuffers,
    /// Latest editor version per path edited while no pass could analyse it.
    /// Acknowledged by the cold pass that replays them ([LIVE-OPEN-BUFFERS]).
    pub(super) deferred_acknowledgements: BTreeMap<PathBuf, i32>,
}

impl std::fmt::Debug for AnalysisSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisSession").finish_non_exhaustive()
    }
}

impl AnalysisSession {
    /// Installs (or clears with `None`) the embedding-progress reporter.
    pub fn set_embedding_progress_reporter(&mut self, reporter: Option<EmbeddingProgressReporter>) {
        self.embedding_progress_reporter = reporter;
    }

    /// Returns an `Arc` to the current report snapshot.
    #[must_use]
    pub fn report(&self) -> Arc<Report> {
        Arc::clone(&self.latest_report)
    }

    /// Recomputes admission evidence for two explicitly named occurrences.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::AnalysisNotReady`] while a cache-seeded session
    /// has no installed pipeline and forwards endpoint or provider errors
    /// from [`PipelineSession::compare_pair`].
    pub fn compare_pair(
        &self,
        params: &crate::report::PairComparisonParams,
    ) -> Result<crate::report::PairComparison, LiveError> {
        let pipeline = self.pipeline.as_ref().ok_or(LiveError::AnalysisNotReady)?;
        let provider = (!matches!(self.embedding_mode, EmbeddingMode::Off))
            .then_some(self.embedding_provider.as_ref());
        pipeline.compare_pair(params, provider).map_err(Into::into)
    }

    /// In-process view of the installed pipeline, used by the merge
    /// engine's AST accessors ([AUTOFIX-MERGE]) — `None` during the
    /// cache-seed window. Never serialised to the wire.
    #[must_use]
    pub fn pipeline(&self) -> Option<&PipelineSession> {
        self.pipeline.as_ref()
    }

    /// Swaps in `report` and republishes it to the lock-free snapshot in
    /// lock-step ([LSP-NON-INTERFERENCE]). Every mutation that changes
    /// `latest_report` routes through here so the snapshot the LSP reads
    /// can never drift from the session's authoritative report.
    fn publish_report(&mut self, report: Arc<Report>) {
        self.latest_report = Arc::clone(&report);
        let mut guard = self
            .report_snapshot
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = report;
        *self
            .analysed_buffers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(self.buffers.clone());
    }

    /// Returns the exclusion handle the live watcher must filter with.
    ///
    /// The watcher and the corpus apply one policy. Starting the watcher
    /// with its own config let an opted-in dependency tree be scanned
    /// cold and then ignored live ([CONFIG-EXCLUDE-DEPENDENCIES]).
    #[must_use]
    pub fn exclusion_handle(&self) -> LiveExclusion {
        Arc::clone(&self.exclusion)
    }

    /// Republishes the pipeline's resolved exclusion policy to the
    /// watcher. Called after any pass that may have reloaded config, so
    /// a `.deslop.toml` edit re-scopes ingest with no restart.
    fn republish_exclusion(&self) {
        if let Some(pipeline) = self.pipeline.as_ref() {
            publish_exclusion(&self.exclusion, pipeline.exclusion_handle());
        }
    }

    /// Returns a clone of the lock-free report-snapshot handle. The LSP
    /// backend reads its additive editor surfaces from this without ever
    /// awaiting the session mutex ([LSP-NON-INTERFERENCE]).
    #[must_use]
    pub fn report_snapshot_handle(&self) -> Arc<RwLock<Arc<Report>>> {
        Arc::clone(&self.report_snapshot)
    }

    /// The lock-free open buffers behind [`Self::report_snapshot_handle`].
    #[must_use]
    pub fn analysed_buffers_handle(&self) -> Arc<RwLock<Arc<OpenBuffers>>> {
        Arc::clone(&self.analysed_buffers)
    }

    /// Returns the current generation counter.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the workspace root pinned at construction.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// [LIVE-CACHE-SEED] Returns `true` while the session is serving
    /// query results from the cached `live-report.json` and the
    /// background full pass has not yet committed its pipeline.
    #[must_use]
    pub const fn is_seed_only(&self) -> bool {
        self.pipeline.is_none()
    }

    /// Re-analyses `changed`, swaps the report, bumps the generation,
    /// and returns a [`ReportDelta`] versus the previous snapshot.
    /// During the cache-seed window, queues `changed` and returns an
    /// empty delta — the queued paths are replayed inside
    /// [`Self::install_pipeline`].
    ///
    /// Watched config files (`<root>/.deslop.toml` or the explicit
    /// override) in `changed` are detected inside
    /// [`crate::PipelineSession::update_files`], which reloads the
    /// exclusion config and re-evaluates the existing corpus against
    /// it — dropping newly-excluded files and re-discovering newly
    /// re-included ones ([LIVE-CONFIG-LIVE]).
    ///
    /// # Errors
    ///
    /// Propagates pipeline errors via [`LiveError::Core`]. A bad
    /// `.deslop.toml` (parse / pattern error) is logged and the
    /// previous config is kept — a typo never bricks the daemon.
    pub fn apply_changes(&mut self, changed: &[PathBuf]) -> Result<ReportDelta, LiveError> {
        if self.pipeline.is_none() {
            self.pending_changes.extend(changed.iter().cloned());
            return Ok(ReportDelta::between(
                Some((self.generation, &self.latest_report)),
                self.generation,
                &self.latest_report,
            ));
        }
        let previous = Arc::clone(&self.latest_report);
        let prev_generation = self.generation;
        let Some(next) = self.run_pipeline(changed)? else {
            // Nothing analysed moved, so the report cannot have changed.
            // Publishing a new generation here would wake every subscriber
            // — panel, diagnostics, MCP — to re-fetch an identical report
            //. Report no delta against the generation we kept.
            return Ok(ReportDelta::between(
                Some((prev_generation, &previous)),
                prev_generation,
                &previous,
            ));
        };
        self.generation = self.generation.saturating_add(1);
        // `update_files` reloads the exclusion config when a watched
        // config path moved, so the watcher's copy is refreshed on the
        // same pass rather than one edit later.
        self.republish_exclusion();
        let next_arc = Arc::new(next);
        self.publish_report(Arc::clone(&next_arc));
        // [LIVE-CLUSTER-OFFSET-FRESHNESS] Refresh the mtime ledger so a follow-up
        // read of the same files does not re-trigger this same pass.
        self.freshness
            .record_from_report(&self.root, &self.latest_report);
        // Per-keystroke writes were the dominant source of inotify
        // chatter on large workspaces. The seed-cache file is now
        // persisted only on cold-pass install ([LIVE-SEED-CACHE]); the
        // MCP reads the live state via IPC, not this file.
        Ok(ReportDelta::between(
            Some((prev_generation, &previous)),
            self.generation,
            &next_arc,
        ))
    }

    /// [LIVE-CLUSTER-OFFSET-FRESHNESS] / /.
    ///
    /// Detects files whose on-disk mtime is newer than what the
    /// analyser last observed, and runs a synchronous
    /// [`Self::apply_changes`] pass for those paths before any IPC
    /// read serves cluster occurrences. The cost is one `stat` per
    /// occurrence path per read — well under a millisecond even on
    /// monorepo-scale workspaces — and it eliminates the stale-data
    /// window between a synchronous edit and the next debounced
    /// watcher tick.
    ///
    /// Failures are logged but never propagated: a stale-data read
    /// is bad, but a failed read is worse. The caller continues with
    /// whatever the previous pass produced.
    pub fn refresh_if_stale(&mut self) {
        let stale = self
            .freshness
            .detect_stale_paths(&self.root, &self.latest_report);
        if stale.is_empty() {
            return;
        }
        tracing::debug!(
            stale_count = stale.len(),
            "live_read_freshness_apply_changes",
        );
        match self.apply_changes(&stale) {
            Ok(_delta) => {}
            Err(error) => {
                tracing::warn!(
                    %error,
                    stale_count = stale.len(),
                    "live_read_freshness_refresh_failed; serving prior snapshot",
                );
            }
        }
    }

    /// Returns the resolved session configuration.
    #[must_use]
    pub fn session_config(&self) -> SessionConfig {
        SessionConfig {
            workspace_root: self.root.clone(),
            min_nodes: self.min_nodes,
            languages: self.parser_ids(),
            embedding_provenance: self.latest_report.embedding_provenance.clone(),
            exclusion_config_path: self.config_path.clone(),
            cache_root: crate::paths::cache_dir(&self.root),
            // [CONFIG-INCREMENTAL-OPTOUT] The surfaced value is the
            // *effective* mode — the request gated by the live config —
            // never the raw request: a session asked to persist under
            // `[analysis] incremental = false` runs every pass uncached,
            // and reporting `true` here made the config surface claim a
            // store no pass consults. Before the first pipeline exists
            // nothing has run, so the request is the only honest answer.
            incremental: self
                .pipeline
                .as_ref()
                .map_or(self.incremental, PipelineSession::effective_incremental),
        }
    }

    /// Stable list of registered parser ids. During the cache-seed
    /// window the pipeline isn't installed yet, so fall back to
    /// scanning the seeded report's occurrences and mapping their
    /// extensions to language ids — keeps `session/config` useful
    /// for IPC subscribers that arrive before the cold pass commits
    /// ([LIVE-CACHE-SEED]).
    fn parser_ids(&self) -> Vec<String> {
        if let Some(pipeline) = self.pipeline.as_ref() {
            return pipeline
                .parsers()
                .iter()
                .map(|parser| parser.id().to_owned())
                .collect();
        }
        languages_from_report_occurrences(&self.latest_report)
    }

    /// Runs the underlying pipeline. Caller guarantees `pipeline` is
    /// `Some` — [`Self::apply_changes`] short-circuits otherwise.
    /// [LIVE-SCHEDULER-NOOP] [`None`] means the pass touched no analysed
    /// file, so the previous report still stands.
    fn run_pipeline(&mut self, changed: &[PathBuf]) -> Result<Option<Report>, LiveError> {
        let pipeline = self.pipeline.as_mut().ok_or(LiveError::AnalysisNotReady)?;
        let embedding = EmbeddingSettings {
            mode: self.embedding_mode,
            provider: Some(self.embedding_provider.as_ref()),
            batch_yield: live_batch_yield(self.embedding_mode),
            progress: None,
        };
        Ok(pipeline.update_files(changed, embedding, &self.buffers)?)
    }

    /// [LIVE-SEED-CACHE] Persists the current report snapshot to
    /// `{root}/.deslop/cache/live-report.json` so the next LSP startup
    /// has a fast warm-start. Called only on initial-pass and
    /// cold-pass install — never on per-keystroke incremental updates.
    /// Best-effort: failures are logged but never propagated.
    fn write_state_file(&self) {
        persist_state_file(
            &self.root,
            self.latest_report.as_ref(),
            self.generation,
            &self.cache_seed_key(),
        );
    }

    /// [LIVE-CACHE-SEED-KEY] The identity of this session's analysis
    /// settings, recorded beside the state file so the next startup can
    /// tell a re-usable seed from an incompatible one.
    fn cache_seed_key(&self) -> CacheSeedKey {
        CacheSeedKey::new(
            &self.root,
            self.min_nodes,
            self.incremental,
            self.config_path.as_deref(),
            self.embedding_mode,
            &self.embedding_provider.spec(),
        )
    }

    /// Asserts `path` is under the workspace root.
    pub(super) fn guard_path(&self, path: &Path) -> Result<(), LiveError> {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        // Canonicalize both sides so macOS `/var/...` ↔
        // `/private/var/...` resolves identically — without this the
        // MCP sends canonicalized paths and the LSP fails to match
        // a non-canonical workspace root prefix.
        // [LIVE-WORKSPACE-SCOPE] Unsaved and deleted leaves use the same
        // canonical surviving parent as the buffer map, including Windows prefixes.
        let canonical_resolved = crate::pipeline::canonical_source_path(&resolved);
        let canonical_root = crate::pipeline::canonical_source_path(&self.root);
        if canonical_resolved.starts_with(&canonical_root) {
            Ok(())
        } else {
            Err(LiveError::PathOutsideWorkspace {
                path: canonical_resolved,
                workspace_root: canonical_root,
            })
        }
    }
}

/// Clones the current report out of a lock-free snapshot handle without
/// touching the session mutex. The LSP backend uses this so its additive
/// editor surfaces never block behind an in-flight analysis pass
/// ([LSP-NON-INTERFERENCE]).
#[must_use]
pub fn read_report_snapshot(handle: &RwLock<Arc<Report>>) -> Arc<Report> {
    Arc::clone(
        &handle
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
}

/// Clones the buffers the current report analysed out of their lock-free
/// handle, for the same callers as [`read_report_snapshot`].
#[must_use]
pub fn read_analysed_buffers(handle: &RwLock<Arc<OpenBuffers>>) -> Arc<OpenBuffers> {
    Arc::clone(
        &handle
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
}

/// Derives language ids from the occurrence paths of a report, mapping each
/// path onto its canonical parser id via the registry-driven
/// [`language_for_path`]. Used as a fallback for `parser_ids` during the
/// cache-seed window before the background pipeline installs. Deriving from
/// the parser registry (not a hand-maintained extension map) keeps the seeded
/// language set from drifting when a language is added ([PIPELINE-LANG-TRAIT],
/// ).
fn languages_from_report_occurrences(report: &Report) -> Vec<String> {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for cluster in &report.clusters {
        for occurrence in &cluster.occurrences {
            let language = occurrence
                .language
                .as_deref()
                .unwrap_or_else(|| language_for_path(&occurrence.path));
            if language != "unknown" {
                let _inserted = seen.insert(language);
            }
        }
    }
    seen.into_iter().map(str::to_owned).collect()
}
