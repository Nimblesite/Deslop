//! Shared human-facing cluster presentation for LSP surfaces.

use deslop_core::{
    buckets::{bucket_labels, classify},
    report::{occurrence_count, ReportCluster},
};
use serde_json::{json, Value};

/// Formats the cluster headline used by hover and diagnostics.
#[must_use]
pub fn cluster_summary(cluster: &ReportCluster, rank: Option<usize>) -> String {
    let labels = bucket_labels(classify(cluster));
    format!(
        "{rank}{title} — {action} {occurrences}.",
        rank = rank_prefix(rank),
        title = labels.plain_title,
        action = labels.action_sentence,
        occurrences = occurrence_phrase(cluster),
    )
}

/// Formats the diagnostic message without exposing the stable hash.
#[must_use]
pub fn diagnostic_message(cluster: &ReportCluster) -> String {
    let labels = bucket_labels(classify(cluster));
    format!("{} — {}", labels.plain_title, labels.action_sentence)
}

/// Stores machine-facing cluster identity outside visible diagnostic text.
#[must_use]
pub fn diagnostic_data(cluster: &ReportCluster) -> Value {
    let labels = bucket_labels(classify(cluster));
    json!({
        "cluster_id": cluster.id.as_str(),
        "taxonomy": labels.taxonomy_label,
    })
}

/// Formats the four signal scores as one compact sentence.
#[must_use]
pub fn signal_sentence(cluster: &ReportCluster) -> String {
    format!(
        "signals: structural {structural:.2}, jaccard {jaccard:.2}, embedding {embedding:.2}, fused {fused:.2}.",
        structural = cluster.signals.structural,
        jaccard = cluster.signals.token_jaccard,
        embedding = cluster.signals.embedding_cos,
        fused = cluster.signals.fused,
    )
}

/// Formats `N occurrence(s)` with the authoritative cluster count.
fn occurrence_phrase(cluster: &ReportCluster) -> String {
    let count = occurrence_count(cluster);
    let suffix = if count == 1 { "occurrence" } else { "occurrences" };
    format!("{count} {suffix}")
}

/// Formats the optional global impact rank prefix.
fn rank_prefix(rank: Option<usize>) -> String {
    rank.map_or_else(String::new, |value| format!("#{value} "))
}
