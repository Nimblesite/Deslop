//! [CORPUS-BASELINE] The two confidence checks the real-repository gate runs
//! over a finished report: `fused_spread` and `type2_recall`.
//!
//! Both exist because the synthetic fixtures that pin
//! [FUSION-STRATEGY-BOUNDED-MAX] and [FUSION-CONTENT-GATE] are built to
//! demonstrate a mechanism, not to survive a real corpus. A fixture proves the
//! gate *can* separate a proven rename from sibling scaffolding on five files
//! the author chose; it says nothing about whether the operating point holds
//! across 30,000 real ones. These two catch the failure modes that only appear
//! at that scale, and they catch them in opposite directions — one guards
//! against the confidence axis carrying no information, the other against it
//! carrying so much that it swallows the findings.
//!
//! Both are keyed on rendered report fields only, with no rank in the key, so
//! they are stable against the cluster-order churn `corpus/known-failures.json`
//! documents.

use serde_json::Value;

use crate::corpus::Failure;

/// Minimum visible clusters before [`check_fused_spread`] will judge a report.
///
/// A repository that reports two clusters can legitimately render one
/// confidence for both — two byte-identical pairs really do both score 1.0.
/// Collapse is only evidence of a broken axis once there is enough population
/// for the axis to have had something to say.
const SPREAD_MIN_CLUSTERS: usize = 10;

/// Minimum demoted clusters before [`check_type2_recall`] will judge a report.
///
/// Below this the absence of act-now findings is ordinary — a clean repository
/// has neither population. The check fires on the *shape* of a report that
/// found plenty of same-shape families and vouched for none of them.
const TYPE2_MIN_DEMOTED: usize = 20;

/// Wire bucket labels the engine considers actionable.
const ACT_NOW_BUCKETS: [&str; 2] = ["identical", "nearly_identical"];

/// Wire bucket labels the content gate demotes a shape-identical cluster into.
const DEMOTED_BUCKETS: [&str; 2] = ["structural_only", "loosely_similar"];

/// [CORPUS-BASELINE] `fused_spread` — the rendered confidence must distinguish
/// clusters that carry visibly different evidence.
///
/// This is gh #343 caught at scale. `PairScore::fused` summed three correlated
/// axes and clamped, so every cluster with any two signals above 0.5 rendered
/// `fused = 1.000`: a mid-band cluster at `structural 0.00 / token 0.30 /
/// embedding 0.94` was indistinguishable from a byte-proven verbatim copy. The
/// synthetic fixture that pins it uses one hand-built corpus; the same
/// saturation on a real repository shows up as thousands of clusters sharing
/// one confidence, which no fixture can demonstrate.
///
/// The predicate is deliberately weak — *more than one* distinct value across
/// clusters whose raw signal triples differ. A strong spread requirement would
/// encode an operating point and churn every time ranking moves; a collapse to
/// a single value cannot be anything but a broken axis, because the inputs it
/// was computed from provably differed.
pub fn check_fused_spread(report: &Value, failures: &mut Vec<Failure>) {
    let clusters = visible_clusters(report);
    if clusters.len() < SPREAD_MIN_CLUSTERS {
        return;
    }
    let fused: Vec<u64> = clusters
        .iter()
        .map(|cluster| rounded_bits(signal(cluster, "fused")))
        .collect();
    let Some(collapsed) = single_value(&fused) else {
        return;
    };
    let triples: Vec<[u64; 3]> = clusters
        .iter()
        .map(|cluster| {
            [
                rounded_bits(signal(cluster, "structural")),
                rounded_bits(signal(cluster, "token_jaccard")),
                rounded_bits(signal(cluster, "embedding_cos")),
            ]
        })
        .collect();
    let shapes = distinct(&triples);
    if shapes < 2 {
        return;
    }
    failures.push(Failure::new(
        "fused_spread",
        format!(
            "all {} visible clusters render fused = {:.3} while their signal triples take {shapes} \
             distinct values — the confidence axis is carrying no information (gh #343)",
            clusters.len(),
            f64::from_bits(collapsed),
        ),
    ));
}

/// The single value every entry takes, or `None` when they differ or the
/// slice is empty.
fn single_value(values: &[u64]) -> Option<u64> {
    let mut entries = values.iter();
    let first = *entries.next()?;
    entries.all(|value| *value == first).then_some(first)
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
/// A repository that produced a large demoted population and *zero* act-now
/// clusters is that failure. It is not a threshold on a score: a real corpus of
/// that size containing no verbatim copy and no proven rename at all is not a
/// state the engine should ever report.
pub fn check_type2_recall(report: &Value, failures: &mut Vec<Failure>) {
    let clusters = visible_clusters(report);
    let demoted = clusters
        .iter()
        .filter(|c| in_set(c, &DEMOTED_BUCKETS))
        .count();
    let act_now = clusters
        .iter()
        .filter(|c| in_set(c, &ACT_NOW_BUCKETS))
        .count();
    if demoted < TYPE2_MIN_DEMOTED || act_now > 0 {
        return;
    }
    failures.push(Failure::new(
        "type2_recall",
        format!(
            "{demoted} same-shape clusters were demoted and not one reached an act-now bucket \
             ({}) — the content gate vouched for nothing in the whole repository, so every \
             genuine rename is being reported as unverified scaffolding",
            ACT_NOW_BUCKETS.join(" / "),
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

/// A signal rounded to three decimals, as bits, so equality is exact.
///
/// Comparing rendered `f64`s directly would make these checks a float-equality
/// test on values that legitimately differ in the last bit. Three decimals is
/// the precision the report itself renders at, and `to_bits` keeps the result
/// a total-equality key without a lossy numeric cast — `f64::from_bits` turns
/// it straight back into the value to print.
fn rounded_bits(value: f64) -> u64 {
    ((value.clamp(0.0, 1.0) * 1000.0).round() / 1000.0).to_bits()
}

/// Number of distinct entries, without requiring `Ord` on the element type.
fn distinct<T: PartialEq>(values: &[T]) -> usize {
    values
        .iter()
        .enumerate()
        .filter(|(index, value)| !values.iter().take(*index).any(|seen| seen == *value))
        .count()
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
        json!({
            "bucket": bucket,
            "signals": {
                "structural": structural,
                "token_jaccard": token,
                "embedding_cos": 0.0,
                "fused": fused,
            },
            "occurrences": [{ "hidden": false }, { "hidden": false }],
        })
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

    /// `count` clusters whose triples all differ, at one shared fused value.
    fn saturated(count: usize, fused: f64) -> Value {
        let clusters: Vec<Value> = (0..count)
            .map(|index| {
                let step = f64::from(u32::try_from(index).unwrap_or(0)) / 100.0;
                cluster("nearly_identical", 0.2 + step, 0.3 + step, fused)
            })
            .collect();
        report(&clusters)
    }

    #[test]
    fn saturation_across_differing_evidence_is_reported() {
        let mut failures = Vec::new();
        check_fused_spread(&saturated(12, 1.0), &mut failures);
        assert_eq!(
            failures.len(),
            1,
            "a collapsed confidence axis must be reported"
        );
        assert_eq!(only_check(&failures), Some("fused_spread"));
        assert!(
            detail_mentions(&failures, "12 visible clusters"),
            "the detail must name the population size: {failures:?}",
        );
        assert!(
            detail_mentions(&failures, "1.000"),
            "and the single value they all rendered: {failures:?}",
        );
    }

    #[test]
    fn saturation_at_any_value_is_reported_not_just_at_one() {
        // The defect is collapse, not the number collapsed onto. A gate that
        // only recognised `fused == 1.0` would miss a mid-band collapse.
        let mut failures = Vec::new();
        check_fused_spread(&saturated(12, 0.42), &mut failures);
        assert_eq!(failures.len(), 1);
        assert!(detail_mentions(&failures, "0.420"), "{failures:?}");
    }

    #[test]
    fn a_spread_confidence_passes() {
        let clusters: Vec<Value> = (0..12)
            .map(|index| {
                let step = f64::from(u32::try_from(index).unwrap_or(0)) / 100.0;
                cluster("nearly_identical", 0.2 + step, 0.3 + step, 0.5 + step)
            })
            .collect();
        let mut failures = Vec::new();
        check_fused_spread(&report(&clusters), &mut failures);
        assert!(
            failures.is_empty(),
            "a spread axis is the healthy case: {failures:?}"
        );
    }

    #[test]
    fn identical_evidence_may_render_one_confidence() {
        // Twelve byte-identical pairs really do all score 1.0. The check must
        // fire on collapse *despite* differing inputs, never on agreement.
        let clusters: Vec<Value> = (0..12)
            .map(|_| cluster("identical", 1.0, 1.0, 1.0))
            .collect();
        let mut failures = Vec::new();
        check_fused_spread(&report(&clusters), &mut failures);
        assert!(
            failures.is_empty(),
            "identical inputs may share an output: {failures:?}"
        );
    }

    #[test]
    fn a_small_report_is_not_judged_on_spread() {
        let mut failures = Vec::new();
        check_fused_spread(&saturated(SPREAD_MIN_CLUSTERS - 1, 1.0), &mut failures);
        assert!(
            failures.is_empty(),
            "too small a population to conclude anything"
        );
        check_fused_spread(&saturated(SPREAD_MIN_CLUSTERS, 1.0), &mut failures);
        assert_eq!(failures.len(), 1, "and exactly at the floor it is judged");
    }

    #[test]
    fn hidden_clusters_cannot_collapse_the_spread() {
        let clusters: Vec<Value> = (0..12)
            .map(|index| {
                let step = f64::from(u32::try_from(index).unwrap_or(0)) / 100.0;
                cluster("nearly_identical", 0.2 + step, 0.3 + step, 1.0)
            })
            .collect();
        let clusters: Vec<Value> = clusters.into_iter().map(hide).collect();
        let mut failures = Vec::new();
        check_fused_spread(&report(&clusters), &mut failures);
        assert!(
            failures.is_empty(),
            "a hidden cluster makes no claim to the user, so it cannot fail a claim check"
        );
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
    fn one_act_now_cluster_clears_the_recall_check() {
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
        clusters.push(hide(cluster("identical", 1.0, 1.0, 1.0)));
        let mut failures = Vec::new();
        check_type2_recall(&report(&clusters), &mut failures);
        assert_eq!(failures.len(), 1, "a hidden rescue is no rescue");
    }
}
