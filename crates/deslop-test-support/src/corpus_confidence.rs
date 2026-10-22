//! Corpus assertions for mass-only cluster reports.

use serde_json::Value;

use crate::{
    corpus::{
        cluster_shows_span, field_u64, reports_clone_spanning, visible_clone_clusters,
        visible_clusters, Failure,
    },
    // [RANK-STRUCTURAL-ONLY] Informational families are outside clone mass and ranking.
    corpus_score::is_informational_finding,
};

/// Canonical extent field on a mass-only cluster.
const CANONICAL_NODE_COUNT: &str = "canonical_node_count";
/// Visible member-count field on a mass-only cluster.
const OCCURRENCE_COUNT: &str = "occurrence_count";
/// Carried member-count field, hidden occurrences included.
const OCCURRENCES_TOTAL: &str = "occurrences_total";
/// Canonical duplicated-mass field.
const MASS: &str = "mass";
/// Engine-stamped global order field.
const RANK: &str = "rank";
/// Informational records carry no duplicate quantity.
const INFORMATIONAL_QUANTITY: u64 = 0;
/// A family requires two carried occurrences, including hidden copies.
const MIN_CARRIED_OCCURRENCES: u64 = 2;
/// A canonical occurrence is not an additional copy.
const CANONICAL_OCCURRENCE: u64 = 1;
/// Reports number their positions from one.
const FIRST_POSITION: usize = 1;
/// Contract check identifiers shared by diagnostics and their assertions.
const MASS_CHECK: &str = "cluster_mass";
/// Engine-stamped rank contract identifier.
const RANK_CHECK: &str = "cluster_rank";
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

/// Enforces `mass = canonical_nodes × max(visible_occurrences - 1, 0)`
/// and the two-occurrence floor a duplicate is defined by.
///
/// The floor counts *carried* occurrences, not visible ones. A cluster
/// whose only visible occurrence sits beside `report_hide`-suppressed
/// copies is kept intact and shown to the user, so they see regular code
/// duplicating generated code ([EXCLUSION-CONFIG]); reading the floor off
/// the visible count would condemn exactly that cluster, and the gate
/// would demand the engine drop a finding the spec requires it to
/// publish. Such a cluster carries mass 0 by the same equation, which is
/// what sinks it to the bottom of the ranking.
fn check_mass(cluster: &Value, index: usize, failures: &mut Vec<Failure>) {
    let carried = field_u64(cluster, OCCURRENCES_TOTAL);
    let expected = expected_mass(cluster);
    let actual = field_u64(cluster, MASS);
    if carried < MIN_CARRIED_OCCURRENCES || actual != expected {
        failures.push(Failure::new(
            MASS_CHECK,
            mass_detail(cluster, index, expected),
        ));
    }
}

/// [RANK-MASS-SUM] Only clones carry duplicated AST mass.
fn expected_mass(cluster: &Value) -> u64 {
    if is_informational_finding(cluster) {
        return INFORMATIONAL_QUANTITY;
    }
    field_u64(cluster, CANONICAL_NODE_COUNT)
        .saturating_mul(field_u64(cluster, OCCURRENCE_COUNT).saturating_sub(CANONICAL_OCCURRENCE))
}

/// The printed equation follows the record's actual clone eligibility.
fn mass_expectation(cluster: &Value, expected: u64) -> String {
    if is_informational_finding(cluster) {
        return format!("zero duplicate mass ({expected}) for informational findings");
    }
    format!(
        "{} × max({} - {CANONICAL_OCCURRENCE}, 0) = {expected}",
        field_u64(cluster, CANONICAL_NODE_COUNT),
        field_u64(cluster, OCCURRENCE_COUNT)
    )
}

/// Mass diagnostics explain both visible and carried counts without attributing mass to information.
fn mass_detail(cluster: &Value, index: usize, expected: u64) -> String {
    let position = index.saturating_add(FIRST_POSITION);
    let actual = field_u64(cluster, MASS);
    let carried = field_u64(cluster, OCCURRENCES_TOTAL);
    let occurrences = field_u64(cluster, OCCURRENCE_COUNT);
    let equation = mass_expectation(cluster, expected);
    format!("cluster {position} has mass {actual} with {carried} carried and {occurrences} visible occurrences; expected {equation} from at least {MIN_CARRIED_OCCURRENCES} carried")
}

/// Enforces one-based report order on the engine-stamped rank.
fn check_rank(cluster: &Value, index: usize, failures: &mut Vec<Failure>) {
    let expected = if is_informational_finding(cluster) {
        INFORMATIONAL_QUANTITY
    } else {
        u64::try_from(index.saturating_add(FIRST_POSITION)).unwrap_or(u64::MAX)
    };
    let actual = field_u64(cluster, RANK);
    if actual != expected {
        failures.push(Failure::new(
            RANK_CHECK,
            format!(
                "cluster at position {} carries rank {actual}; expected {expected}",
                index.saturating_add(FIRST_POSITION)
            ),
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
    let Some((rank, _)) = visible_clone_clusters(report)
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
    let widest = widest_curated_clone(report, &files);
    if widest < min_nodes {
        failures.push(Failure::new("type2_recall", format!("widest cluster spanning {files:?} has {widest} canonical nodes; expected at least {min_nodes}. Hand-verified Type-2 rename: {why}")));
    }
}

/// Only published clones can establish a curated family's extent.
fn widest_curated_clone(report: &Value, files: &[String]) -> u64 {
    visible_clone_clusters(report)
        .into_iter()
        .filter(|cluster| cluster_shows_span(cluster, files))
        .map(|cluster| field_u64(cluster, CANONICAL_NODE_COUNT))
        .max()
        .unwrap_or_default()
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
