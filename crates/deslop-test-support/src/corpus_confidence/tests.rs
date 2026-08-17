//! Unit tests for [`super`] — the `fused_bounded_max` and
//! `type2_recall` corpus confidence checks ([CORPUS-BASELINE]).

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

#[test]
fn the_shipped_arithmetic_passes_and_the_quarantined_one_fails() {
    // The negative control this gate exists for. Every triple is rendered
    // twice — once through `bounded_fused`, once through the gh #343
    // sum-then-clamp arm — and the check must separate them.
    let shipped: Vec<Value> = TRIPLES
        .iter()
        .map(|&(bucket, s, t, e)| with_embedding(bucket, s, t, e, bounded_max(s, t, e)))
        .collect();
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(&shipped), &mut failures);
    assert!(
        failures.is_empty(),
        "the shipped bounded max must never trip its own gate: {failures:?}"
    );

    let reverted: Vec<Value> = TRIPLES
        .iter()
        .map(|&(bucket, s, t, e)| with_embedding(bucket, s, t, e, sum_then_clamp(s, t, e)))
        .collect();
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(&reverted), &mut failures);
    assert_eq!(
        failures.len(),
        1,
        "restoring the sum must be caught: {failures:?}"
    );
    assert_eq!(only_check(&failures), Some("fused_bounded_max"));
    assert!(
        detail_mentions(&failures, "2 of 5 visible clusters"),
        "the two mid-band triples breach; the three whose strongest axis is already 1.0 are \
         hidden by the clamp: {failures:?}",
    );
    assert!(
        detail_mentions(&failures, "loosely_similar"),
        "the detail must name a breaching cluster's bucket: {failures:?}",
    );
}

#[test]
fn a_saturated_axis_hides_the_reverted_sum_from_this_gate() {
    // The limit of what a *rendered-report* check can see, stated as an
    // assertion rather than left for someone to rediscover. Where the
    // strongest axis is already 1.0 the clamp makes sum and max agree
    // exactly, so no invariant over rendered signals can separate them.
    // Detecting the revert on those clusters needs a pin at the
    // *admission* layer, on a pair whose axes all sit below the threshold
    // while their sum clears it — which is the calibration this gate does
    // not replace.
    for &(bucket, structural, token, embedding) in &TRIPLES {
        let strongest = bounded_max(structural, token, embedding);
        let reverted = sum_then_clamp(structural, token, embedding);
        let mut failures = Vec::new();
        check_fused_bounded_max(
            &report(&[with_embedding(
                bucket, structural, token, embedding, reverted,
            )]),
            &mut failures,
        );
        let visible = (reverted - strongest) > BOUNDED_MAX_EPSILON;
        assert_eq!(
            failures.len(),
            usize::from(visible),
            "{bucket}: strongest axis {strongest:.3}, reverted render {reverted:.3}",
        );
        assert_eq!(
            strongest >= 1.0,
            !visible,
            "{bucket}: a saturated strongest axis is exactly the blind spot",
        );
    }
}

#[test]
fn one_saturated_cluster_is_reported_however_healthy_the_rest_are() {
    // The false negative the old distribution predicate carried: it asked
    // whether the population took more than one value, so a single honest
    // outlier cleared a report of otherwise saturated clusters.
    let mut clusters: Vec<Value> = (0..40)
        .map(|index| {
            let step = f64::from(u32::try_from(index).unwrap_or(0)) / 100.0;
            cluster("nearly_identical", 0.2 + step, 0.3 + step, 0.3 + step)
        })
        .collect();
    clusters.push(with_embedding("loosely_similar", 0.2, 0.3, 0.4, 0.9));
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(&clusters), &mut failures);
    assert_eq!(failures.len(), 1, "one breach in 41 is still a breach");
    assert!(
        detail_mentions(&failures, "1 of 41 visible clusters"),
        "and the count must be honest about how many: {failures:?}",
    );
}

#[test]
fn distinct_triples_sharing_one_legitimate_max_pass() {
    // The false positive the old predicate carried: bounded max is
    // many-to-one on purpose. These twelve clusters differ on the axes
    // that are not dominant and legitimately render one confidence.
    let clusters: Vec<Value> = (0..12)
        .map(|index| {
            let step = f64::from(u32::try_from(index).unwrap_or(0)) / 100.0;
            with_embedding("nearly_identical", 0.9, 0.1 + step, 0.2 + step, 0.9)
        })
        .collect();
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(&clusters), &mut failures);
    assert!(
        failures.is_empty(),
        "one dominant axis across differing triples is the formula working, not a defect: \
         {failures:?}"
    );
}

#[test]
fn a_report_of_byte_identical_clones_passes() {
    // Twelve byte-identical pairs really do all score 1.0, and 1.0 is
    // exactly their strongest axis.
    let clusters: Vec<Value> = (0..12)
        .map(|_| cluster("identical", 1.0, 1.0, 1.0))
        .collect();
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(&clusters), &mut failures);
    assert!(
        failures.is_empty(),
        "identical inputs may share an output: {failures:?}"
    );
}

#[test]
fn a_content_gated_confidence_below_its_ceiling_passes() {
    // The shape [FUSION-CONTENT-GATE] renders: a proven rename scaled
    // *down* off a saturated shape signal. The gate is an inequality, so
    // scaling down can never trip it — which is what lets a proven rename
    // legitimately render below the admission bar.
    let clusters = [
        cluster("nearly_identical", 1.0, 1.0, 0.62),
        cluster("structural_only", 1.0, 0.0, 0.0),
        with_embedding("same_behavior", 0.1, 0.2, 0.88, 0.88),
    ];
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(&clusters), &mut failures);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn a_single_cluster_report_is_still_judged() {
    // No population floor: the invariant is per cluster, so there is no
    // size below which a breach stops being one. The old spread check
    // needed a floor of ten and could say nothing about smaller repos.
    let mut failures = Vec::new();
    check_fused_bounded_max(
        &report(&[with_embedding("x", 0.2, 0.3, 0.4, 0.9)]),
        &mut failures,
    );
    assert_eq!(failures.len(), 1, "one cluster is enough to be wrong");
    assert!(
        detail_mentions(&failures, "1 of 1 visible clusters"),
        "{failures:?}"
    );
}

#[test]
fn hidden_clusters_cannot_breach_the_invariant() {
    let clusters: Vec<Value> = (0..12)
        .map(|index| {
            let step = f64::from(u32::try_from(index).unwrap_or(0)) / 100.0;
            hide(cluster("nearly_identical", 0.2 + step, 0.3 + step, 1.0))
        })
        .collect();
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(&clusters), &mut failures);
    assert!(
        failures.is_empty(),
        "a hidden cluster makes no claim to the user, so it cannot fail a claim check"
    );
}

#[test]
fn float_round_trip_noise_is_not_a_breach() {
    // The invariant must absorb representation error and nothing wider.
    let mut failures = Vec::new();
    check_fused_bounded_max(
        &report(&[cluster("nearly_identical", 0.7, 0.3, 0.7 + 1e-9)]),
        &mut failures,
    );
    assert!(
        failures.is_empty(),
        "last-bit noise is not a defect: {failures:?}"
    );

    check_fused_bounded_max(
        &report(&[cluster("nearly_identical", 0.7, 0.3, 0.7 + 1e-3)]),
        &mut failures,
    );
    assert_eq!(failures.len(), 1, "a breach at report precision is one");
}

#[test]
fn a_report_that_vouches_for_nothing_is_reported() {
    let clusters: Vec<Value> = (0..TYPE2_MIN_DEMOTED)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    let mut failures = Vec::new();
    check_type2_recall(&report(&clusters), &mut failures);
    assert_eq!(failures.len(), 1, "total demotion must be reported");
    assert_eq!(only_check(&failures), Some("type2_recall"));
    assert!(
        detail_mentions(&failures, "nearly_identical"),
        "the detail must name the bucket that stayed empty: {failures:?}",
    );
}

#[test]
fn one_gate_vouched_cluster_clears_the_recall_check() {
    let mut clusters: Vec<Value> = (0..TYPE2_MIN_DEMOTED)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    clusters.push(cluster("nearly_identical", 1.0, 1.0, 0.9));
    let mut failures = Vec::new();
    check_type2_recall(&report(&clusters), &mut failures);
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
    let mut clusters: Vec<Value> = (0..TYPE2_MIN_DEMOTED)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    clusters.extend((0..452).map(|_| cluster("identical", 1.0, 1.0, 1.0)));
    let mut failures = Vec::new();
    check_type2_recall(&report(&clusters), &mut failures);
    assert_eq!(
        failures.len(),
        1,
        "452 byte-proven clones must not rescue a repository that vouched for no rename"
    );
    assert_eq!(only_check(&failures), Some("type2_recall"));
    assert!(
        detail_mentions(&failures, "452 byte-identical clusters"),
        "and the detail must say why they did not count: {failures:?}",
    );
}

#[test]
fn a_small_demoted_population_is_not_judged_on_recall() {
    let clusters: Vec<Value> = (0..TYPE2_MIN_DEMOTED - 1)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    let mut failures = Vec::new();
    check_type2_recall(&report(&clusters), &mut failures);
    assert!(
        failures.is_empty(),
        "a clean repository has neither population"
    );
}

#[test]
fn a_hidden_act_now_cluster_does_not_rescue_recall() {
    // A cluster the renderer hid was never offered to the user, so it
    // cannot stand as evidence that the gate vouched for something.
    let mut clusters: Vec<Value> = (0..TYPE2_MIN_DEMOTED)
        .map(|_| cluster("structural_only", 1.0, 0.3, 0.31))
        .collect();
    clusters.push(hide(cluster("nearly_identical", 1.0, 1.0, 0.9)));
    let mut failures = Vec::new();
    check_type2_recall(&report(&clusters), &mut failures);
    assert_eq!(failures.len(), 1, "a hidden rescue is no rescue");
}
