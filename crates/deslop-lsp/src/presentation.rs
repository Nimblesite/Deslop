//! Shared human-facing cluster presentation for LSP surfaces.

use deslop_core::{
    buckets::{bucket_labels, classify},
    render::signals::plain_explanation,
    report::{occurrence_count, ReportCluster},
};
use serde_json::{json, Value};

/// Formats the diagnostic message: category × count — action sentence —
/// confidence explanation.
///
/// [FUSED-CONTENT-GATE] The trailing explanation is the one shared
/// `render::signals` rendering of the fused confidence and the measured
/// content evidence. Without it the bucket title is unfalsifiable: a
/// corroborated Type-2 rename and an anchor-poor scaffolding family both
/// show `structural 1.00`, and only `agreement` / `rename` / `literal`
/// tell the reader which one is on screen.
#[must_use]
pub fn diagnostic_message(cluster: &ReportCluster) -> String {
    let labels = bucket_labels(classify(cluster));
    let count = occurrence_count(cluster);
    format!(
        "{} × {} — {} — {}",
        labels.plain_title,
        count,
        labels.action_sentence,
        plain_explanation(cluster.signals),
    )
}

/// Stores machine-facing cluster identity outside visible diagnostic text.
///
/// [LSP-AGENT-FRIENDLY] The cluster id rides the machine-facing `data` so
/// an agent can call `deslop/clusterById` without parsing the message text.
#[must_use]
pub fn diagnostic_data(cluster: &ReportCluster) -> Value {
    let labels = bucket_labels(classify(cluster));
    json!({
        "cluster_id": cluster.id.as_str(),
        "taxonomy": labels.taxonomy_label,
    })
}
