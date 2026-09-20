//! Shared human-facing cluster presentation for LSP surfaces.

use deslop_core::{
    buckets::ClusterKind,
    report::{occurrence_count, ReportCluster},
};
use serde_json::{json, Value};

/// Formats the diagnostic message: the cluster's clone kind
/// ([CLONE-KIND-LABELS]), its occurrence count, and its mass.
#[must_use]
pub fn diagnostic_message(cluster: &ReportCluster) -> String {
    if cluster.kind == ClusterKind::StructuralOnly {
        return format!(
            "{} — informational, not a clone",
            cluster.kind.labels().title
        );
    }
    let count = occurrence_count(cluster);
    format!(
        "{title} × {count} — mass {mass}",
        title = cluster.kind.labels().title,
        mass = cluster.mass
    )
}

/// Stores machine-facing cluster identity outside visible diagnostic text.
///
/// [LSP-AGENT-FRIENDLY] The cluster id rides the machine-facing `data` so
/// an agent can call `deslop/clusterById` without parsing the message text.
#[must_use]
pub fn diagnostic_data(cluster: &ReportCluster) -> Value {
    let mut data = serde_json::Map::from_iter([
        ("cluster_id".to_owned(), json!(cluster.id)),
        ("kind".to_owned(), json!(cluster.kind.wire_label())),
    ]);
    if cluster.kind != ClusterKind::StructuralOnly {
        data.extend([
            ("mass".to_owned(), json!(cluster.mass)),
            ("rank".to_owned(), json!(cluster.rank)),
        ]);
    }
    Value::Object(data)
}
