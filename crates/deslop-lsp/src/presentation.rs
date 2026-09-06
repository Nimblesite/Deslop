//! Shared human-facing cluster presentation for LSP surfaces.

use deslop_core::{
    buckets::{bucket_labels, classify},
    report::{occurrence_count, ReportCluster},
};
use serde_json::{json, Value};

/// Formats the diagnostic message: category × count — action sentence.
#[must_use]
pub fn diagnostic_message(cluster: &ReportCluster) -> String {
    let labels = bucket_labels(classify(cluster));
    let count = occurrence_count(cluster);
    format!(
        "{} × {} — {}",
        labels.plain_title, count, labels.action_sentence
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
