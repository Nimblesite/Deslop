//! Generated wire-format models for the Deslop live IPC surface.
//!
//! Source: `docs/models/live-ipc.td` (typeDiagram).
//! Generator: `scripts/typediagram-gen.mjs`.
//!
//! DO NOT EDIT BY HAND. Re-run `make typediagram-gen` (or any cargo
//! build) to regenerate. This file is gitignored.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
