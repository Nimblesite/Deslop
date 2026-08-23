//! Unit tests for [`super`] — the `fused_bounded_max`,
//! `type2_gate_liveness` and curated `type2_recall` corpus confidence
//! checks ([CORPUS-BASELINE], [CORPUS-RECALL]).

use super::*;
use serde_json::json;

/// The single reported failure, or `None` when there is not exactly one.
/// Returning an `Option` keeps the assertions in the tests, where their
/// messages can name the report that produced them.
fn only(failures: &[Failure]) -> Option<&Failure> {
    match failures {
        [single] => Some(single),
        _ => None,
    }
}

/// The check id of the single reported failure, for direct comparison.
fn only_check(failures: &[Failure]) -> Option<&str> {
    only(failures).map(|failure| failure.check.as_str())
}

/// True when the single reported failure's detail contains `needle`.
fn detail_mentions(failures: &[Failure], needle: &str) -> bool {
    only(failures).is_some_and(|failure| failure.detail.contains(needle))
}

/// Asserts `failures` is exactly one failure of `check` whose detail
/// mentions `needle`. The curated, liveness and fused suites each
/// respelled this same triple; Deslop scored the copies against this
/// repo's own corpus. Both messages stay per-call so no suite loses the
/// sentence that says what its case is actually proving.
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

/// One cluster with the given bucket, signal triple and rendered fused.
fn cluster(bucket: &str, structural: f64, token: f64, fused: f64) -> Value {
    with_embedding(bucket, structural, token, 0.0, fused)
}

/// The same, with the semantic axis set too.
fn with_embedding(bucket: &str, structural: f64, token: f64, embedding: f64, fused: f64) -> Value {
    json!({
        "bucket": bucket,
        "signals": {
            "structural": structural,
            "token_jaccard": token,
            "embedding_cos": embedding,
            "fused": fused,
        },
        "occurrences": [{ "hidden": false }, { "hidden": false }],
    })
}

/// The shipped arithmetic: the strongest single axis, bounded.
fn bounded_max(structural: f64, token: f64, embedding: f64) -> f64 {
    structural.max(token).max(embedding).clamp(0.0, 1.0)
}

/// The quarantined arithmetic from gh #343, kept here as a negative
/// control. A gate that never fails against the code it was written to
/// catch asserts nothing, so every `fused_bounded_max` test that expects a
/// pass is re-run through this to prove it would have caught the revert.
fn sum_then_clamp(structural: f64, token: f64, embedding: f64) -> f64 {
    (structural + token + embedding).clamp(0.0, 1.0)
}

/// The same cluster with every occurrence hidden, so it renders nothing.
fn hide(mut cluster: Value) -> Value {
    if let Some(entry) = cluster.get_mut("occurrences") {
        *entry = json!([{ "hidden": true }, { "hidden": true }]);
    }
    cluster
}

fn report(clusters: &[Value]) -> Value {
    json!({ "clusters": clusters })
}

/// The signal triples the negative control is run over. Each is a shape
/// the engine really renders, and each has at least two positive axes so
/// the sum and the max provably disagree.
const TRIPLES: [(&str, f64, f64, f64); 5] = [
    ("identical", 1.0, 1.0, 0.0),
    ("nearly_identical", 1.0, 1.0, 0.42),
    ("structural_only", 1.0, 0.30, 0.0),
    ("loosely_similar", 0.20, 0.30, 0.94),
    ("same_behavior", 0.10, 0.20, 0.88),
];

/// Smallest cluster that can credibly *be* the whole-module rename the
/// curated entries in these tests describe, in `canonical_node_count`.
///
/// A floor, not a pin: the legitimate whole-module view of one curated
/// pair measured 348 and 395 nodes on two different builds of the same
/// pinned tokio commit, so the extent a correct engine reports moves.
/// What does not move is the two orders of magnitude between that view
/// and the fragments gh #439 shows satisfying the check —
/// [`curated::ACCESSOR_NODES`] and [`curated::FRAGMENT_NODES`]. No
/// operating point is being tuned here; any value in the wide gap
/// separates them.
const CURATED_MIN_NODES: u64 = 300;

/// The whole-module view of a curated pair, as tokio renders it today.
const MODULE_NODES: u64 = 348;

/// One cluster whose occurrences carry the given rendered paths, at the
/// extent a credible whole-module rename carries. [`sized`] overrides it.
fn spanning(bucket: &str, structural: f64, token: f64, files: &[&str]) -> Value {
    let occurrences: Vec<Value> = files
        .iter()
        .map(|file| json!({ "path": file, "hidden": false }))
        .collect();
    json!({
        "bucket": bucket,
        "canonical_node_count": MODULE_NODES,
        "signals": {
            "structural": structural,
            "token_jaccard": token,
            "embedding_cos": 0.0,
            "fused": token.max(structural),
        },
        "occurrences": occurrences,
    })
}

/// The same cluster reported at `nodes` instead, so a case can vary the
/// extent alone and leave every other rendered field identical.
fn sized(mut cluster: Value, nodes: u64) -> Value {
    cluster["canonical_node_count"] = json!(nodes);
    cluster
}

/// A manifest curating one hand-verified Type-2 pair, with the extent
/// [CORPUS-RECALL] requires an entry to curate.
fn manifest_with_type2(files: &[&str]) -> Value {
    json!({
        "must_find_type2": [{
            "files": files,
            "min_nodes": CURATED_MIN_NODES,
            "why": "hand-verified rename pair for the unit test",
        }]
    })
}

mod curated;
mod fused;
mod liveness;
mod recall;
