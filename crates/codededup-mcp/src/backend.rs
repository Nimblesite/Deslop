//! [`McpBackend`] — the trait the server dispatches tool calls against.
//!
//! The MCP server is a transport adapter; the backend holds the live
//! analysis state. This file ships one concrete implementation,
//! [`PipelineSessionBackend`], which owns a
//! [`codededup_core::PipelineSession`] and re-runs analysis on every
//! agent-driven `mark_changed` signal.
//!
//! When `codededup_core::live::LiveApi` lands (P7), a `LiveApiBackend`
//! impl slots in without changing the server code — that is the whole
//! point of this trait ([MCP-WHY-LIVE], [MCP-CAPABILITIES]).

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use codededup_core::{
    ast::NormalizedNode,
    embedding::{DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL},
    list_ollama_models,
    report::{CacheStats, ReportCluster, ReportOccurrence},
    state::FileRegistry,
    CoreError, EmbeddingMode, EmbeddingProvenance, EmbeddingProvider, EmbeddingSettings,
    EmbeddingSpec, OllamaModelInfo, OllamaProvider, PipelineSession, ProviderError, Report,
    StubProvider, DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
};

use thiserror::Error;
use tracing::{info, warn};

use crate::safety::{resolve_within_root, PathResolutionError};

/// Errors surfaced by the backend during tool execution.
#[derive(Debug, Error)]
pub enum BackendError {
    /// Underlying core pipeline error.
    #[error("core pipeline failure: {0}")]
    Core(#[from] CoreError),
    /// Path argument resolved outside the workspace root
    /// ([MCP-SAFETY]).
    #[error(transparent)]
    Path(#[from] PathResolutionError),
    /// Embedding provider unreachable / misbehaving.
    #[error("embedding provider failure: {0}")]
    Provider(#[from] ProviderError),
    /// The requested cluster id is not present in the current report.
    #[error("no cluster with id {0:?}")]
    UnknownCluster(String),
    /// `find-similar` received a snippet whose language is not
    /// registered with the session.
    #[error("language {0:?} is not registered with this session")]
    UnsupportedLanguage(String),
    /// `find-similar` received a snippet tree-sitter could not parse.
    #[error("failed to parse snippet: {0}")]
    UnparseableInput(String),
    /// A registered embedding provider id is unknown — only `ollama`
    /// and `stub` are supported on the fast path.
    #[error("embedding provider {0:?} is not registered")]
    UnknownEmbeddingProvider(String),
    /// Internal mutex was poisoned. Fatal — the session is toast.
    #[error("backend state mutex poisoned; analysis aborted")]
    MutexPoisoned,
}

/// Read-only view over the server-facing capabilities of the backend.
///
/// Each method corresponds to exactly one MCP tool (plus the resource
/// reads for `codededup://report` / `codededup://schema`). Every
/// method is pure except [`Self::set_embedding_model`], which the
/// [MCP-SAFETY] contract explicitly permits.
pub trait McpBackend: Send + Sync {
    /// Returns the workspace root pinned at session initialisation.
    fn root(&self) -> &Path;

    /// Returns the current generation counter. Bumped on every
    /// analysis refresh or `set_embedding_model` call so agents can
    /// reconcile notifications against responses.
    fn generation(&self) -> u64;

    /// Returns the current report snapshot.
    ///
    /// # Errors
    ///
    /// Propagates [`BackendError::Core`] on analysis failure or
    /// [`BackendError::MutexPoisoned`] on a fatal state lock error.
    fn report_get(&self) -> Result<Arc<Report>, BackendError>;

    /// Returns the clusters whose occurrences touch `path`.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Path`] when `path` resolves outside
    /// the workspace root.
    fn report_for_file(&self, path: &Path) -> Result<Vec<ReportCluster>, BackendError>;

    /// Returns the clusters overlapping `[start_byte, end_byte)` in
    /// `path`.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Path`] when `path` resolves outside
    /// the workspace root.
    fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Result<Vec<ReportCluster>, BackendError>;

    /// Returns the clusters most similar to a range on an open file
    /// (no snippet rebuild) or to a freestanding snippet.
    ///
    /// Implements [MCP-TOOL-FINDSIMILAR]. `top_n` caps the returned
    /// list; `0` means "use the default" (currently 5).
    ///
    /// # Errors
    ///
    /// Propagates [`BackendError::UnparseableInput`] for snippet
    /// parse failures, [`BackendError::UnsupportedLanguage`] for
    /// unknown language ids, and [`BackendError::Path`] for range
    /// inputs outside the workspace root.
    fn find_similar(
        &self,
        input: FindSimilarInput<'_>,
        top_n: usize,
    ) -> Result<FindSimilarOutput, BackendError>;

    /// Returns a cluster by its stable id.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::UnknownCluster`] when `id` is not in
    /// the current report.
    fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, BackendError>;

    /// Enumerates embedding models available on the host. The list
    /// always begins with the built-in `stub` provider; Ollama being
    /// unreachable is not an error ([MCP-CAPABILITIES]).
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Provider`] for provider errors other
    /// than `Unreachable`, which degrades gracefully.
    fn list_embedding_models(&self) -> Result<Vec<OllamaModelInfo>, BackendError>;

    /// Swaps the active embedding provider / model.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Provider`] when the new provider is
    /// unreachable or misbehaves.
    fn set_embedding_model(
        &self,
        provider_id: &str,
        model_id: &str,
        endpoint: Option<&str>,
    ) -> Result<EmbeddingSpec, BackendError>;

    /// Returns the current session configuration.
    ///
    /// # Errors
    ///
    /// Propagates [`BackendError::Core`] on report refresh failure.
    fn session_config(&self) -> Result<SessionConfigSnapshot, BackendError>;

    /// Signals to the backend that one or more watched files have
    /// changed. Implementations that track a `PipelineSession` will
    /// re-run analysis and bump [`Self::generation`].
    ///
    /// # Errors
    ///
    /// Propagates [`BackendError::Core`] on analysis failure.
    fn mark_changed(&self, paths: &[PathBuf]) -> Result<(), BackendError>;
}

/// Input variants accepted by [`McpBackend::find_similar`].
#[derive(Debug, Clone)]
pub enum FindSimilarInput<'a> {
    /// An existing byte range on a file already in the corpus.
    Range {
        /// Path to the file the agent is editing.
        path: &'a Path,
        /// Inclusive start byte of the range.
        start_byte: usize,
        /// Exclusive end byte of the range.
        end_byte: usize,
    },
    /// A snippet of source the agent is *about to* write.
    Snippet {
        /// Source text to parse.
        snippet: &'a str,
        /// Language id (one of `csharp`, `rust`, `python`).
        language: &'a str,
    },
}

/// Output shape for [`McpBackend::find_similar`]. Explicit
/// `below_min_nodes` flag per [MCP-TOOL-FINDSIMILAR].
#[derive(Debug, Clone)]
pub struct FindSimilarOutput {
    /// Matching clusters, worst-first.
    pub clusters: Vec<ReportCluster>,
    /// True when the snippet / range produced no fingerprint because
    /// it was smaller than `min_nodes`.
    pub below_min_nodes: bool,
}

/// Snapshot returned by [`McpBackend::session_config`].
#[derive(Debug, Clone)]
pub struct SessionConfigSnapshot {
    /// Workspace root pinned at init.
    pub root: PathBuf,
    /// Current `min_nodes` floor.
    pub min_nodes: u32,
    /// Language ids registered with the session (alphabetical).
    pub languages: Vec<String>,
    /// Active embedding provenance (if any).
    pub embedding_provenance: Option<EmbeddingProvenance>,
    /// Whether the incremental fingerprint cache is enabled.
    pub incremental: bool,
    /// Cache-hit totals since session start.
    pub cumulative_cache_stats: CacheStats,
}

/// Knobs for constructing a [`PipelineSessionBackend`].
#[derive(Debug, Clone)]
pub struct SessionBackendConfig {
    /// Workspace root to analyse.
    pub root: PathBuf,
    /// Minimum subtree node count (mirrors the CLI `--min-nodes` flag).
    pub min_nodes: u32,
    /// Whether to consult the on-disk fingerprint cache.
    pub incremental: bool,
    /// Embedding-pass mode.
    pub embedding_mode: EmbeddingMode,
    /// Embedding provider id (`stub`, `ollama`, …).
    pub embedding_provider: String,
    /// Embedding model id (meaningful for the `ollama` provider).
    pub embedding_model: String,
    /// Embedding endpoint override (currently only Ollama honours it).
    pub embedding_endpoint: String,
    /// Optional `.codededup.toml` override path.
    pub config_path: Option<PathBuf>,
}

impl Default for SessionBackendConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            min_nodes: 30,
            incremental: false,
            embedding_mode: EmbeddingMode::Off,
            embedding_provider: DEFAULT_PROVIDER_ID.to_owned(),
            embedding_model: DEFAULT_OLLAMA_MODEL.to_owned(),
            embedding_endpoint: DEFAULT_OLLAMA_ENDPOINT.to_owned(),
            config_path: None,
        }
    }
}

/// `McpBackend` implementation backed by a [`PipelineSession`] guarded
/// by a [`Mutex`]. Single-flight execution matches the MCP request
/// model and avoids interleaving analysis passes.
#[derive(Debug)]
pub struct PipelineSessionBackend {
    /// Shared session configuration.
    config: SessionBackendConfig,
    /// Mutable state behind a single mutex so concurrent tool calls
    /// serialise cleanly.
    state: Mutex<SessionState>,
}

/// Mutable state behind the backend mutex.
#[derive(Debug)]
struct SessionState {
    /// Owned pipeline session.
    session: PipelineSession,
    /// Latest rendered report.
    report: Arc<Report>,
    /// Monotonic generation counter.
    generation: u64,
    /// Active embedding provider (if any).
    provider: Option<Box<dyn EmbeddingProvider>>,
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
        let provider = select_provider(&config)?;
        let (session, report) = PipelineSession::initialise(
            config.root.clone(),
            config.min_nodes,
            config.incremental,
            config.config_path.clone(),
            EmbeddingSettings {
                mode: config.embedding_mode,
                provider: provider.as_deref(),
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
        Ok(Self {
            config,
            state: Mutex::new(SessionState {
                session,
                report,
                generation: 1,
                provider,
            }),
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
        Ok(filter_clusters_by_path(
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
        Ok(filter_clusters_by_range(
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
                let clusters = filter_clusters_by_range(
                    &report,
                    &resolved,
                    start_byte,
                    end_byte,
                    &self.config.root,
                );
                Ok(FindSimilarOutput {
                    clusters: trim_top_n(clusters, effective_top_n),
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
        let mut all: Vec<OllamaModelInfo> = vec![stub_model_info()];
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
        let new_provider: Box<dyn EmbeddingProvider> = match provider_id {
            STUB_PROVIDER_ID => Box::new(StubProvider::new()),
            DEFAULT_PROVIDER_ID => Box::new(OllamaProvider::connect(
                endpoint.unwrap_or(&self.config.embedding_endpoint),
                model_id,
            )?),
            other => return Err(BackendError::UnknownEmbeddingProvider(other.to_owned())),
        };
        let spec = new_provider.spec();
        let mut state = lock_state(&self.state)?;
        state.provider = Some(new_provider);
        state.generation = state.generation.saturating_add(1);
        info!(
            provider_id = spec.provider_id,
            model_id = spec.model_id,
            model_version = spec.model_version,
            dimensions = spec.dimensions,
            "mcp_embedding_model_swapped"
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
        let settings = EmbeddingSettings {
            mode: self.config.embedding_mode,
            provider: state.provider.as_deref(),
        };
        let refreshed = state.session.update_files(paths, settings)?;
        state.report = Arc::new(refreshed);
        state.generation = state.generation.saturating_add(1);
        Ok(())
    }
}

impl PipelineSessionBackend {
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
        let min_floor = self.config.min_nodes as usize;
        if parsed.subtree_node_count() < min_floor {
            return Ok(FindSimilarOutput {
                clusters: Vec::new(),
                below_min_nodes: true,
            });
        }
        let clusters = state.report.clusters.clone();
        Ok(FindSimilarOutput {
            clusters: trim_top_n(clusters, top_n),
            below_min_nodes: false,
        })
    }
}

/// Filters the report down to clusters whose occurrences touch
/// `absolute_candidate`.
fn filter_clusters_by_path(
    report: &Report,
    absolute_candidate: &Path,
    root: &Path,
) -> Vec<ReportCluster> {
    report
        .clusters
        .iter()
        .filter(|cluster| {
            cluster
                .occurrences
                .iter()
                .any(|occ| paths_equal(&occ.path, absolute_candidate, root))
        })
        .cloned()
        .collect()
}

/// Filters the report to clusters overlapping
/// `[start_byte, end_byte)` on `absolute_candidate`.
fn filter_clusters_by_range(
    report: &Report,
    absolute_candidate: &Path,
    start_byte: usize,
    end_byte: usize,
    root: &Path,
) -> Vec<ReportCluster> {
    report
        .clusters
        .iter()
        .filter(|cluster| {
            cluster.occurrences.iter().any(|occ| {
                paths_equal(&occ.path, absolute_candidate, root)
                    && occurrence_overlaps(occ, start_byte, end_byte)
            })
        })
        .cloned()
        .collect()
}

/// Returns whether `occ` overlaps `[start_byte, end_byte)`.
fn occurrence_overlaps(occ: &ReportOccurrence, start_byte: usize, end_byte: usize) -> bool {
    occ.start_byte < end_byte && occ.end_byte > start_byte
}

/// Compares an occurrence path (stored relative to the scan root by
/// the renderer) against an absolute path. Matches on either the
/// absolute form or on `root.join(occ.path)`.
fn paths_equal(occurrence_path: &Path, absolute_candidate: &Path, root: &Path) -> bool {
    if occurrence_path == absolute_candidate {
        return true;
    }
    let joined = root.join(occurrence_path);
    if joined == absolute_candidate {
        return true;
    }
    if let Ok(canonical_joined) = std::fs::canonicalize(&joined) {
        if canonical_joined == absolute_candidate {
            return true;
        }
    }
    false
}

/// Trims `clusters` to the top `n` entries (already worst-first).
fn trim_top_n(mut clusters: Vec<ReportCluster>, top_n: usize) -> Vec<ReportCluster> {
    if clusters.len() > top_n {
        clusters.truncate(top_n);
    }
    clusters
}

/// Constructs the synthetic `OllamaModelInfo` entry for the built-in
/// stub provider.
fn stub_model_info() -> OllamaModelInfo {
    OllamaModelInfo {
        name: STUB_PROVIDER_ID.to_owned(),
        bare_id: STUB_PROVIDER_ID.to_owned(),
        digest: "stub-v1".to_owned(),
        size_bytes: 0,
        is_embedding_model: true,
    }
}

/// Resolves the configured provider using the `embedding_mode` /
/// `embedding_provider` / `embedding_model` / `embedding_endpoint`
/// tuple. Mirrors the CLI's provider selection so MCP sessions match
/// batch runs exactly.
fn select_provider(
    config: &SessionBackendConfig,
) -> Result<Option<Box<dyn EmbeddingProvider>>, BackendError> {
    match config.embedding_mode {
        EmbeddingMode::Off => Ok(None),
        EmbeddingMode::Auto | EmbeddingMode::Required => match config.embedding_provider.as_str() {
            STUB_PROVIDER_ID => Ok(Some(Box::new(StubProvider::new()))),
            DEFAULT_PROVIDER_ID => match OllamaProvider::connect(
                &config.embedding_endpoint,
                &config.embedding_model,
            ) {
                Ok(provider) => Ok(Some(Box::new(provider))),
                Err(err) if matches!(config.embedding_mode, EmbeddingMode::Auto) => {
                    warn!(reason = %err, "ollama_unreachable_embedding_disabled_auto");
                    Ok(None)
                }
                Err(err) => Err(err.into()),
            },
            other => Err(BackendError::UnknownEmbeddingProvider(other.to_owned())),
        },
    }
}

/// Locks the backend state, mapping poisoning onto a stable error.
fn lock_state(mutex: &Mutex<SessionState>) -> Result<MutexGuard<'_, SessionState>, BackendError> {
    mutex.lock().map_err(|_poisoned| BackendError::MutexPoisoned)
}

/// Returns the distinct set of registered language ids in stable
/// alphabetical order.
fn distinct_languages(session: &PipelineSession) -> Vec<String> {
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for parser in session.parsers() {
        let _inserted = ids.insert(parser.id().to_owned());
    }
    ids.into_iter().collect()
}
