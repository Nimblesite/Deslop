//! Live analysis session ([LIVE-STATE]).
//!
//! Owns one [`PipelineSession`], the latest rendered [`Report`]
//! snapshot, the monotonic generation counter, and the active embedding
//! provider. The session is the single live struct in the live module
//! — nothing else holds mutable analysis state.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    delta::ReportDelta,
    embedding::{EmbeddingMode, EmbeddingProvider},
    lang::LanguageParser,
    pipeline::{EmbeddingSettings, PipelineSession},
    report::{Report, ReportCluster},
};

use super::{
    embedding_refresh::{CommittedEmbeddingRefresh, EmbeddingRefreshInput, EmbeddingRefreshJob},
    errors::LiveError,
    session_helpers::{
        append_ollama_models, cluster_matches_any_hash, cluster_overlaps_range,
        cluster_touches_path, collapse_overlapping_clusters_for_range, earliest_byte_for_path,
        initialise_pipeline, live_batch_yield, parse_and_hash_snippet, stub_model_info, truncate,
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

/// Live analysis session. Wraps [`PipelineSession`] with the live-only
/// metadata documented in [LIVE-STATE].
pub struct AnalysisSession {
    /// Underlying analysis state.
    pipeline: PipelineSession,
    /// Atomic snapshot of the current report.
    latest_report: Arc<Report>,
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
}

impl std::fmt::Debug for AnalysisSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisSession").finish_non_exhaustive()
    }
}

impl AnalysisSession {
    /// [LIVE-EMBEDDING-CONSENT] Constructs a new live session by running the first full
    /// analysis against `root` without the embedding pass. Live
    /// surfaces require an explicit model selection before local
    /// embedding work starts.
    ///
    /// # Errors
    ///
    /// Propagates any [`LiveError::Core`] surfaced by the underlying
    /// pipeline initialisation.
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
    /// already-selected embedding model. Used when a client persisted
    /// explicit user consent.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError`] when pipeline initialisation cannot read
    /// config, discover files, parse sources, or build the initial
    /// report.
    pub fn new_with_mode(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        mode: EmbeddingMode,
    ) -> Result<Self, LiveError> {
        let (pipeline, report) = initialise_pipeline(
            root,
            min_nodes,
            incremental,
            config_path.clone(),
            mode,
            embedding_provider.as_ref(),
        )?;
        Ok(Self::finalise(
            pipeline,
            report,
            embedding_provider,
            mode,
            incremental,
            config_path,
        ))
    }

    /// Assembles the session struct from the initialised pipeline.
    /// Extracted so [`Self::new`] stays within the 20-line budget.
    fn finalise(
        pipeline: PipelineSession,
        report: Report,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        embedding_mode: EmbeddingMode,
        incremental: bool,
        config_path: Option<PathBuf>,
    ) -> Self {
        Self {
            pipeline,
            latest_report: Arc::new(report),
            generation: 1,
            embedding_provider,
            embedding_mode,
            incremental,
            config_path,
            embedding_progress_reporter: None,
            embedding_refresh_revision: 0,
        }
    }

    /// Installs (or clears with `None`) the embedding-progress reporter
    /// invoked around [`Self::set_embedding_model`]. The LSP backend
    /// installs a reporter that forwards events onto the LSP client as
    /// `deslop/embeddingProgress` notifications.
    pub fn set_embedding_progress_reporter(&mut self, reporter: Option<EmbeddingProgressReporter>) {
        self.embedding_progress_reporter = reporter;
    }

    /// Returns an `Arc` to the current report snapshot.
    #[must_use]
    pub fn report(&self) -> Arc<Report> {
        Arc::clone(&self.latest_report)
    }

    /// Returns the current generation counter.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the workspace root pinned at construction.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.pipeline.root()
    }

    /// Re-analyses `changed`, swaps the report, bumps the generation,
    /// and returns a [`ReportDelta`] versus the previous snapshot.
    ///
    /// # Errors
    ///
    /// Propagates pipeline errors via [`LiveError::Core`].
    pub fn apply_changes(&mut self, changed: &[PathBuf]) -> Result<ReportDelta, LiveError> {
        let previous = Arc::clone(&self.latest_report);
        let prev_generation = self.generation;
        let next = self.run_pipeline(changed)?;
        self.generation = self.generation.saturating_add(1);
        let next_arc = Arc::new(next);
        self.latest_report = Arc::clone(&next_arc);
        Ok(ReportDelta::between(
            Some((prev_generation, &previous)),
            self.generation,
            &next_arc,
        ))
    }

    /// Resolves a `find_similar` request against the live corpus.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnsupportedLanguage`] for snippet inputs
    /// whose language has no registered parser, and
    /// [`LiveError::UnparseableInput`] / [`LiveError::Core`] when
    /// parsing fails.
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
            workspace_root: self.pipeline.root().to_path_buf(),
            min_nodes: self.pipeline.min_nodes(),
            languages: self.parser_ids(),
            embedding_provenance: self.latest_report.embedding_provenance.clone(),
            exclusion_config_path: self.config_path.clone(),
            cache_root: self
                .pipeline
                .root()
                .join(crate::embedding::cache::DEFAULT_CACHE_DIR_NAME),
            incremental: self.incremental,
        }
    }

    /// Returns clusters whose occurrences overlap `path`.
    #[must_use]
    pub fn report_for_file(&self, path: &Path) -> FileReport {
        let mut clusters: Vec<ReportCluster> = self
            .latest_report
            .clusters
            .iter()
            .filter(|cluster| cluster_touches_path(cluster, path))
            .cloned()
            .collect();
        clusters.sort_by_key(|cluster| earliest_byte_for_path(cluster, path));
        FileReport {
            path: path.to_path_buf(),
            clusters,
        }
    }

    /// Returns clusters overlapping a byte range in `path`.
    #[must_use]
    pub fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Vec<ReportCluster> {
        let clusters = self
            .latest_report
            .clusters
            .iter()
            .filter(|cluster| cluster_overlaps_range(cluster, path, start_byte, end_byte))
            .cloned()
            .collect();
        collapse_overlapping_clusters_for_range(clusters, path, start_byte, end_byte)
    }

    /// Looks up a cluster by its stable id.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnknownCluster`] when no cluster matches.
    pub fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, LiveError> {
        self.latest_report
            .clusters
            .iter()
            .find(|cluster| cluster.id == id)
            .cloned()
            .ok_or_else(|| LiveError::UnknownCluster { id: id.to_owned() })
    }

    /// Queues a selected-model embedding refresh and returns the
    /// detached job description. The expensive pipeline pass runs
    /// outside the session lock.
    pub(super) fn prepare_embedding_refresh(
        &mut self,
        provider: Arc<dyn EmbeddingProvider>,
    ) -> EmbeddingRefreshJob {
        self.embedding_provider = provider;
        self.embedding_mode = EmbeddingMode::Auto;
        self.embedding_refresh_revision = self.embedding_refresh_revision.saturating_add(1);
        let total = self.pipeline.fingerprint_count() as u64;
        let job = EmbeddingRefreshJob::new(EmbeddingRefreshInput {
            revision: self.embedding_refresh_revision,
            root: self.pipeline.root().to_path_buf(),
            min_nodes: self.pipeline.min_nodes(),
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
        self.latest_report = Arc::new(report);
        Some(CommittedEmbeddingRefresh {
            previous_generation,
            previous_report,
            provenance,
        })
    }

    /// Lists embedding models available to the session — built-in
    /// stub plus any Ollama models reachable at `endpoint`. Falls back
    /// to stub-only when Ollama is unreachable ([LIVE-QUERY-API]).
    #[must_use]
    pub fn list_embedding_models(endpoint: &str) -> Vec<EmbeddingModelInfo> {
        let mut out = vec![stub_model_info()];
        match crate::embedding::list_ollama_models(endpoint) {
            Ok(models) => append_ollama_models(&mut out, models),
            Err(error) => {
                tracing::info!(%error, "ollama unreachable; returning stub-only model list");
            }
        }
        out
    }

    /// Stable list of registered parser ids.
    fn parser_ids(&self) -> Vec<String> {
        self.pipeline
            .parsers()
            .iter()
            .map(|parser| parser.id().to_owned())
            .collect()
    }

    /// Runs the underlying pipeline with the active provider/mode
    /// settings.
    fn run_pipeline(&mut self, changed: &[PathBuf]) -> Result<Report, LiveError> {
        let embedding = EmbeddingSettings {
            mode: self.embedding_mode,
            provider: Some(self.embedding_provider.as_ref()),
            batch_yield: live_batch_yield(self.embedding_mode),
            progress: None,
        };
        Ok(self.pipeline.update_files(changed, embedding)?)
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
        Ok(FindSimilarResult {
            clusters,
            below_min_nodes: false,
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
        let snippet_hashes = parse_and_hash_snippet(parser, snippet, self.pipeline.min_nodes())?;
        if snippet_hashes.is_empty() {
            return Ok(FindSimilarResult {
                clusters: Vec::new(),
                below_min_nodes: true,
            });
        }
        let mut clusters = self.clusters_matching_hashes(&snippet_hashes);
        truncate(&mut clusters, max_results);
        Ok(FindSimilarResult {
            clusters,
            below_min_nodes: false,
        })
    }

    /// Returns the cluster snapshots whose stable id matches one of
    /// the supplied snippet hashes.
    fn clusters_matching_hashes(&self, snippet_hashes: &[[u8; 32]]) -> Vec<ReportCluster> {
        self.latest_report
            .clusters
            .iter()
            .filter(|cluster| cluster_matches_any_hash(cluster, snippet_hashes))
            .cloned()
            .collect()
    }

    /// Returns the registered parser for `language` or an error.
    fn parser_for_language(&self, language: &str) -> Result<&dyn LanguageParser, LiveError> {
        let parsers = self.pipeline.parsers();
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

    /// Asserts `path` is under the workspace root. Canonicalises the
    /// candidate so symlink pairs like macOS `/var/...` →
    /// `/private/var/...` are treated as one ([#141 MCP-SAFETY]).
    fn guard_path(&self, path: &Path) -> Result<(), LiveError> {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.pipeline.root().join(path)
        };
        let canonical = std::fs::canonicalize(&resolved).unwrap_or(resolved);
        if canonical.starts_with(self.pipeline.root()) {
            Ok(())
        } else {
            Err(LiveError::PathOutsideWorkspace {
                path: canonical,
                workspace_root: self.pipeline.root().to_path_buf(),
            })
        }
    }
}
