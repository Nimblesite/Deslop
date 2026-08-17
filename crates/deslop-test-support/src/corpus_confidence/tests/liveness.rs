//! [CORPUS-RECALL] `type2_gate_liveness` — a report that vouches for no
//! same-shape family while demoting many is a gate that stopped judging.

use super::*;

/// Runs the liveness check over `demoted` same-shape demotions plus whatever
/// the case under test adds, and returns what it reported.
fn judge_population(demoted: usize, extra: Vec<Value>) -> Vec<Failure> {
    let mut clusters: Vec<Value> = (0..demoted)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    clusters.extend(extra);
    let mut failures = Vec::new();
    check_type2_gate_liveness(&report(&clusters), &mut failures);
    failures
}

/// The population every case below starts from: enough demoted same-shape
/// families that the check judges the report at all.
fn judge(extra: Vec<Value>) -> Vec<Failure> {
    judge_population(TYPE2_MIN_DEMOTED, extra)
}

#[test]
fn a_report_that_vouches_for_nothing_is_reported() {
    let failures = judge(Vec::new());
    assert_eq!(failures.len(), 1, "total demotion must be reported");
    assert_eq!(only_check(&failures), Some("type2_gate_liveness"));
    assert!(
        detail_mentions(&failures, "nearly_identical"),
        "the detail must name the bucket that stayed empty: {failures:?}",
    );
}

#[test]
fn one_gate_vouched_cluster_clears_the_recall_check() {
    let failures = judge(vec![cluster("nearly_identical", 1.0, 1.0, 0.9)]);
    assert!(
        failures.is_empty(),
        "the check asks whether the gate vouched for anything, not how much: {failures:?}"
    );
}

#[test]
fn byte_identical_clones_cannot_stand_in_for_type2_recall() {
    // The vacuity this check shipped with. `identical` is decided by byte
    // equivalence before `route_shape_identical` or
    // `content_gated_signals` run, so however many of them a repository
    // reports, none is evidence the content gate vouched for a rename.
    // Tokio renders 452; counting them meant every Type-2 rename in the
    // repository could regress into the demoted tier with the gate green.
    let failures = judge(
        (0..452)
            .map(|_| cluster("identical", 1.0, 1.0, 1.0))
            .collect(),
    );
    assert_eq!(
        failures.len(),
        1,
        "452 byte-proven clones must not rescue a repository that vouched for no rename"
    );
    assert_eq!(only_check(&failures), Some("type2_gate_liveness"));
    assert!(
        detail_mentions(&failures, "452 byte-identical clusters"),
        "and the detail must say why they did not count: {failures:?}",
    );
}

#[test]
fn a_small_demoted_population_is_not_judged_on_recall() {
    let failures = judge_population(TYPE2_MIN_DEMOTED - 1, Vec::new());
    assert!(
        failures.is_empty(),
        "a clean repository has neither population"
    );
}

#[test]
fn a_hidden_act_now_cluster_does_not_rescue_recall() {
    // A cluster the renderer hid was never offered to the user, so it
    // cannot stand as evidence that the gate vouched for something.
    let failures = judge(vec![hide(cluster("nearly_identical", 1.0, 1.0, 0.9))]);
    assert_eq!(failures.len(), 1, "a hidden rescue is no rescue");
}

#[test]
fn a_sub_floor_token_near_miss_cannot_stand_in_for_the_gate() {
    // The false green this check shipped with: the token-LSH Type-3 path
    // can classify a cluster `nearly_identical` from `structural = 0`,
    // `embedding = 0` and a token Jaccard below the saturating floor —
    // `content_gated_signals` returned without ever judging it. One such
    // unrelated near miss kept the old check green while every genuine
    // rename sank into the demoted tier.
    let failures = judge(vec![cluster("nearly_identical", 0.0, 0.93, 0.93)]);
    assert_eq!(
        failures.len(),
        1,
        "a near miss the gate never judged must not clear the liveness check: {failures:?}"
    );
    assert_eq!(only_check(&failures), Some("type2_gate_liveness"));
}

#[test]
fn a_token_saturated_cluster_is_gate_evidence() {
    // The gate's other precondition: a kind-stream Jaccard at or above the
    // saturating floor is shape evidence, the gate judged the cluster, and
    // its surviving `nearly_identical` verdict is a vouch even with
    // `structural` unsaturated (a mixed LSH-glued cluster keeps its
    // estimated structural value).
    let failures = judge(vec![cluster("nearly_identical", 0.62, 0.98, 0.9)]);
    assert!(
        failures.is_empty(),
        "a token-saturated survivor is a gate vouch: {failures:?}"
    );
}
