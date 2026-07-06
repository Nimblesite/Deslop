//! Live analysis session ([LIVE-STATE]).
//!
//! Owns one [`PipelineSession`], the latest rendered [`Report`]
//! snapshot, the monotonic generation counter, and the active embedding
//! provider. The session is the single live struct in the live module
//! — nothing else holds mutable analysis state.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    delta::ReportDelta,
    embedding::{EmbeddingMode, EmbeddingProvider},
    lang::LanguageParser,
    pipeline::{language_for_path, EmbeddingSettings, PipelineSession},
    report::{Report, ReportCluster},
};

use super::{
    cluster_lookup::resolve_cluster_by_id_prefix,
    embedding_refresh::{CommittedEmbeddingRefresh, EmbeddingRefreshInput, EmbeddingRefreshJob},
    errors::LiveError,
    freshness::FreshnessTracker,
    session_helpers::{
        append_ollama_models, cluster_matches_any_hash, cluster_overlaps_range,
        cluster_touches_path, collapse_overlapping_clusters_for_range, earliest_byte_for_path,
        initialise_pipeline, live_batch_yield, parse_and_hash_snippet, persist_state_file,
        truncate, try_load_cached_report,
    },
    wire::{
        EmbeddingModelInfo, EmbeddingProgress, FileReport, FindSimilarInput, FindSimilarRequest,
        FindSimilarResult, SessionConfig,
    },
};

/// Sink invoked around a `set_embedding_model` swap so transports can
/// forward the progress onto `deslop/embeddingProgress`. The reporter
/// is `Send + Sync` so it survives being moved into a tokio handler.
pub type EmbeddingProgressReporter = Arc<dyn Fn(EmbeddingProgress) + Send + Sync>;

/// Bundle of arguments required to assemble an [`AnalysisSession`].
/// Exists so the constructor signature stays under the line budget.
struct SessionInit {
    /// Workspace root pinned at construction.
    root: PathBuf,
    /// Subtree-size floor pinned at construction.
    min_nodes: u32,
    /// Pipeline session, or `None` for the cache-seed fast path.
    pipeline: Option<PipelineSession>,
    /// Initial report — fresh or loaded from the disk cache.
    report: Report,
    /// Active embedding provider.
    embedding_provider: Arc<dyn EmbeddingProvider>,
    /// Active embedding mode.
    mode: EmbeddingMode,
    /// Whether the on-disk fingerprint cache is consulted.
    incremental: bool,
    /// Optional explicit exclusion config path.
    config_path: Option<PathBuf>,
}

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
    /// Mtime ledger consulted before every IPC read
    /// ([LIVE-CLUSTER-OFFSET-FRESHNESS], [Deslop#153], [Deslop#156]). Refreshed
    /// after every analysis pass so a stale-mtime read forces a
    /// synchronous `apply_changes` ahead of serving the response.
    freshness: FreshnessTracker,
}

impl std::fmt::Debug for AnalysisSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisSession").finish_non_exhaustive()
    }
}

impl AnalysisSession {
    /// [LIVE-EMBEDDING-CONSENT] Constructs a new live session by
    /// running the first full analysis without an embedding pass.
    ///
    /// # Errors
    ///
    /// Propagates [`LiveError::Core`] from pipeline initialisation.
    pub fn new(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Result<Self, LiveError> {
        Self::new_with_mode(
            root,
            min_nodes,
            incremental,
            config_path,
            embedding_provider,
            EmbeddingMode::Off,
        )
    }

    /// [LIVE-EMBEDDING-CONSENT] Constructs a live session with an
    /// already-selected embedding model.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError`] when pipeline initialisation cannot read
    /// config, discover files, parse sources, or build the report.
    pub fn new_with_mode(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        mode: EmbeddingMode,
    ) -> Result<Self, LiveError> {
        let (pipeline, report) = initialise_pipeline(
            root.clone(),
            min_nodes,
            incremental,
            config_path.clone(),
            mode,
            embedding_provider.as_ref(),
        )?;
        let session = Self::finalise(SessionInit {
            root,
            min_nodes,
            pipeline: Some(pipeline),
            report,
            embedding_provider,
            mode,
            incremental,
            config_path,
        });
        session.write_state_file();
        Ok(session)
    }

    /// [LIVE-CACHE-SEED] Constructs a session from
    /// `{root}/.deslop-cache/live-report.json` so query methods can
    /// answer instantly while the background full pass runs. Returns
    /// `None` if the cache file is missing or corrupt. Callers must
    /// follow up with [`Self::install_pipeline`] when the
    /// asynchronous pass produces a real `(PipelineSession, Report)`.
    pub fn try_seeded_from_cache(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        mode: EmbeddingMode,
    ) -> Option<Self> {
        let report = try_load_cached_report(&root)?;
        let cluster_count = report.clusters.len();
        let session = Self::finalise(SessionInit {
            root,
            min_nodes,
            pipeline: None,
            report,
            embedding_provider,
            mode,
            incremental,
            config_path,
        });
        tracing::info!(cached_clusters = cluster_count, "lsp_seeded_from_cache");
        Some(session)
    }

    /// [LIVE-CACHE-SEED] Replaces the cache-seeded report with the
    /// freshly-computed pipeline output and replays any
    /// [`Self::apply_changes`] calls that arrived during the
    /// cache-seed window. Returns the previous report so callers can
    /// fold it into delta history.
    ///
    /// # Errors
    ///
    /// Propagates pipeline errors when replaying queued changes.
    pub fn install_pipeline(
        &mut self,
        pipeline: PipelineSession,
        report: Report,
    ) -> Result<Arc<Report>, LiveError> {
        let previous_report = Arc::clone(&self.latest_report);
        self.pipeline = Some(pipeline);
        self.generation = self.generation.saturating_add(1);
        self.publish_report(Arc::new(report));
        // [LIVE-CLUSTER-OFFSET-FRESHNESS] / [Deslop#153] Record the on-disk mtime
        // of every file in the freshly-installed report so the next
        // read does not falsely refire a refresh.
        self.freshness
            .record_from_report(&self.root, &self.latest_report);
        let pending = std::mem::take(&mut self.pending_changes);
        if !pending.is_empty() {
            let _delta = self.apply_changes(&pending)?;
        }
        // The seed cache is written by the cold-pass install path
        // ([LIVE-SEED-CACHE]) — `cache_seed::commit_refresh` calls
        // [`Self::persist_seed_cache`] explicitly. `refresh_full` and
        // other reuse paths share `install_pipeline` but must not
        // touch disk.
        Ok(previous_report)
    }

    /// Public, explicit seed-cache write. Called after the LSP cold
    /// pass installs the freshly-built pipeline so the next LSP
    /// startup has a warm cache. Must NOT be called on per-pass
    /// updates ([LIVE-SEED-CACHE]).
    pub fn persist_seed_cache(&self) {
        self.write_state_file();
    }

    /// Forces a fresh full-workspace analysis pass for editor command
    /// dispatch ([LSP-COMMANDS]).
    ///
    /// # Errors
    ///
    /// Returns [`LiveError`] when pipeline initialisation fails.
    pub fn refresh_full(&mut self) -> Result<ReportDelta, LiveError> {
        let previous_generation = self.generation;
        let (pipeline, report) = initialise_pipeline(
            self.root.clone(),
            self.min_nodes,
            self.incremental,
            self.config_path.clone(),
            self.embedding_mode,
            self.embedding_provider.as_ref(),
        )?;
        let previous = self.install_pipeline(pipeline, report)?;
        Ok(ReportDelta::between(
            Some((previous_generation, &previous)),
            self.generation,
            &self.latest_report,
        ))
    }

    /// Disables live embedding analysis and refreshes the report
    /// without semantic clusters.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError`] when the no-embedding refresh fails.
    pub fn disable_embeddings(&mut self) -> Result<ReportDelta, LiveError> {
        self.embedding_mode = EmbeddingMode::Off;
        self.embedding_refresh_revision = self.embedding_refresh_revision.saturating_add(1);
        self.refresh_full()
    }

    /// Flips live incremental-cache use and returns the updated config.
    pub fn toggle_incremental(&mut self) -> SessionConfig {
        self.incremental = !self.incremental;
        if let Some(pipeline) = &mut self.pipeline {
            pipeline.set_incremental(self.incremental);
        }
        self.session_config()
    }

    /// Assembles the session struct.
    fn finalise(init: SessionInit) -> Self {
        let mut freshness = FreshnessTracker::new();
        freshness.record_from_report(&init.root, &init.report);
        let latest_report = Arc::new(init.report);
        Self {
            root: init.root,
            min_nodes: init.min_nodes,
            pipeline: init.pipeline,
            pending_changes: Vec::new(),
            report_snapshot: Arc::new(RwLock::new(Arc::clone(&latest_report))),
            latest_report,
            generation: 1,
            embedding_provider: init.embedding_provider,
            embedding_mode: init.mode,
            incremental: init.incremental,
            config_path: init.config_path,
            embedding_progress_reporter: None,
            embedding_refresh_revision: 0,
            freshness,
        }
    }

    /// Installs (or clears with `None`) the embedding-progress reporter.
    pub fn set_embedding_progress_reporter(&mut self, reporter: Option<EmbeddingProgressReporter>) {
        self.embedding_progress_reporter = reporter;
    }

    /// Returns an `Arc` to the current report snapshot.
    #[must_use]
    pub fn report(&self) -> Arc<Report> {
        Arc::clone(&self.latest_report)
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
    }

    /// Returns a clone of the lock-free report-snapshot handle. The LSP
    /// backend reads its additive editor surfaces from this without ever
    /// awaiting the session mutex ([LSP-NON-INTERFERENCE]).
    #[must_use]
    pub fn report_snapshot_handle(&self) -> Arc<RwLock<Arc<Report>>> {
        Arc::clone(&self.report_snapshot)
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
    /// re-included ones ([LIVE-CONFIG-LIVE], #139, #189).
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
        let next = self.run_pipeline(changed)?;
        self.generation = self.generation.saturating_add(1);
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

    /// [LIVE-CLUSTER-OFFSET-FRESHNESS] / [Deslop#153] / [Deslop#156].
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

    /// Resolves a `find_similar` request.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnsupportedLanguage`] for unknown languages
    /// and [`LiveError::UnparseableInput`] / [`LiveError::Core`] for
    /// parse failures.
    pub fn find_similar(
        &self,
        request: &FindSimilarRequest,
    ) -> Result<FindSimilarResult, LiveError> {
        match &request.input {
            FindSimilarInput::OpenRange {
                path,
                start_byte,
                end_byte,
            } => self.find_similar_for_range(path, *start_byte, *end_byte, request.max_results),
            FindSimilarInput::Snippet { snippet, language } => self.find_similar_for_snippet(
                snippet.as_str(),
                language.as_str(),
                request.max_results,
            ),
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
            cache_root: self
                .root
                .join(crate::embedding::cache::DEFAULT_CACHE_DIR_NAME),
            incremental: self.incremental,
        }
    }

    /// Returns clusters whose occurrences overlap `path`. Thin wrapper
    /// over [`report_for_file_in`] so the live read and the LSP backend's
    /// lock-free snapshot read share one implementation.
    #[must_use]
    pub fn report_for_file(&self, path: &Path) -> FileReport {
        report_for_file_in(&self.latest_report, path)
    }

    /// Returns clusters overlapping a byte range in `path`. Thin wrapper
    /// over [`report_for_range_in`].
    #[must_use]
    pub fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Vec<ReportCluster> {
        report_for_range_in(&self.latest_report, path, start_byte, end_byte)
    }

    /// Looks up a cluster by its stable id ([Deslop#149]).
    ///
    /// Accepts either the full 16-hex canonical id or any prefix of at
    /// least 7 hex characters — the same slug the VSIX shows in hover
    /// bubbles and tree rows via `clusterSlug()`. Falls back to the
    /// shared [`resolve_cluster_by_id_prefix`] helper so the truncated-id
    /// behaviour stays consistent across the live API and the MCP wire.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnknownCluster`] when no cluster matches, the
    /// prefix is shorter than the slug floor, or the prefix is ambiguous.
    pub fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, LiveError> {
        resolve_cluster_by_id_prefix(&self.latest_report.clusters, id).cloned()
    }

    /// Queues a selected-model embedding refresh and returns the
    /// detached job description.
    pub(super) fn prepare_embedding_refresh(
        &mut self,
        provider: Arc<dyn EmbeddingProvider>,
    ) -> EmbeddingRefreshJob {
        self.embedding_provider = provider;
        self.embedding_mode = EmbeddingMode::Auto;
        self.embedding_refresh_revision = self.embedding_refresh_revision.saturating_add(1);
        let total = self
            .pipeline
            .as_ref()
            .map_or(0_u64, |pipeline| pipeline.fingerprint_count() as u64);
        let job = EmbeddingRefreshJob::new(EmbeddingRefreshInput {
            revision: self.embedding_refresh_revision,
            root: self.root.clone(),
            min_nodes: self.min_nodes,
            incremental: self.incremental,
            config_path: self.config_path.clone(),
            provider: Arc::clone(&self.embedding_provider),
            total,
            reporter: self.embedding_progress_reporter.clone(),
        });
        job.report_queued();
        job
    }

    /// Commits a completed embedding refresh when it is still the
    /// latest selected-model request.
    pub(super) fn commit_embedding_refresh(
        &mut self,
        job: &EmbeddingRefreshJob,
        report: Report,
    ) -> Option<CommittedEmbeddingRefresh> {
        if job.revision != self.embedding_refresh_revision {
            return None;
        }
        let previous_generation = self.generation;
        let previous_report = Arc::clone(&self.latest_report);
        self.generation = self.generation.saturating_add(1);
        let provenance = report.embedding_provenance.clone();
        self.publish_report(Arc::new(report));
        // [LIVE-CLUSTER-OFFSET-FRESHNESS] keep the mtime ledger aligned with the
        // freshly-committed report so reads do not loop-refire.
        self.freshness
            .record_from_report(&self.root, &self.latest_report);
        // Embedding refresh writes are still per-pass and noisy; seed
        // cache only updates on cold-pass install.
        Some(CommittedEmbeddingRefresh {
            previous_generation,
            previous_report,
            provenance,
        })
    }

    /// Lists embedding models available to the session.
    ///
    /// Returns models from every registered production provider. When
    /// no production provider is reachable (e.g. Ollama not running),
    /// returns an empty list — the VSIX surfaces the "Ollama not
    /// detected" empty state without a stub fallback.
    #[must_use]
    pub fn list_embedding_models(endpoint: &str) -> Vec<EmbeddingModelInfo> {
        let mut out: Vec<EmbeddingModelInfo> = Vec::new();
        match crate::embedding::list_ollama_models(endpoint) {
            Ok(models) => append_ollama_models(&mut out, models),
            Err(error) => {
                tracing::info!(
                    %error,
                    "ollama_unreachable_no_models_returned",
                );
            }
        }
        out
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
    fn run_pipeline(&mut self, changed: &[PathBuf]) -> Result<Report, LiveError> {
        let pipeline = self.pipeline.as_mut().ok_or(LiveError::AnalysisNotReady)?;
        let embedding = EmbeddingSettings {
            mode: self.embedding_mode,
            provider: Some(self.embedding_provider.as_ref()),
            batch_yield: live_batch_yield(self.embedding_mode),
            progress: None,
        };
        Ok(pipeline.update_files(changed, embedding)?)
    }

    /// Resolves the open-buffer-range variant of `find_similar`.
    fn find_similar_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
        max_results: Option<usize>,
    ) -> Result<FindSimilarResult, LiveError> {
        self.guard_path(path)?;
        let mut clusters = self.report_for_range(path, start_byte, end_byte);
        truncate(&mut clusters, max_results);
        let total_occurrences: usize = clusters.iter().map(crate::report::occurrence_count).sum();
        Ok(FindSimilarResult {
            clusters,
            below_min_nodes: false,
            total_occurrences,
        })
    }

    /// Resolves the snippet variant of `find_similar`.
    fn find_similar_for_snippet(
        &self,
        snippet: &str,
        language: &str,
        max_results: Option<usize>,
    ) -> Result<FindSimilarResult, LiveError> {
        let parser = self.parser_for_language(language)?;
        let snippet_hashes = parse_and_hash_snippet(parser, snippet, self.min_nodes)?;
        if snippet_hashes.is_empty() {
            return Ok(FindSimilarResult {
                clusters: Vec::new(),
                below_min_nodes: true,
                total_occurrences: 0,
            });
        }
        let mut clusters = self.clusters_matching_hashes(&snippet_hashes);
        truncate(&mut clusters, max_results);
        let total_occurrences: usize = clusters.iter().map(crate::report::occurrence_count).sum();
        Ok(FindSimilarResult {
            clusters,
            below_min_nodes: false,
            total_occurrences,
        })
    }

    /// Returns clusters whose stable id matches one of `snippet_hashes`.
    fn clusters_matching_hashes(&self, snippet_hashes: &[[u8; 32]]) -> Vec<ReportCluster> {
        self.latest_report
            .clusters
            .iter()
            .filter(|cluster| cluster_matches_any_hash(cluster, snippet_hashes))
            .cloned()
            .collect()
    }

    /// Returns the registered parser for `language`. Returns
    /// [`LiveError::AnalysisNotReady`] in the cache-seed window.
    fn parser_for_language(&self, language: &str) -> Result<&dyn LanguageParser, LiveError> {
        let pipeline = self.pipeline.as_ref().ok_or(LiveError::AnalysisNotReady)?;
        let parsers = pipeline.parsers();
        if let Some(found) = parsers.iter().find(|parser| parser.id() == language) {
            return Ok(found.as_ref());
        }
        Err(LiveError::UnsupportedLanguage {
            requested: language.to_owned(),
            registered: parsers
                .iter()
                .map(|parser| parser.id().to_owned())
                .collect(),
        })
    }

    /// [LIVE-SEED-CACHE] Persists the current report snapshot to
    /// `{root}/.deslop-cache/live-report.json` so the next LSP startup
    /// has a fast warm-start. Called only on initial-pass and
    /// cold-pass install — never on per-keystroke incremental updates.
    /// Best-effort: failures are logged but never propagated.
    fn write_state_file(&self) {
        persist_state_file(&self.root, self.latest_report.as_ref(), self.generation);
    }

    /// Asserts `path` is under the workspace root.
    fn guard_path(&self, path: &Path) -> Result<(), LiveError> {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        // Canonicalize both sides so macOS `/var/...` ↔
        // `/private/var/...` resolves identically — without this the
        // MCP sends canonicalized paths and the LSP fails to match
        // a non-canonical workspace root prefix.
        let canonical_resolved = std::fs::canonicalize(&resolved).unwrap_or(resolved);
        let canonical_root =
            std::fs::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());
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

/// Builds a [`FileReport`] for `path` from an arbitrary report snapshot.
/// Shared by [`AnalysisSession::report_for_file`] and the LSP backend's
/// lock-free editor reads so both surfaces filter and sort identically
/// ([LSP-NON-INTERFERENCE]).
#[must_use]
pub fn report_for_file_in(report: &Report, path: &Path) -> FileReport {
    let mut clusters: Vec<ReportCluster> = report
        .clusters
        .iter()
        .filter(|cluster| cluster_touches_path(cluster, path))
        .cloned()
        .collect();
    clusters.sort_by_key(|cluster| earliest_byte_for_path(cluster, path));
    let total_occurrences: usize = clusters.iter().map(crate::report::occurrence_count).sum();
    FileReport {
        path: path.to_path_buf(),
        clusters,
        total_occurrences,
    }
}

/// Returns clusters overlapping a byte range in `path` from an arbitrary
/// report snapshot. Shared by [`AnalysisSession::report_for_range`] and
/// the LSP backend's lock-free editor reads.
#[must_use]
pub fn report_for_range_in(
    report: &Report,
    path: &Path,
    start_byte: usize,
    end_byte: usize,
) -> Vec<ReportCluster> {
    let clusters = report
        .clusters
        .iter()
        .filter(|cluster| cluster_overlaps_range(cluster, path, start_byte, end_byte))
        .cloned()
        .collect();
    collapse_overlapping_clusters_for_range(clusters, path, start_byte, end_byte)
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

/// Derives language ids from the occurrence paths of a report, mapping each
/// path onto its canonical parser id via the registry-driven
/// [`language_for_path`]. Used as a fallback for `parser_ids` during the
/// cache-seed window before the background pipeline installs. Deriving from
/// the parser registry (not a hand-maintained extension map) keeps the seeded
/// language set from drifting when a language is added ([PIPELINE-LANG-TRAIT],
/// GH #270).
fn languages_from_report_occurrences(report: &Report) -> Vec<String> {
    let mut seen: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
    for cluster in &report.clusters {
        for occurrence in &cluster.occurrences {
            let language = language_for_path(&occurrence.path);
            if language != "unknown" {
                let _inserted = seen.insert(language);
            }
        }
    }
    seen.into_iter().map(str::to_owned).collect()
}
