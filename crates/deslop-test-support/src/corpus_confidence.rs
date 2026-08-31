//! Corpus assertions for mass-only cluster reports.

use serde_json::Value;

use crate::corpus::{
    cluster_shows_span, field_u64, reports_clone_spanning, visible_clusters, Failure,
};

/// Canonical extent field on a mass-only cluster.
const CANONICAL_NODE_COUNT: &str = "canonical_node_count";
/// Visible member-count field on a mass-only cluster.
const OCCURRENCE_COUNT: &str = "occurrence_count";
/// Canonical duplicated-mass field.
const MASS: &str = "mass";
/// Engine-stamped global order field.
const RANK: &str = "rank";
/// Curated manifest extent floor.
const CURATED_EXTENT_FIELD: &str = "min_nodes";
/// Fields forbidden because they belong to pairs or retired presentation policy.
const FORBIDDEN_CLUSTER_FIELDS: [&str; 11] = [
    "signals",
    "signal_source",
    "content",
    "evidence_verdict",
    "bucket",
    "category",
    "classification",
    "weight",
    "summary",
    "interpretation",
    "language",
];

/// Verifies the exhaustive cluster schema, mass equation, and order.
pub fn check_cluster_mass_contract(report: &Value, failures: &mut Vec<Failure>) {
    for (index, cluster) in visible_clusters(report).into_iter().enumerate() {
        check_forbidden_fields(cluster, index, failures);
        check_mass(cluster, index, failures);
        check_rank(cluster, index, failures);
    }
}

/// Rejects pair evidence and presentation classifications on clusters.
fn check_forbidden_fields(cluster: &Value, index: usize, failures: &mut Vec<Failure>) {
    let leaked: Vec<&str> = FORBIDDEN_CLUSTER_FIELDS
        .iter()
        .copied()
        .filter(|field| cluster.get(field).is_some())
        .collect();
    if !leaked.is_empty() {
        failures.push(Failure::new(
            "cluster_contract",
            format!(
                "cluster {} leaks forbidden pair/presentation fields: {leaked:?}",
                index.saturating_add(1)
            ),
        ));
    }
}

/// Enforces `mass = canonical_nodes × max(visible_occurrences - 1, 0)`.
fn check_mass(cluster: &Value, index: usize, failures: &mut Vec<Failure>) {
    let nodes = field_u64(cluster, CANONICAL_NODE_COUNT);
    let occurrences = field_u64(cluster, OCCURRENCE_COUNT);
    let expected = nodes.saturating_mul(occurrences.saturating_sub(1));
    let actual = field_u64(cluster, MASS);
    if occurrences < 2 || actual != expected {
        failures.push(Failure::new(
            "cluster_mass",
            format!("cluster {} has mass {actual}; expected {nodes} × max({occurrences} - 1, 0) = {expected}", index.saturating_add(1)),
        ));
    }
}

/// Enforces one-based report order on the engine-stamped rank.
fn check_rank(cluster: &Value, index: usize, failures: &mut Vec<Failure>) {
    let expected = u64::try_from(index.saturating_add(1)).unwrap_or(u64::MAX);
    let actual = field_u64(cluster, RANK);
    if actual != expected {
        failures.push(Failure::new(
            "cluster_rank",
            format!("cluster at position {expected} carries rank {actual}"),
        ));
    }
}

/// Verifies every curated exact-copy family is visible and within its rank ceiling.
pub fn check_curated_recall(manifest: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let entries = manifest
        .get("must_find")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    for entry in entries {
        check_one_curated_clone(entry, report, failures);
    }
}

/// Verifies every curated Type-2 family is visible at its curated extent.
pub fn check_type2_curated_recall(manifest: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let entries = manifest
        .get("must_find_type2")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    for entry in entries {
        check_one_curated_type2(entry, report, failures);
    }
}

/// Checks one curated exact-copy family.
fn check_one_curated_clone(entry: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let files = curated_files(entry);
    let why = entry.get("why").and_then(Value::as_str).unwrap_or("");
    let Some((rank, _)) = visible_clusters(report)
        .into_iter()
        .enumerate()
        .find(|(_, cluster)| cluster_shows_span(cluster, &files))
    else {
        failures.push(Failure::new(
            "recall",
            format!("no cluster spans {files:?}. Verified duplicate: {why}"),
        ));
        return;
    };
    check_rank_ceiling(entry, rank.saturating_add(1), &files, why, failures);
}

/// Checks one curated Type-2 family without asking a cluster for pair evidence.
fn check_one_curated_type2(entry: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let files = curated_files(entry);
    let why = entry.get("why").and_then(Value::as_str).unwrap_or("");
    let Some(min_nodes) = entry.get(CURATED_EXTENT_FIELD).and_then(Value::as_u64) else {
        failures.push(Failure::new("type2_recall", format!("entry for {files:?} lacks `{CURATED_EXTENT_FIELD}`. Hand-verified Type-2 rename: {why}")));
        return;
    };
    if !reports_clone_spanning(report, &files) {
        failures.push(Failure::new(
            "type2_recall",
            format!("no cluster spans {files:?}. Hand-verified Type-2 rename: {why}"),
        ));
        return;
    }
    let widest = visible_clusters(report)
        .into_iter()
        .filter(|cluster| cluster_shows_span(cluster, &files))
        .map(|cluster| field_u64(cluster, CANONICAL_NODE_COUNT))
        .max()
        .unwrap_or_default();
    if widest < min_nodes {
        failures.push(Failure::new("type2_recall", format!("widest cluster spanning {files:?} has {widest} canonical nodes; expected at least {min_nodes}. Hand-verified Type-2 rename: {why}")));
    }
}

/// Applies an optional curated maximum rank.
fn check_rank_ceiling(
    entry: &Value,
    rank: usize,
    files: &[String],
    why: &str,
    failures: &mut Vec<Failure>,
) {
    let Some(ceiling) = entry.get("max_rank").and_then(Value::as_u64) else {
        return;
    };
    if u64::try_from(rank).unwrap_or(u64::MAX) > ceiling {
        failures.push(Failure::new("recall_quality", format!("verified duplicate spanning {files:?} ranks {rank} below curated ceiling {ceiling}. Verified duplicate: {why}")));
    }
}

/// Returns a curated file list only when it names at least two files.
fn curated_files(entry: &Value) -> Vec<String> {
    let files: Vec<String> = entry
        .get("files")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|file| file.as_str().map(ToOwned::to_owned))
        .collect();
    if files.len() >= 2 {
        files
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
