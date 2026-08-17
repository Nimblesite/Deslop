//! [CORPUS-BASELINE] The two confidence checks the real-repository gate runs
//! over a finished report: `fused_bounded_max` and `type2_recall`.
//!
//! Both exist because the synthetic fixtures that pin
//! [FUSION-STRATEGY-BOUNDED-MAX] and [FUSION-CONTENT-GATE] are built to
//! demonstrate a mechanism, not to survive a real corpus. A fixture proves the
//! gate *can* separate a proven rename from sibling scaffolding on five files
//! the author chose; it says nothing about whether the operating point holds
//! across 30,000 real ones. These two catch the failure modes that only appear
//! at that scale, and they catch them in opposite directions — one guards the
//! confidence arithmetic itself, the other guards against that arithmetic
//! swallowing the findings.
//!
//! Both are keyed on rendered report fields only, with no rank in the key, so
//! they are stable against the cluster-order churn `corpus/known-failures.json`
//! documents.

use serde_json::Value;

use crate::corpus::Failure;

/// Minimum demoted clusters before [`check_type2_recall`] will judge a report.
///
/// Below this the absence of act-now findings is ordinary — a clean repository
/// has neither population. The check fires on the *shape* of a report that
/// found plenty of same-shape families and vouched for none of them.
const TYPE2_MIN_DEMOTED: usize = 20;

/// The bucket a shape-identical cluster reaches only by the content gate
/// vouching for it — the one act-now destination [`check_type2_recall`] can
/// read as recall evidence.
///
/// `identical` is deliberately **not** here. That bucket is decided by raw
/// byte-equivalence in `report_bucket_kind`, and both `route_shape_identical`
/// and `content_gated_signals` return before touching it, so a byte-identical
/// clone is proof about the *byte comparison*, not about the gate. Counting it
/// made the check vacuous: Tokio renders 452 `identical` clusters, so every
/// Type-2 rename in the repository could regress into the demoted tier and the
/// gate would still pass.
const CONTENT_VOUCHED_BUCKET: &str = "nearly_identical";

/// Tolerance on the bounded-max invariant in [`check_fused_bounded_max`].
///
/// The rendered signals are `f64`s serialised to JSON and read back, and the
/// invariant is an inequality between values the renderer computed from each
/// other, so only representation error is being absorbed here — not slack for
/// a differently-shaped formula. `1e-6` is far below the 0.001 the report
/// itself renders at and far above the last-bit spread of a round trip.
const BOUNDED_MAX_EPSILON: f64 = 1e-6;

/// Wire bucket labels the content gate demotes a shape-identical cluster into.
const DEMOTED_BUCKETS: [&str; 2] = ["structural_only", "loosely_similar"];

/// [CORPUS-BASELINE] `fused_bounded_max` — every rendered confidence must obey
/// [FUSION-STRATEGY-BOUNDED-MAX].
///
/// This is gh #343 pinned as a formula, per cluster, rather than as a
/// distribution. `PairScore::fused` summed three correlated axes and clamped,
/// so a pair at `structural 0.00 / token 0.30 / embedding 0.94` admitted at
/// `1.00` — indistinguishable from a byte-proven verbatim copy. The shipped
/// arithmetic is `bounded_fused` = the strongest single axis, and every path
/// that rewrites the confidence downstream only ever scales it *down*:
///
/// - an ungated cluster keeps the pair's `bounded_fused`, which is the max;
/// - `content_gated_signals` renders `max(embedding, max(structural, token) ×
///   content_confidence)` with `content_confidence ∈ [0,1]`;
/// - a byte-proven `Identical` cluster renders `1.0` with `token_jaccard` also
///   corrected to `1.0`, so the max is `1.0` too.
///
/// So `fused ≤ max(structural, token_jaccard, embedding_cos)` holds for every
/// cluster the engine can legitimately render, and `min(1, s + t + e)` breaks
/// it the moment any two axes are positive. That makes this an exact contract
/// with no operating point in it: it cannot churn when ranking moves, it
/// cannot be rescued by one healthy outlier in a population of saturated
/// clusters, and it cannot fire on a repository whose clusters legitimately
/// share one confidence.
///
/// It replaces an earlier `fused_spread` check that asked whether the
/// population took more than one distinct value. That predicate was unsound in
/// both directions — a single outlier cleared it however many clusters were
/// saturated, and a repository of genuinely byte-identical clones failed it —
/// and on the scheduled corpora it never reached the arithmetic at all, since
/// those scans run embeddings-off where every cluster is either byte-identical
/// or content-gated and the incoming pair `fused` is discarded at render.
pub fn check_fused_bounded_max(report: &Value, failures: &mut Vec<Failure>) {
    let clusters = visible_clusters(report);
    let breaches: Vec<String> = clusters
        .iter()
        .filter_map(|cluster| bounded_max_breach(cluster))
        .collect();
    let Some(first) = breaches.first() else {
        return;
    };
    failures.push(Failure::new(
        "fused_bounded_max",
        format!(
            "{} of {} visible clusters render a confidence above the strongest axis they were \
             computed from, which [FUSION-STRATEGY-BOUNDED-MAX] forbids — the first is {first} \
             (gh #343: the sum-then-clamp arm renders exactly this)",
            breaches.len(),
            clusters.len(),
        ),
    ));
}

/// Describes how one cluster breaks the bounded-max invariant, or `None`.
fn bounded_max_breach(cluster: &Value) -> Option<String> {
    let structural = signal(cluster, "structural");
    let token = signal(cluster, "token_jaccard");
    let embedding = signal(cluster, "embedding_cos");
    let fused = signal(cluster, "fused");
    let strongest = structural.max(token).max(embedding);
    (fused > strongest + BOUNDED_MAX_EPSILON).then(|| {
        format!(
            "bucket {} at structural {structural:.3} / token {token:.3} / embedding \
             {embedding:.3} rendering fused {fused:.3}, over the {strongest:.3} ceiling",
            cluster
                .get("bucket")
                .and_then(Value::as_str)
                .unwrap_or("<unlabelled>"),
        )
    })
}

/// [CORPUS-BASELINE] `type2_recall` — the content gate must vouch for
/// *something*.
///
/// The recall side of [FUSION-CONTENT-GATE], and the direction a
/// false-positive check structurally cannot see. `lacks_content_support`
/// demotes a shape-identical cluster whose content evidence is absent; set the
/// operating point slightly too high and every genuine Type-2 rename in a real
/// repository sinks into the demoted tier with the scaffolding it was built to
/// separate. The report stays plausible — it is full of clusters — while every
/// finding a user would act on has quietly become "verify before extracting".
///
/// A repository that produced a large demoted population and *zero*
/// gate-vouched clusters is that failure. It is not a threshold on a score: a
/// real corpus of that size containing no proven rename at all is not a state
/// the engine should ever report.
///
/// Only [`CONTENT_VOUCHED_BUCKET`] counts as recall. A `identical` cluster is
/// decided by byte-equivalence before the gate runs, so it is evidence the
/// byte comparison works and says nothing about whether the gate is vouching
/// for anything — see that constant for what counting it cost.
pub fn check_type2_recall(report: &Value, failures: &mut Vec<Failure>) {
    let clusters = visible_clusters(report);
    let demoted = clusters
        .iter()
        .filter(|c| in_set(c, &DEMOTED_BUCKETS))
        .count();
    let vouched = clusters
        .iter()
        .filter(|c| in_set(c, &[CONTENT_VOUCHED_BUCKET]))
        .count();
    if demoted < TYPE2_MIN_DEMOTED || vouched > 0 {
        return;
    }
    let proven = clusters.iter().filter(|c| in_set(c, &["identical"])).count();
    failures.push(Failure::new(
        "type2_recall",
        format!(
            "{demoted} same-shape clusters were demoted and not one reached \
             `{CONTENT_VOUCHED_BUCKET}` — the content gate vouched for nothing in the whole \
             repository, so every genuine rename is being reported as unverified scaffolding \
             (the {proven} byte-identical clusters here are decided before the gate runs and \
             cannot stand in for it)",
        ),
    ));
}

/// Clusters the report actually shows a user. A hidden cluster carries no
/// claim, so it can neither collapse the spread nor rescue recall.
fn visible_clusters(report: &Value) -> Vec<&Value> {
    match report.get("clusters").and_then(Value::as_array) {
        None => Vec::new(),
        Some(clusters) => clusters
            .iter()
            .filter(|cluster| !all_occurrences_hidden(cluster))
            .collect(),
    }
}

/// True when every occurrence of a cluster is hidden, so nothing is rendered.
fn all_occurrences_hidden(cluster: &Value) -> bool {
    match cluster.get("occurrences").and_then(Value::as_array) {
        None => true,
        Some(occurrences) => occurrences
            .iter()
            .all(|occurrence| occurrence.get("hidden").and_then(Value::as_bool) == Some(true)),
    }
}

/// True when the cluster's wire bucket is one of `buckets`.
fn in_set(cluster: &Value, buckets: &[&str]) -> bool {
    cluster
        .get("bucket")
        .and_then(Value::as_str)
        .is_some_and(|bucket| buckets.contains(&bucket))
}

/// Reads one signal off a cluster, defaulting to zero when absent.
fn signal(cluster: &Value, name: &str) -> f64 {
    cluster
        .get("signals")
        .and_then(|signals| signals.get(name))
        .and_then(Value::as_f64)
        .unwrap_or_default()
}



#[cfg(test)]
mod tests {
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
    fn with_embedding(
        bucket: &str,
        structural: f64,
        token: f64,
        embedding: f64,
        fused: f64,
    ) -> Value {
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
                &report(&[with_embedding(bucket, structural, token, embedding, reverted)]),
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
        check_fused_bounded_max(&report(&[with_embedding("x", 0.2, 0.3, 0.4, 0.9)]), &mut failures);
        assert_eq!(failures.len(), 1, "one cluster is enough to be wrong");
        assert!(detail_mentions(&failures, "1 of 1 visible clusters"), "{failures:?}");
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
        assert!(failures.is_empty(), "last-bit noise is not a defect: {failures:?}");

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
}
