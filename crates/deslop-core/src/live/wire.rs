//! Serialisable DTOs shared by [`super::api::LiveApi`] and the LSP /
//! MCP transports ([LIVE-QUERY-API], [LIVE-NOTIFICATIONS]).
//!
//! Every type in this module is `serde::Serialize + Deserialize` so a
//! transport can lift it onto the wire without translation. Field
//! names use `snake_case` to match the JSON-RPC payloads documented
//! in `docs/specs/lsp.md` and `docs/specs/mcp.md`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    delta::ReportDelta,
    report::{EmbeddingProvenance, ReportCluster},
};

/// Input to `duplicates/findSimilar`. Discriminated by the `kind`
/// field so a transport can dispatch on the variant without a
/// secondary lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FindSimilarInput {
    /// Look up clusters whose occurrences overlap the byte range in
    /// the open buffer at `path`.
    OpenRange {
        /// Workspace-relative or absolute path to the open file.
        path: PathBuf,
        /// Inclusive byte offset of the range start.
        start_byte: usize,
        /// Exclusive byte offset of the range end.
        end_byte: usize,
    },
    /// Parse `snippet` against the registered parser for `language`
    /// and look the resulting fingerprints up in the live corpus.
    Snippet {
        /// Source-text snippet to fingerprint.
        snippet: String,
        /// Language id matching one of the registered parsers
        /// (`"csharp"`, `"rust"`, `"python"`).
        language: String,
    },
}

/// Outer envelope for `duplicates/findSimilar` requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarRequest {
    /// Discriminated input variant.
    pub input: FindSimilarInput,
    /// Optional cap on returned clusters. `None` means "no cap".
    pub max_results: Option<usize>,
}

/// Result of `duplicates/findSimilar`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarResult {
    /// Top-N clusters covering the input, worst-first.
    pub clusters: Vec<ReportCluster>,
    /// `true` when the snippet/range produced no fingerprints because
    /// every subtree fell below the session's `min_nodes` floor. Lets
    /// agents distinguish "no clones" from "subtree too small to
    /// fingerprint" ([MCP-TOOL-FINDSIMILAR]).
    pub below_min_nodes: bool,
}

/// File-scoped subset of a report. Returned by `report/forFile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReport {
    /// Path the report covers, relative to the workspace root when
    /// possible.
    pub path: PathBuf,
    /// Clusters whose occurrences touch `path`, byte-range sorted.
    pub clusters: Vec<ReportCluster>,
}

/// One row from the embedding-model picker. Aggregates Ollama models
/// (`/api/tags`) and the built-in `stub` provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelInfo {
    /// Provider registry key (`"ollama"`, `"stub"`).
    pub provider_id: String,
    /// Human-readable model id (`"nomic-embed-text"`, `"blake3-stub"`).
    pub model_id: String,
    /// Optional opaque version string.
    pub model_version: Option<String>,
    /// Optional dimensionality, when known.
    pub dimensions: Option<usize>,
    /// `true` when the model is recommended for code embeddings.
    pub recommended: bool,
    /// `true` when a probe at listing time confirmed the provider was
    /// reachable. Stub is always reachable.
    pub reachable: bool,
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

/// Compact summary of a [`ReportDelta`] suitable for push
/// notifications. Subscribers that need full payloads call
/// `report/delta`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeSummary {
    /// Number of clusters newly present in the latest generation.
    pub clusters_added: usize,
    /// Number of clusters removed in the latest generation.
    pub clusters_removed: usize,
    /// Number of clusters whose payload changed in the latest
    /// generation.
    pub clusters_updated: usize,
    /// Worst (highest) weight in the latest generation, `0.0` when
    /// the report is empty.
    pub worst_weight: f64,
}

impl ChangeSummary {
    /// Builds a [`ChangeSummary`] from a [`ReportDelta`]. `worst_weight`
    /// is the maximum weight among the clusters surfaced by the delta;
    /// `0.0` when nothing changed.
    #[must_use]
    pub fn from_delta(delta: &ReportDelta) -> Self {
        let worst_weight = delta
            .clusters_added
            .iter()
            .chain(delta.clusters_updated.iter())
            .map(|cluster| cluster.weight)
            .fold(0.0_f64, f64::max);
        Self {
            clusters_added: delta.clusters_added.len(),
            clusters_removed: delta.clusters_removed.len(),
            clusters_updated: delta.clusters_updated.len(),
            worst_weight,
        }
    }
}

/// Wire payload for the `report/changed` notification ([LIVE-NOTIFICATIONS]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportChangedNotification {
    /// New generation that produced the change.
    pub generation: u64,
    /// Compact summary suitable for status indicators.
    pub summary: ChangeSummary,
}

/// Phase of the embedding pass surfaced to the editor via
/// `deslop/embeddingProgress` ([LIVE-NOTIFICATIONS]).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingPhase {
    /// Pass has just begun. `done` is `0`, `total` is populated.
    Starting,
    /// Pass finished successfully. `done == total`.
    Complete,
    /// Pass aborted with `message`. `done` reflects subtrees
    /// embedded before the failure.
    Failed,
}

/// Wire payload for the `deslop/embeddingProgress` notification. Fired
/// around a `deslop/embeddingSetModel` swap so the editor's session
/// panel can render "X / Y subtrees" instead of freezing on the old
/// model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProgress {
    /// Lifecycle phase.
    pub phase: EmbeddingPhase,
    /// Provider id the swap targets (`"ollama"`, `"stub"`).
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

/// Wire payload for the `analysis/state` notification ([LIVE-NOTIFICATIONS]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AnalysisState {
    /// Scheduler is idle — no pass in flight.
    Idle,
    /// Scheduler is processing a pass that started at
    /// `started_at_ms` (clock-relative).
    Running {
        /// Millisecond timestamp the pass started, as reported by the
        /// session's [`super::clock::Clock`].
        started_at_ms: u64,
    },
    /// Scheduler is parked on an error. `message` carries the
    /// upstream diagnostic for surfacing in the editor's status bar.
    Errored {
        /// Human-readable diagnostic.
        message: String,
    },
}
