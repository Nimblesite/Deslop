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
/// false-positive check structurally cannot see. `route_shape_identical`
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
    let proven = clusters
        .iter()
        .filter(|c| in_set(c, &["identical"]))
        .count();
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
mod tests;
