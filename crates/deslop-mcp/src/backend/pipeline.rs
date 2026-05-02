//! `PipelineSessionBackend` — the concrete [`McpBackend`] implementation.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use deslop_core::{
    list_ollama_models, report::ReportCluster, state::FileRegistry, EmbeddingMode,
    EmbeddingProvider, EmbeddingSettings, EmbeddingSpec, OllamaModelInfo, OllamaProvider,
    PipelineSession, ProviderError, Report, StubProvider, DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
};
use tracing::info;

use crate::safety::resolve_within_root;

use super::{
    filters, live_batch_yield, persistence, refresh, BackendError, FindSimilarInput,
    FindSimilarOutput, McpBackend, SessionBackendConfig, SessionConfigSnapshot,
};

/// Mutable state behind the backend mutex.
#[derive(Debug)]
pub(super) struct SessionState {
    /// Owned pipeline session.
    pub(super) session: PipelineSession,
    /// Latest rendered report.
    pub(super) report: Arc<Report>,
    /// Monotonic generation counter.
    pub(super) generation: u64,
    /// Active embedding provider (if any).
    pub(super) provider: Option<Arc<dyn EmbeddingProvider>>,
    /// [MCP-EMBEDDING-CONSENT] Active embedding mode. Starts from CLI config; selecting a
    /// model turns live embeddings on for subsequent changes.
    pub(super) embedding_mode: EmbeddingMode,
    /// Monotonic id for detached embedding refreshes.
    pub(super) embedding_revision: u64,
}

/// `McpBackend` implementation backed by a [`PipelineSession`] guarded
/// by a shared [`Mutex`]. Foreground tool calls stay short; selected
/// embedding refreshes run on a detached low-priority worker.
#[derive(Debug)]
pub struct PipelineSessionBackend {
    /// Shared session configuration.
    config: SessionBackendConfig,
    /// Mutable state behind a single mutex so concurrent tool calls
    /// serialise cleanly.
    state: Arc<Mutex<SessionState>>,
}

impl PipelineSessionBackend {
    /// Initialises a new backend by running the first full analysis
    /// against `config.root`.
    ///
    /// # Errors
    ///
    /// Propagates [`BackendError::Core`] for pipeline failures and
    /// [`BackendError::Provider`] when `config.embedding_mode` is
    /// `Required` and the provider cannot be reached.
    pub fn initialise(config: SessionBackendConfig) -> Result<Self, BackendError> {
        let started = Instant::now();
        let provider = refresh::select_provider(&config)?;
        let (session, report) = PipelineSession::initialise(
            config.root.clone(),
            config.min_nodes,
            config.incremental,
            config.config_path.clone(),
            EmbeddingSettings {
                mode: config.embedding_mode,
                provider: provider.as_deref(),
                batch_yield: live_batch_yield(config.embedding_mode),
                progress: None,
            },
        )?;
        let report = Arc::new(report);
        info!(
            files_analysed = report.files_analysed,
            clusters = report.clusters.len(),
            min_nodes = config.min_nodes,
            elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            "mcp_session_initialised"
        );
        let embedding_mode = config.embedding_mode;
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(SessionState {
                session,
                report,
                generation: 1,
                provider,
                embedding_mode,
                embedding_revision: 0,
            })),
        })
    }

    /// Returns the current report wrapped in an [`Arc`].
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::MutexPoisoned`] when the mutex is
    /// poisoned.
    pub fn current_report(&self) -> Result<Arc<Report>, BackendError> {
        let state = lock_state(&self.state)?;
        Ok(state.report.clone())
    }

    /// `find-similar` (snippet variant) parses `snippet` with the
    /// registered parser for `language` and — if the resulting tree
    /// clears the `min_nodes` floor — returns the top-N clusters from
    /// the live report. Parsing is in-memory; nothing mutates the
    /// caches ([MCP-TOOL-FINDSIMILAR]).
    fn find_similar_snippet(
        &self,
        snippet: &str,
        language: &str,
        top_n: usize,
    ) -> Result<FindSimilarOutput, BackendError> {
        if snippet.is_empty() {
            return Ok(FindSimilarOutput {
                clusters: Vec::new(),
                below_min_nodes: false,
            });
        }
        let (parsed_nodes, report_clusters) = {
            let state = lock_state(&self.state)?;
            let parser = state
                .session
                .parsers()
                .iter()
                .find(|candidate| candidate.id() == language)
                .ok_or_else(|| BackendError::UnsupportedLanguage(language.to_owned()))?;
            let mut scratch = FileRegistry::new();
            let scratch_id = scratch.register(PathBuf::from("<mcp-snippet>"));
            let parsed = parser
                .parse_and_normalize(snippet.as_bytes(), scratch_id)
                .map_err(|err| BackendError::UnparseableInput(err.to_string()))?;
            (parsed.subtree_node_count(), state.report.clusters.clone())
        };
        let min_floor = self.config.min_nodes as usize;
        if parsed_nodes < min_floor {
            return Ok(FindSimilarOutput {
                clusters: Vec::new(),
                below_min_nodes: true,
            });
        }
        Ok(FindSimilarOutput {
            clusters: filters::trim_top_n(report_clusters, top_n),
            below_min_nodes: false,
        })
    }
}

impl McpBackend for PipelineSessionBackend {
    fn root(&self) -> &Path {
        &self.config.root
    }

    fn generation(&self) -> u64 {
        lock_state(&self.state).map_or(0, |state| state.generation)
    }

    fn report_get(&self) -> Result<Arc<Report>, BackendError> {
        self.current_report()
    }

    fn report_for_file(&self, path: &Path) -> Result<Vec<ReportCluster>, BackendError> {
        let resolved = resolve_within_root(&self.config.root, path)?;
        let report = self.current_report()?;
        Ok(filters::filter_clusters_by_path(
            &report,
            &resolved,
            &self.config.root,
        ))
    }

    fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Result<Vec<ReportCluster>, BackendError> {
        let resolved = resolve_within_root(&self.config.root, path)?;
        let report = self.current_report()?;
        Ok(filters::filter_clusters_by_range(
            &report,
            &resolved,
            start_byte,
            end_byte,
            &self.config.root,
        ))
    }

    fn find_similar(
        &self,
        input: FindSimilarInput<'_>,
        top_n: usize,
    ) -> Result<FindSimilarOutput, BackendError> {
        let effective_top_n = if top_n == 0 { 5 } else { top_n };
        match input {
            FindSimilarInput::Range {
                path,
                start_byte,
                end_byte,
            } => {
                let resolved = resolve_within_root(&self.config.root, path)?;
                let report = self.current_report()?;
                let clusters = filters::filter_clusters_by_range(
                    &report,
                    &resolved,
                    start_byte,
                    end_byte,
                    &self.config.root,
                );
                Ok(FindSimilarOutput {
                    clusters: filters::trim_top_n(clusters, effective_top_n),
                    below_min_nodes: false,
                })
            }
            FindSimilarInput::Snippet { snippet, language } => {
                self.find_similar_snippet(snippet, language, effective_top_n)
            }
        }
    }

    fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, BackendError> {
        let report = self.current_report()?;
        report
            .clusters
            .iter()
            .find(|candidate| candidate.id == id)
            .cloned()
            .ok_or_else(|| BackendError::UnknownCluster(id.to_owned()))
    }

    fn list_embedding_models(&self) -> Result<Vec<OllamaModelInfo>, BackendError> {
        let mut all: Vec<OllamaModelInfo> = vec![filters::stub_model_info()];
        match list_ollama_models(&self.config.embedding_endpoint) {
            Ok(models) => all.extend(models),
            Err(ProviderError::Unreachable { .. }) => {
                info!("ollama_unreachable_falling_back_to_stub_only");
            }
            Err(other) => return Err(other.into()),
        }
        Ok(all)
    }

    fn set_embedding_model(
        &self,
        provider_id: &str,
        model_id: &str,
        endpoint: Option<&str>,
    ) -> Result<EmbeddingSpec, BackendError> {
        let new_provider: Arc<dyn EmbeddingProvider> = match provider_id {
            STUB_PROVIDER_ID => Arc::new(StubProvider::new()),
            DEFAULT_PROVIDER_ID => Arc::new(OllamaProvider::connect(
                endpoint.unwrap_or(&self.config.embedding_endpoint),
                model_id,
            )?),
            other => return Err(BackendError::UnknownEmbeddingProvider(other.to_owned())),
        };
        let spec = new_provider.spec();
        persistence::persist_shared_embedding_settings(&self.config.root, &spec, endpoint)?;
        let revision = {
            let mut state = lock_state(&self.state)?;
            state.provider = Some(Arc::clone(&new_provider));
            state.embedding_mode = EmbeddingMode::Auto;
            state.embedding_revision = state.embedding_revision.saturating_add(1);
            state.embedding_revision
        };
        refresh::spawn_mcp_embedding_refresh(
            self.config.clone(),
            Arc::clone(&self.state),
            new_provider,
            revision,
        );
        info!(
            provider_id = spec.provider_id,
            model_id = spec.model_id,
            model_version = spec.model_version,
            dimensions = spec.dimensions,
            "mcp_embedding_model_queued"
        );
        Ok(spec)
    }

    fn session_config(&self) -> Result<SessionConfigSnapshot, BackendError> {
        let state = lock_state(&self.state)?;
        let languages = distinct_languages(&state.session);
        Ok(SessionConfigSnapshot {
            root: self.config.root.clone(),
            min_nodes: self.config.min_nodes,
            languages,
            embedding_provenance: state.report.embedding_provenance.clone(),
            incremental: self.config.incremental,
            cumulative_cache_stats: state.session.cumulative_cache_stats(),
        })
    }

    fn mark_changed(&self, paths: &[PathBuf]) -> Result<(), BackendError> {
        let mut state = lock_state(&self.state)?;
        let SessionState {
            session,
            provider,
            report,
            generation,
            embedding_mode,
            ..
        } = &mut *state;
        let settings = EmbeddingSettings {
            mode: *embedding_mode,
            provider: provider.as_deref(),
            batch_yield: live_batch_yield(*embedding_mode),
            progress: None,
        };
        let new_report = session.update_files(paths, settings)?;
        *report = Arc::new(new_report);
        *generation = generation.saturating_add(1);
        drop(state);
        Ok(())
    }
}

/// Locks the backend state, mapping poisoning onto a stable error.
pub(super) fn lock_state(
    mutex: &Mutex<SessionState>,
) -> Result<MutexGuard<'_, SessionState>, BackendError> {
    mutex
        .lock()
        .map_err(|_poisoned| BackendError::MutexPoisoned)
}

/// Returns the distinct set of registered language ids in stable
/// alphabetical order.
pub(super) fn distinct_languages(session: &PipelineSession) -> Vec<String> {
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for parser in session.parsers() {
        let _inserted = ids.insert(parser.id().to_owned());
    }
    ids.into_iter().collect()
}
