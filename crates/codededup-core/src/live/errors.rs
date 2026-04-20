//! Error type for the live module ([LIVE-PACKAGING]).
//!
//! Mirrors the JSON-RPC fault model the LSP / MCP transports expose.
//! Each variant carries enough structured context that a transport
//! adapter can lift it into a JSON-RPC error without losing fields.
//! [`LiveErrorWire`] is the serialisable shape consumed by transports.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    /// Wraps any [`CoreError`] surfaced by the underlying pipeline.
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// Serialisable wire shape for [`LiveError`]. Transports lift
/// `LiveError` into this struct before encoding so the JSON-RPC error
/// payload is stable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveErrorWire {
    /// Short machine-readable identifier (e.g. `"unparseable_input"`).
    pub code: String,
    /// Human-readable rendering, equivalent to `format!("{err}")`.
    pub message: String,
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
            Self::PathOutsideWorkspace { .. } => "path_outside_workspace",
            Self::UnknownCluster { .. } => "unknown_cluster",
            Self::ProviderUnreachable { .. } => "provider_unreachable",
            Self::SchedulerBusy { .. } => "scheduler_busy",
            Self::Core(_) => "core_error",
        }
    }
}
