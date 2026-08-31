//! Unit tests for mass-only corpus confidence assertions.

use super::*;
use serde_json::{json, Value};

/// Returns the single failure when exactly one was reported.
fn only(failures: &[Failure]) -> Option<&Failure> {
    match failures {
        [single] => Some(single),
        _ => None,
    }
}

/// Returns the check id of the single failure.
fn only_check(failures: &[Failure]) -> Option<&str> {
    only(failures).map(|failure| failure.check.as_str())
}

/// Whether the single failure detail contains `needle`.
fn detail_mentions(failures: &[Failure], needle: &str) -> bool {
    only(failures).is_some_and(|failure| failure.detail.contains(needle))
}

/// Asserts one named failure with useful detail.
fn assert_only_failure(
    failures: &[Failure],
    check: &str,
    why_one: &str,
    needle: &str,
    why_detail: &str,
) {
    assert_eq!(failures.len(), 1, "{why_one}");
    assert_eq!(only_check(failures), Some(check), "{why_one}");
    assert!(
        detail_mentions(failures, needle),
        "{why_detail}: {failures:?}"
    );
}

/// One valid mass-only cluster over the supplied visible files.
fn spanning(id: &str, nodes: u64, rank: u64, files: &[&str]) -> Value {
    let occurrences: Vec<Value> = files
        .iter()
        .map(|file| json!({"path": file, "hidden": false}))
        .collect();
    let occurrence_count = u64::try_from(files.len()).unwrap_or(u64::MAX);
    json!({
        "id": id,
        "rank": rank,
        "rank_band": "worst",
        "mass": nodes.saturating_mul(occurrence_count.saturating_sub(1)),
        "canonical_node_count": nodes,
        "occurrence_count": occurrence_count,
        "occurrences": occurrences,
    })
}

/// One mass-only report.
fn report(clusters: &[Value]) -> Value {
    json!({"clusters": clusters})
}

/// Marks one matching occurrence hidden.
fn hide_occurrence(mut cluster: Value, path: &str) -> Value {
    let Some(occurrences) = cluster.get_mut("occurrences").and_then(Value::as_array_mut) else {
        return cluster;
    };
    for occurrence in occurrences {
        if occurrence.get("path").and_then(Value::as_str) == Some(path) {
            let _old = occurrence
                .as_object_mut()
                .and_then(|fields| fields.insert("hidden".to_owned(), Value::Bool(true)));
        }
    }
    cluster
}

mod curated;
mod liveness;
mod recall;
