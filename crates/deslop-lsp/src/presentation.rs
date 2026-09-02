//! Shared human-facing cluster presentation for LSP surfaces.

use deslop_core::report::{occurrence_count, ReportCluster};
use serde_json::{json, Value};

/// Formats a neutral mass-only diagnostic message.
#[must_use]
pub fn diagnostic_message(cluster: &ReportCluster) -> String {
    let count = occurrence_count(cluster);
    format!("Duplicate code × {count} — mass {}", cluster.mass)
}

/// Stores machine-facing cluster identity outside visible diagnostic text.
///
/// [LSP-AGENT-FRIENDLY] The cluster id rides the machine-facing `data` so
/// an agent can call `deslop/clusterById` without parsing the message text.
#[must_use]
pub fn diagnostic_data(cluster: &ReportCluster) -> Value {
    json!({
        "cluster_id": cluster.id.as_str(),
        "mass": cluster.mass,
        "rank": cluster.rank,
        "rank_band": cluster.rank_band.as_str(),
    })
}
