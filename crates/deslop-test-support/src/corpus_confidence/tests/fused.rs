//! [CORPUS-BASELINE] `fused_bounded_max` — the rendered fused score is
//! the strongest single axis, never a sum. Every passing case is re-run
//! through a negative control so the gate cannot pass while blind.

use super::*;

/// The failures [`check_fused_bounded_max`] reports for `clusters`.
/// Seven cases respelled the same build-vec / call / pass-by-mut-ref
/// preamble; Deslop scored the copies against this repo's own corpus.
fn judge_fused(clusters: &[Value]) -> Vec<Failure> {
    let mut failures = Vec::new();
    check_fused_bounded_max(&report(clusters), &mut failures);
    failures
}

#[test]
fn the_shipped_arithmetic_passes_and_the_quarantined_one_fails() {
    // The negative control this gate exists for. Every triple is rendered
    // twice — once through `bounded_fused`, once through the gh #343
    // sum-then-clamp arm — and the check must separate them.
    let shipped: Vec<Value> = TRIPLES
        .iter()
        .map(|&(bucket, s, t, e)| with_embedding(bucket, s, t, e, bounded_max(s, t, e)))
        .collect();
    let failures = judge_fused(&shipped);
    assert!(
        failures.is_empty(),
        "the shipped bounded max must never trip its own gate: {failures:?}"
    );

    let reverted: Vec<Value> = TRIPLES
        .iter()
        .map(|&(bucket, s, t, e)| with_embedding(bucket, s, t, e, sum_then_clamp(s, t, e)))
        .collect();
    let failures = judge_fused(&reverted);
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
    let failures = judge_fused(&clusters);
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
    let failures = judge_fused(&clusters);
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
    let failures = judge_fused(&clusters);
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
    let failures = judge_fused(&clusters);
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
    let failures = judge_fused(&clusters);
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
