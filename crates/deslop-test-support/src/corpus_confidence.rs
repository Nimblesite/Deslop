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
/// Curated manifest rank ceiling, inclusive and optional per entry.
const CURATED_RANK_FIELD: &str = "max_rank";
/// The only classification a curated Type-2 rename may reach: it is not
/// byte-identical by definition, so `identical` is unreachable, and
/// anything looser means the content gate did not vouch for the rename.
const VOUCHED_TYPE2_CLASSIFICATION: &str = "nearly_identical";
/// Check id for curated exact-copy membership.
const RECALL: &str = "recall";
/// Check id for the quality clauses on a curated exact copy.
const RECALL_QUALITY: &str = "recall_quality";
/// Check id for curated Type-2 rename recall.
const TYPE2_RECALL: &str = "type2_recall";
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

/// The only classification a curated byte-identical pair may reach.
/// `must_find` entries are verified byte-for-byte, so anything looser is
/// the engine contradicting a proven fact about the source.
const VOUCHED_EXACT_CLASSIFICATION: &str = "identical";

/// Verifies every curated exact-copy family is visible and within its rank ceiling.
pub fn check_curated_recall(
    manifest: &Value,
    report: &Value,
    verdicts: &[Value],
    failures: &mut Vec<Failure>,
) {
    let entries = manifest
        .get("must_find")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    for entry in entries {
        check_one_curated_clone(entry, report, verdicts, failures);
    }
}

/// Verifies every curated Type-2 family is visible at its curated extent.
pub fn check_type2_curated_recall(
    manifest: &Value,
    report: &Value,
    verdicts: &[Value],
    failures: &mut Vec<Failure>,
) {
    let entries = manifest
        .get("must_find_type2")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    for entry in entries {
        check_one_curated_type2(entry, report, verdicts, failures);
    }
}

/// Checks one curated exact-copy family.
fn check_one_curated_clone(
    entry: &Value,
    report: &Value,
    verdicts: &[Value],
    failures: &mut Vec<Failure>,
) {
    let files = curated_files(entry);
    let why = entry.get("why").and_then(Value::as_str).unwrap_or("");
    let Some((rank, _)) = visible_clusters(report)
        .into_iter()
        .enumerate()
        .find(|(_, cluster)| cluster_shows_span(cluster, &files))
    else {
        failures.push(Failure::new(
            RECALL,
            format!("no cluster spans {files:?}. Verified duplicate: {why}"),
        ));
        return;
    };
    check_rank_ceiling(
        entry,
        rank.saturating_add(1),
        &files,
        why,
        RECALL_QUALITY,
        failures,
    );
    check_pair_identical(&files, why, verdicts, failures);
}

/// The classification clause for a curated byte-identical pair.
fn check_pair_identical(
    files: &[String],
    why: &str,
    verdicts: &[Value],
    failures: &mut Vec<Failure>,
) {
    let Some(evidence) = verdict_for(files, verdicts) else {
        failures.push(Failure::new(RECALL_QUALITY, format!("no admission evidence was obtained for {files:?}, so the clause holding a byte-identical pair to `{VOUCHED_EXACT_CLASSIFICATION}` judged nothing. Verified duplicate: {why}")));
        return;
    };
    let classification = evidence
        .get("classification")
        .and_then(Value::as_str)
        .unwrap_or("absent");
    if classification != VOUCHED_EXACT_CLASSIFICATION {
        failures.push(Failure::new(RECALL_QUALITY, format!("the curated byte-identical pair spanning {files:?} is classified `{classification}`, not `{VOUCHED_EXACT_CLASSIFICATION}` — the engine is contradicting a fact verified byte-for-byte about the source. Verified duplicate: {why}")));
    }
}

/// Where the curated extent floor landed among the spanning clusters.
enum CuratedExtent {
    /// Best report position of a spanning cluster at or above the floor.
    Reached(usize),
    /// Widest spanning extent found, still short of the floor.
    Short(u64),
}

/// Checks one curated Type-2 family without asking a cluster for pair evidence.
fn check_one_curated_type2(
    entry: &Value,
    report: &Value,
    verdicts: &[Value],
    failures: &mut Vec<Failure>,
) {
    let files = curated_files(entry);
    let why = entry.get("why").and_then(Value::as_str).unwrap_or("");
    let Some(min_nodes) = entry.get(CURATED_EXTENT_FIELD).and_then(Value::as_u64) else {
        failures.push(Failure::new(TYPE2_RECALL, format!("entry for {files:?} lacks `{CURATED_EXTENT_FIELD}`. Hand-verified Type-2 rename: {why}")));
        return;
    };
    if !reports_clone_spanning(report, &files) {
        failures.push(Failure::new(
            TYPE2_RECALL,
            format!("no cluster spans {files:?}. Hand-verified Type-2 rename: {why}"),
        ));
        return;
    }
    match curated_extent(report, &files, min_nodes) {
        CuratedExtent::Short(widest) => failures.push(Failure::new(TYPE2_RECALL, format!("widest cluster spanning {files:?} has {widest} canonical nodes; expected at least {min_nodes}. Hand-verified Type-2 rename: {why}"))),
        CuratedExtent::Reached(rank) => {
            check_rank_ceiling(entry, rank, &files, why, TYPE2_RECALL, failures);
            check_pair_vouched(&files, why, verdicts, failures);
        }
    }
}

/// The admission clause: the engine must have *vouched* for the curated
/// relation, not merely produced a cluster of the right size spanning the
/// right files.
///
/// A cluster is a component; it says two files share a shape. Whether the
/// engine admitted *this pair* as a rename lives in the pair record, which
/// the mass-only wire keeps off clusters entirely ([PIPELINE-FUSED]), so
/// the gate obtains it from `deslop --compare` ([PAIR-COMPARE-CLI]).
/// Without a verdict the clause judged nothing and must fail rather than
/// pass, the stance [CORPUS-SCOPE] takes on a missing bound (gh #488).
fn check_pair_vouched(
    files: &[String],
    why: &str,
    verdicts: &[Value],
    failures: &mut Vec<Failure>,
) {
    let Some(evidence) = verdict_for(files, verdicts) else {
        failures.push(Failure::new(TYPE2_RECALL, format!("no admission evidence was obtained for {files:?}, so the clause that tells an admitted rename from a coincidental component judged nothing. Hand-verified Type-2 rename: {why}")));
        return;
    };
    let classification = evidence
        .get("classification")
        .and_then(Value::as_str)
        .unwrap_or("absent");
    let admitted = evidence.get("admitted").and_then(Value::as_bool) == Some(true);
    let content_ok = evidence.get("content_required").and_then(Value::as_bool) != Some(true)
        || evidence.get("content_ok").and_then(Value::as_bool) == Some(true);
    if !admitted || !content_ok || classification != VOUCHED_TYPE2_CLASSIFICATION {
        failures.push(Failure::new(TYPE2_RECALL, format!("the cluster spanning {files:?} is reported at the curated extent, but the engine did not vouch for the pair: admitted {admitted}, content guard satisfied {content_ok}, classification `{classification}` where `{VOUCHED_TYPE2_CLASSIFICATION}` is the only one a curated rename may reach. Hand-verified Type-2 rename: {why}")));
    }
}

/// The `evidence` object of the verdict curated for exactly `files`.
fn verdict_for<'a>(files: &[String], verdicts: &'a [Value]) -> Option<&'a Value> {
    verdicts
        .iter()
        .find(|verdict| curated_files(verdict) == files)
        .and_then(|verdict| verdict.get("evidence"))
}

/// Resolves the curated extent clause, carrying the rank of the cluster
/// that satisfies it.
///
/// The rank asserted must belong to the cluster that *is* the curated
/// duplicate. A sub-extent fragment ranking first cannot answer the
/// ceiling for the module buried behind it — gh #439 witness 2 is exactly
/// that shape, a 39-node fragment standing in for the whole-module view.
fn curated_extent(report: &Value, files: &[String], min_nodes: u64) -> CuratedExtent {
    let spanning = spanning_extents(report, files);
    spanning
        .iter()
        .filter(|(_, nodes)| *nodes >= min_nodes)
        .map(|(rank, _)| *rank)
        .min()
        .map_or_else(
            || {
                CuratedExtent::Short(
                    spanning
                        .iter()
                        .map(|(_, nodes)| *nodes)
                        .max()
                        .unwrap_or_default(),
                )
            },
            CuratedExtent::Reached,
        )
}

/// One-based report position and canonical extent of every visible
/// cluster whose shown occurrences span the curated files.
fn spanning_extents(report: &Value, files: &[String]) -> Vec<(usize, u64)> {
    visible_clusters(report)
        .into_iter()
        .enumerate()
        .filter(|(_, cluster)| cluster_shows_span(cluster, files))
        .map(|(index, cluster)| {
            (
                index.saturating_add(1),
                field_u64(cluster, CANONICAL_NODE_COUNT),
            )
        })
        .collect()
}

/// Applies an optional curated maximum rank, inclusive.
///
/// Ranking is the product: a finding a user never scrolls to is a finding
/// they do not get, so a curated pair reported past its ceiling is a
/// recall failure and not a number to print (gh #439).
fn check_rank_ceiling(
    entry: &Value,
    rank: usize,
    files: &[String],
    why: &str,
    check: &str,
    failures: &mut Vec<Failure>,
) {
    let Some(ceiling) = entry.get(CURATED_RANK_FIELD).and_then(Value::as_u64) else {
        return;
    };
    if u64::try_from(rank).unwrap_or(u64::MAX) > ceiling {
        failures.push(Failure::new(check, format!("verified duplicate spanning {files:?} ranks {rank}, past its curated ceiling of {ceiling}. Verified duplicate: {why}")));
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
