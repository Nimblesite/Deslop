//! [`McpBackend`] — the trait the server dispatches tool calls against.
//!
//! The MCP server is a transport adapter; the backend holds the live
//! analysis state. This file ships one concrete implementation,
//! [`StateFileBackend`], which reads `.deslop-cache/live-report.json`
//! written by the LSP server.
//!
//! When `deslop_core::live::LiveApi` lands (P7), a `LiveApiBackend`
//! impl slots in without changing the server code — that is the whole
//! point of this trait ([MCP-WHY-LIVE], [MCP-CAPABILITIES]).

use std::path::{Path, PathBuf};

use crate::NotificationSender;

use deslop_core::{
    report::{CacheStats, ReportCluster},
    CoreError, EmbeddingProvenance, EmbeddingSpec, OllamaModelInfo, Report,
};
use thiserror::Error;

use crate::safety::PathResolutionError;

mod filters;
mod ipc;
mod state;

pub use state::StateFileBackend;

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
    /// A registered embedding provider id is unknown.
    #[error("embedding provider {0:?} is not registered")]
    UnknownEmbeddingProvider(String),
    /// Internal mutex was poisoned. Fatal — the session is toast.
    #[error("backend state mutex poisoned; analysis aborted")]
    MutexPoisoned,
    /// The LSP server is not running — its state file does not exist.
    #[error("LSP is not running — start deslop-lsp to enable this tool")]
    LspNotRunning,
    /// The LSP state file exists but could not be parsed.
    #[error("state file corrupt: {0}")]
    StateFileCorrupt(String),
}

/// Read-only view over the server-facing capabilities of the backend.
///
/// Each method corresponds to exactly one MCP tool (plus the resource
/// reads for `deslop://report` / `deslop://schema`). Every
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
    fn report_get(&self) -> Result<std::sync::Arc<Report>, BackendError>;

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
    /// changed. The [`StateFileBackend`] implementation first asks the
    /// running LSP to execute `deslop.lsp.refreshReport` over IPC, then
    /// reloads the LSP-written state file and pushes
    /// `notifications/resources/updated` + `notifications/deslop/reportChanged`
    /// through the stored sender ([MCP-NOTIFICATIONS]).
    ///
    /// # Errors
    ///
    /// Propagates [`BackendError::Core`] on analysis failure.
    fn mark_changed(&self, paths: &[PathBuf]) -> Result<(), BackendError>;

    /// Wires the shared writer so the backend can push server →
    /// client notifications from any thread ([MCP-NOTIFICATIONS]).
    ///
    /// Called once by [`McpServer::run`] before the read loop starts.
    fn set_notification_sender(&self, sender: NotificationSender);
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

/// Knobs for constructing a [`StateFileBackend`].
#[derive(Debug, Clone)]
pub struct SessionBackendConfig {
    /// Workspace root to analyse.
    pub root: PathBuf,
    /// Optional `.deslop.toml` override path.
    pub config_path: Option<PathBuf>,
}
