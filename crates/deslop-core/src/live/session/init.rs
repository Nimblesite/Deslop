//! [LIVE-STATE] Construction, cache-seed install and full refresh of [`AnalysisSession`].

use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use super::{
    super::{
        cache_seed_key::CacheSeedKey,
        errors::LiveError,
        freshness::FreshnessTracker,
        session_helpers::{initialise_pipeline, try_load_cached_report},
        watcher::live_exclusion,
        wire::SessionConfig,
    },
    AnalysisSession,
};
use crate::{
    config::ExclusionConfig,
    delta::ReportDelta,
    embedding::{EmbeddingMode, EmbeddingProvider},
    pipeline::{OpenBuffers, PipelineSession},
    report::Report,
};

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
    /// `{root}/.deslop/cache/live-report.json` so query methods can
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
        // [LIVE-CACHE-SEED-KEY] A seed is served as an answer, so it
        // must have been computed by a run with the same identity.
        let key = CacheSeedKey::new(
            &root,
            min_nodes,
            incremental,
            config_path.as_deref(),
            mode,
            &embedding_provider.spec(),
        );
        let report = try_load_cached_report(&root, &key)?;
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
        self.republish_exclusion();
        self.generation = self.generation.saturating_add(1);
        self.publish_report(Arc::new(report));
        // [LIVE-CLUSTER-OFFSET-FRESHNESS] / Record the on-disk mtime
        // of every file in the freshly-installed report so the next
        // read does not falsely refire a refresh.
        self.freshness
            .record_from_report(&self.root, &self.latest_report);
        // [LIVE-OPEN-BUFFERS] The full pass read every file from disk,
        // so re-apply the open buffers alongside the queued changes:
        // the editor's text must win again before this report is
        // served as the answer.
        let mut replay = std::mem::take(&mut self.pending_changes);
        replay.extend(self.buffers.paths().map(Path::to_path_buf));
        if !replay.is_empty() {
            let _delta = self.apply_changes(&replay)?;
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
        let exclusion = live_exclusion(init.pipeline.as_ref().map_or_else(
            || Arc::new(ExclusionConfig::empty().with_scan_root(&init.root)),
            PipelineSession::exclusion_handle,
        ));
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
            exclusion,
            freshness,
            buffers: OpenBuffers::default(),
        }
    }
}
