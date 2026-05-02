//! Generated wire-format models for the Deslop live IPC surface.
//!
//! Source: `docs/models/live-ipc.td` (typeDiagram).
//! Generator: `scripts/typediagram-gen.mjs`.
//!
//! DO NOT EDIT BY HAND. Re-run `make typediagram-gen` (or any cargo
//! build) to regenerate. This file is gitignored.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::report::EmbeddingProvenance;
use crate::report::ReportCluster;

/// One row from the Ollama `/api/tags` enumeration. See `docs/models/live-ipc.td`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    /// Full model tag as installed (`nomic-embed-text:latest`).
    pub name: String,
    /// Tag-stripped model id.
    pub bare_id: String,
    /// Truncated content digest (12 hex chars).
    pub digest: String,
    /// Packaged model size in bytes.
    pub size_bytes: u64,
    /// True when a probe returned a non-empty vector.
    pub is_embedding_model: bool,
}

/// One row of the `embedding/listModels` response. See `docs/models/live-ipc.td`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelInfo {
    /// Provider registry key (`ollama`, `stub`).
    pub provider_id: String,
    /// Human-readable model id.
    pub model_id: String,
    /// Optional opaque version string.
    pub model_version: Option<String>,
    /// Optional dimensionality, when known.
    pub dimensions: Option<usize>,
    /// True when recommended for code embeddings.
    pub recommended: bool,
    /// True when the provider was reachable at listing time.
    pub reachable: bool,
}

/// Discriminated input to `duplicates/findSimilar`. See `docs/models/live-ipc.td`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FindSimilarInput {
    /// Look up clusters overlapping a byte range in an open file.
    OpenRange {
        /// Workspace-relative or absolute path.
        path: PathBuf,
        /// Inclusive byte offset of the range start.
        start_byte: usize,
        /// Exclusive byte offset of the range end.
        end_byte: usize,
    },
    /// Parse a snippet against a registered language and look up.
    Snippet {
        /// Source-text snippet to fingerprint.
        snippet: String,
        /// Registered language id (`csharp`, `rust`, `python`).
        language: String,
    },
}

/// Outer envelope for `duplicates/findSimilar` requests. See `docs/models/live-ipc.td`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarRequest {
    /// Discriminated input variant.
    pub input: FindSimilarInput,
    /// Optional cap on returned clusters; `None` means no cap.
    pub max_results: Option<usize>,
}

/// Result of `duplicates/findSimilar`. See `docs/models/live-ipc.td`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarResult {
    /// Top-N clusters covering the input, worst-first.
    pub clusters: Vec<ReportCluster>,
    /// True when every subtree fell below the session's `min_nodes` floor.
    pub below_min_nodes: bool,
}

/// File-scoped subset of a report; returned by `report/forFile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReport {
    /// Path the report covers, workspace-relative when possible.
    pub path: PathBuf,
    /// Clusters whose occurrences touch `path`, byte-range sorted.
    pub clusters: Vec<ReportCluster>,
}

/// Snapshot of the session's resolved configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Workspace root pinned at session creation.
    pub workspace_root: PathBuf,
    /// Subtree-size floor used throughout the session.
    pub min_nodes: u32,
    /// Languages with registered parsers in the session.
    pub languages: Vec<String>,
    /// Currently-active embedding provenance, if any.
    pub embedding_provenance: Option<EmbeddingProvenance>,
    /// Optional explicit exclusion-config path.
    pub exclusion_config_path: Option<PathBuf>,
    /// Cache root (`<workspace>/.deslop-cache`).
    pub cache_root: PathBuf,
    /// Whether the session was created with the incremental cache on.
    pub incremental: bool,
}

/// Compact summary of a `ReportDelta` for push notifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeSummary {
    /// Number of clusters newly present in the latest generation.
    pub clusters_added: usize,
    /// Number of clusters removed in the latest generation.
    pub clusters_removed: usize,
    /// Number of clusters whose payload changed.
    pub clusters_updated: usize,
    /// Worst (highest) weight in the latest generation, `0.0` when empty.
    pub worst_weight: f64,
}

/// Wire payload for the `report/changed` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportChangedNotification {
    /// New generation that produced the change.
    pub generation: u64,
    /// Compact summary suitable for status indicators.
    pub summary: ChangeSummary,
}

/// Phase of the embedding pass surfaced via `deslop/embeddingProgress`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingPhase {
    /// User selected a model and the low-priority pass is queued.
    Queued,
    /// Pass has just begun. `done` is `0`, `total` is populated.
    Starting,
    /// Pass is actively working through provider batches.
    Running,
    /// Pass finished successfully. `done == total`.
    Complete,
    /// Pass aborted with `message`. `done` reflects work before the failure.
    Failed,
}

/// Wire payload for the `deslop/embeddingProgress` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProgress {
    /// Lifecycle phase.
    pub phase: EmbeddingPhase,
    /// Provider id the swap targets (`ollama`, `stub`).
    pub provider_id: String,
    /// Model id the swap targets.
    pub model_id: String,
    /// Subtrees embedded so far.
    pub done: u64,
    /// Total subtrees in the current corpus.
    pub total: u64,
    /// Diagnostic message populated only when `phase == Failed`.
    pub message: Option<String>,
}

/// Wire payload for the `analysis/state` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AnalysisState {
    /// Scheduler is idle — no pass in flight.
    Idle,
    /// Scheduler is processing a pass started at `started_at_ms`.
    Running {
        /// Millisecond timestamp the pass started.
        started_at_ms: u64,
    },
    /// Scheduler is parked on an error; `message` carries the diagnostic.
    Errored {
        /// Human-readable diagnostic.
        message: String,
    },
}
