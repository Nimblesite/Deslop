//! Serialisable DTOs shared by [`super::api::LiveApi`] and the LSP /
//! MCP transports ([LIVE-QUERY-API], [LIVE-NOTIFICATIONS]).
//!
//! Every wire type is generated from `docs/models/live-ipc.td` by
//! `scripts/typediagram/generate.mjs` and re-exported from
//! [`crate::wire_generated`]. This module re-exports them at the
//! historical `crate::live::wire` path and hosts the [`ChangeSummary`]
//! `from_delta` constructor (the only logic kept hand-written; the
//! data shape itself is generated).

use crate::delta::ReportDelta;

pub use crate::wire_generated::{
    AnalysisState, ChangeSummary, EmbeddingModelInfo, EmbeddingPhase, EmbeddingProgress,
    FileReport, FindSimilarInput, FindSimilarRequest, FindSimilarResult, ReportChangedNotification,
    SessionConfig,
};

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
