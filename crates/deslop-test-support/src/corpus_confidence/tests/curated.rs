//! [CORPUS-RECALL] curated `type2_recall` — every hand-verified rename in
//! a manifest's `must_find_type2` list must be reported, shown and vouched.

use super::*;

/// The pair every case below curates.
const PAIR: [&str; 2] = ["src/a.ts", "src/b.ts"];

/// Judges the curated `PAIR` against a report of `clusters`.
fn judge(clusters: &[Value]) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_type2_curated_recall(
        &manifest_with_type2(&PAIR),
        &report(clusters),
        &mut failures,
    );
    failures
}

/// Judges the curated `PAIR` against a report of one cluster that spans
/// `files` with the given bucket and signals.
fn judge_spanning(bucket: &str, structural: f64, token: f64, files: &[&str]) -> Vec<Failure> {
    judge(&[spanning(bucket, structural, token, files)])
}

#[test]
fn a_curated_type2_pair_reported_and_vouched_passes() {
    let failures = judge_spanning("nearly_identical", 1.0, 1.0, &PAIR);
    assert!(
        failures.is_empty(),
        "a visible gate-vouched cluster spanning the curated pair is recall: {failures:?}"
    );
}

#[test]
fn a_missing_curated_type2_pair_is_a_false_negative() {
    let elsewhere = ["src/other.ts", "src/else.ts"];
    let failures = judge_spanning("nearly_identical", 1.0, 1.0, &elsewhere);
    assert_only_failure(
        &failures,
        "type2_recall",
        "an unreported curated pair must fail",
        "src/a.ts",
        "the detail names the missed pair",
    );
}

#[test]
fn a_curated_pair_found_but_demoted_is_a_gate_failure() {
    // The cluster exists and spans the pair, but the gate demoted it — the
    // user is told to verify scaffolding instead of acting on a proven
    // rename. Recall is about what the report *claims*, not what it lists.
    let failures = judge_spanning("structural_only", 1.0, 0.3, &PAIR);
    assert_only_failure(
        &failures,
        "type2_recall",
        "a demoted curated pair must fail",
        "demoted",
        "the detail says the gate failed to vouch",
    );
}

#[test]
fn a_curated_pair_in_a_sub_floor_near_miss_is_not_vouched_recall() {
    // Spanning the pair in a `nearly_identical` cluster the gate never
    // judged (sub-floor token, unsaturated structural) is the same false
    // green the liveness check rejects, scoped to one entry.
    let failures = judge_spanning("nearly_identical", 0.0, 0.93, &PAIR);
    assert_eq!(failures.len(), 1, "an unvouched curated pair must fail");
    assert_eq!(only_check(&failures), Some("type2_recall"));
}

#[test]
fn a_curated_pair_whose_own_occurrence_is_hidden_is_not_recall() {
    // The cluster renders — a sibling occurrence is shown — so every
    // cluster-level visibility test passes. But the curated file's own side
    // is suppressed, so the user never sees the pair the entry proves is
    // there. Recall is what the report *shows*, not what it contains.
    let failures = judge(&[with_hidden(&["src/a.ts", "src/c.ts"], &["src/b.ts"])]);
    assert_only_failure(
        &failures,
        "type2_recall",
        "a suppressed curated side must fail",
        "hidden",
        "the detail must name suppression as a cause",
    );
}

/// A gate-vouched cluster whose occurrences are split between shown and
/// hidden, for the one case cluster-level visibility cannot express.
fn with_hidden(shown: &[&str], hidden: &[&str]) -> Value {
    let occurrences: Vec<Value> = shown
        .iter()
        .map(|file| json!({ "path": file, "hidden": false }))
        .chain(
            hidden
                .iter()
                .map(|file| json!({ "path": file, "hidden": true })),
        )
        .collect();
    json!({
        "bucket": "nearly_identical",
        "signals": { "structural": 1.0, "token_jaccard": 1.0, "embedding_cos": 0.0, "fused": 1.0 },
        "occurrences": occurrences,
    })
}

#[test]
fn an_empty_curated_list_asserts_nothing() {
    let clusters: Vec<Value> = (0..TYPE2_MIN_DEMOTED)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    let mut failures = Vec::new();
    check_type2_curated_recall(&json!({}), &report(&clusters), &mut failures);
    check_type2_curated_recall(
        &json!({ "must_find_type2": [] }),
        &report(&clusters),
        &mut failures,
    );
    assert!(
        failures.is_empty(),
        "no curated entries means no recall assertion, pass or fail: {failures:?}"
    );
}
