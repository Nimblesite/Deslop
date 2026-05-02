//! Error type for the live module ([LIVE-PACKAGING]).
//!
//! Mirrors the JSON-RPC fault model the LSP / MCP transports expose.
//! Each variant carries enough structured context that a transport
//! adapter can lift it into a JSON-RPC error without losing fields.
//! [`LiveErrorWire`] is the serialisable shape consumed by transports;
//! its definition lives in `docs/models/live-ipc.td` and is re-exported
//! here so callers keep the single import path they already use.

use std::path::PathBuf;

use thiserror::Error;

pub use crate::wire_generated::LiveErrorWire;

use crate::error::CoreError;

/// Errors produced by the live module.
#[derive(Debug, Error)]
pub enum LiveError {
    /// The snippet or open-buffer range submitted to
    /// `duplicates/findSimilar` could not be parsed by the registered
    /// language parser.
    #[error("unparseable input at bytes {start_byte}..{end_byte}: {message}")]
    UnparseableInput {
        /// Optional path of the file the range refers to.
        path: Option<PathBuf>,
        /// Inclusive start byte of the range.
        start_byte: usize,
        /// Exclusive end byte of the range.
        end_byte: usize,
        /// Parser-supplied diagnostic message.
        message: String,
    },
    /// A snippet was submitted with a `language` field that no
    /// registered [`crate::lang::LanguageParser`] claims.
    #[error("language {requested} is not supported (registered: {registered:?})")]
    UnsupportedLanguage {
        /// Language id the caller asked for.
        requested: String,
        /// Languages the session has parsers for.
        registered: Vec<String>,
    },
    /// `embedding/setModel` was called with a `provider_id` that does
    /// not match a registered embedding provider.
    #[error("provider {requested} is not supported (registered: {registered:?})")]
    UnsupportedProvider {
        /// Provider id the caller asked for.
        requested: String,
        /// Providers the session supports.
        registered: Vec<String>,
    },
    /// A request referenced a path outside the live workspace root.
    /// Live sessions never touch files outside the root they were
    /// constructed against ([MCP-SAFETY]).
    #[error("path {path:?} is outside workspace root {workspace_root:?}")]
    PathOutsideWorkspace {
        /// Offending path.
        path: PathBuf,
        /// Pinned workspace root.
        workspace_root: PathBuf,
    },
    /// `cluster/byId` was called with an id that does not appear in
    /// the current report.
    #[error("unknown cluster id {id}")]
    UnknownCluster {
        /// Caller-supplied cluster id.
        id: String,
    },
    /// An embedding provider could not be reached. Wraps the upstream
    /// transport diagnostic.
    #[error("embedding provider at {endpoint} unreachable: {message}")]
    ProviderUnreachable {
        /// Endpoint that failed to respond.
        endpoint: String,
        /// Upstream transport message.
        message: String,
    },
    /// The scheduler refused to dispatch because it is already
    /// processing an in-flight pass.
    #[error("scheduler busy: {message}")]
    SchedulerBusy {
        /// Diagnostic context.
        message: String,
    },
    /// The filesystem watcher could not be started — e.g. OS-level
    /// permission denied on the workspace root ([LIVE-WATCHER]).
    #[error("filesystem watcher failed to start: {message}")]
    WatcherInit {
        /// OS or `notify` diagnostic message.
        message: String,
    },
    /// The session was constructed from a cached report but the
    /// background pipeline pass that backs parser-driven queries has
    /// not yet completed ([LIVE-CACHE-SEED]).
    #[error("analysis pipeline not ready yet")]
    AnalysisNotReady,
    /// Wraps any [`CoreError`] surfaced by the underlying pipeline.
    #[error(transparent)]
    Core(#[from] CoreError),
}

impl LiveError {
    /// Lifts `self` into the serialisable [`LiveErrorWire`] shape.
    #[must_use]
    pub fn to_wire(&self) -> LiveErrorWire {
        LiveErrorWire {
            code: self.code().to_owned(),
            message: self.to_string(),
        }
    }

    /// Returns the short code for this variant.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnparseableInput { .. } => "unparseable_input",
            Self::UnsupportedLanguage { .. } => "unsupported_language",
            Self::UnsupportedProvider { .. } => "unsupported_provider",
            Self::PathOutsideWorkspace { .. } => "path_outside_workspace",
            Self::UnknownCluster { .. } => "unknown_cluster",
            Self::ProviderUnreachable { .. } => "provider_unreachable",
            Self::SchedulerBusy { .. } => "scheduler_busy",
            Self::WatcherInit { .. } => "watcher_init",
            Self::AnalysisNotReady => "analysis_not_ready",
            Self::Core(_) => "core_error",
        }
    }
}
