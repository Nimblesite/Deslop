//! [CORPUS-BASELINE] The content-evidence checks the real-repository gate runs
//! over a finished report: `type2_gate_liveness` and curated `type2_recall`.
//!
//! Both exist because the synthetic fixtures that pin
//! [FUSED-STRATEGY-BOUNDED-MAX] and [FUSED-CONTENT-GATE] are built to
//! demonstrate a mechanism, not to survive a real corpus. A fixture proves the
//! gate *can* separate a proven rename from sibling scaffolding on five files
//! the author chose; it says nothing about whether the operating point holds
//! across 30,000 real ones. These checks catch operating-point and recall
//! failures that only appear at that scale.
//!
//! Both are keyed on rendered report fields only, with no rank in the key, so
//! they are stable against the cluster-order churn `corpus/known-failures.json`
//! documents.

use deslop_core::{buckets::has_saturating_shape_evidence, wire_generated::ReportSignals};
use serde_json::Value;

use crate::corpus::{
    cluster_shows_span, field_u64, reports_clone_spanning, visible_clusters, Failure,
};

/// Minimum demoted clusters before [`check_type2_gate_liveness`] will judge a
/// report.
///
/// Below this the absence of supported duplicate findings is ordinary — a clean repository
/// has neither population. The check fires on the *shape* of a report that
/// found plenty of same-shape families and vouched for none of them.
const TYPE2_MIN_DEMOTED: usize = 20;

/// The bucket a shape-identical cluster reaches only by the content gate
/// vouching for it — the one supported destination
/// [`check_type2_gate_liveness`] can read as evidence the gate is alive.
///
/// `identical` is deliberately **not** here. That bucket is decided by raw
/// byte-equivalence in `report_bucket_kind`, and both `route_shape_identical`
/// and the content gate return before touching it, so a byte-identical
/// clone is proof about the *byte comparison*, not about the gate. Counting it
/// made the check vacuous: Tokio renders 452 `identical` clusters, so every
/// Type-2 rename in the repository could regress into the demoted tier and the
/// gate would still pass.
///
/// The bucket alone is not enough either: the token-LSH Type-3 path can
/// classify a cluster `nearly_identical` from token overlap *below* the
/// saturating floor, in which case the content gate returned without
/// ever judging it — so counting such a cluster as gate evidence let one
/// unrelated near miss keep this check green while every genuine rename sank.
/// [`gate_vouched`] therefore also requires the gate's own precondition,
/// [`has_saturating_shape_evidence`].
const CONTENT_VOUCHED_BUCKET: &str = "nearly_identical";

/// Wire bucket labels the content gate demotes a shape-identical cluster into.
const DEMOTED_BUCKETS: [&str; 2] = ["structural_only", "loosely_similar"];

/// [CORPUS-BASELINE] `type2_gate_liveness` — the content gate must vouch for
/// *something* when it demotes a large same-shape population.
///
/// This is a population-shape heuristic, **not** a recall assertion — it
/// identifies no expected pair and reads no curated ground truth (that is
/// [`check_type2_curated_recall`]'s job). What it catches is the catastrophic
/// operating-point failure [FUSED-CONTENT-GATE] makes possible:
/// `route_shape_identical` demotes a shape-identical cluster whose content
/// evidence is absent; set the operating point slightly too high and every
/// genuine Type-2 rename in a real repository sinks into the demoted tier
/// with the scaffolding it was built to separate. The report stays plausible
/// — it is full of clusters — while every finding a user would act on has
/// quietly become "verify before extracting".
///
/// A repository that produced a large demoted population and *zero*
/// gate-vouched clusters is that failure. Only a [`gate_vouched`] cluster
/// counts as evidence the gate is alive: the right bucket reached *through*
/// the gate's own precondition — see [`CONTENT_VOUCHED_BUCKET`] for why
/// neither byte-identical clusters nor sub-floor token near-misses count.
pub fn check_type2_gate_liveness(report: &Value, failures: &mut Vec<Failure>) {
    let clusters = visible_clusters(report);
    let demoted = clusters
        .iter()
        .filter(|c| in_set(c, &DEMOTED_BUCKETS))
        .count();
    let vouched = clusters.iter().filter(|c| gate_vouched(c)).count();
    if demoted < TYPE2_MIN_DEMOTED || vouched > 0 {
        return;
    }
    let proven = clusters
        .iter()
        .filter(|c| in_set(c, &["identical"]))
        .count();
    failures.push(Failure::new(
        "type2_gate_liveness",
        format!(
            "{demoted} same-shape clusters were demoted and not one reached \
             `{CONTENT_VOUCHED_BUCKET}` through the gate — the content gate vouched for \
             nothing in the whole repository, so every genuine rename is being reported as \
             unverified scaffolding (the {proven} byte-identical clusters here are decided \
             before the gate runs and cannot stand in for it)",
        ),
    ));
}

/// True when a cluster is evidence the content gate vouched: the vouched
/// bucket, reached with the saturating shape evidence that is the gate's own
/// precondition. A `nearly_identical` cluster *below* both saturation lines
/// was classified by the ordinary token-LSH Type-3 path — the gate returned
/// without judging it, so it proves nothing about the gate.
fn gate_vouched(cluster: &Value) -> bool {
    in_set(cluster, &[CONTENT_VOUCHED_BUCKET])
        && has_saturating_shape_evidence(rendered_signals(cluster))
}

/// The cluster's rendered signal breakdown, absent axes read as zero.
fn rendered_signals(cluster: &Value) -> ReportSignals {
    ReportSignals {
        structural: signal(cluster, "structural"),
        token_jaccard: signal(cluster, "token_jaccard"),
        embedding_cos: signal(cluster, "embedding_cos"),
        shape: signal(cluster, "shape"),
        pair_agreement: signal(cluster, "pair_agreement"),
        pair_rename_consistency: signal(cluster, "pair_rename_consistency"),
        literal_fraction: signal(cluster, "literal_fraction"),
    }
}

/// Buckets a byte-identical clone may legitimately render in.
///
/// Exactly one. [CORPUS-RECALL] defines `must_find` as duplication a human
/// confirmed **byte for byte**, with the diff that proved it, so anything
/// short of `identical` is the engine disagreeing with a verified fact
/// about the source — the same defect class as missing the pair outright,
/// only harder to see because the report still shows something.
const VERBATIM_BUCKET: &str = "identical";

/// [CORPUS-RECALL] `recall` and `recall_quality` — every hand-verified
/// byte-identical duplicate in `must_find` must be reported, shown, labelled
/// `identical`, and ranked where a user would find it.
///
/// `recall` alone used to be the whole assertion, and it asked only that
/// *some* cluster's occurrence paths covered the curated files. A 137-line
/// byte-identical clone that rendered `loosely_similar`, hid one of its two
/// occurrences and ranked #900 satisfied it completely — while the Type-2
/// check next door already demanded span *plus* bucket *plus* visibility for
/// the strictly harder case. The byte-identical case is the easier proof and
/// held the weaker contract; `recall_quality` closes that.
///
/// An empty list asserts nothing, and an entry naming fewer than two files
/// fails rather than passing vacuously.
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

/// Judges one curated `must_find` entry against the rendered report.
fn check_one_curated_clone(entry: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let files = curated_files(entry);
    let why = entry.get("why").and_then(Value::as_str).unwrap_or("");
    if !reports_clone_spanning(report, &files) {
        failures.push(Failure::new(
            "recall",
            format!("no cluster spans {files:?}. Verified duplicate: {why}"),
        ));
        return;
    }
    let ranked: Vec<(usize, &Value)> = visible_clusters(report)
        .into_iter()
        .enumerate()
        .filter(|(_, cluster)| cluster_shows_span(cluster, &files))
        .collect();
    let Some((rank, cluster)) = ranked
        .iter()
        .find(|(_, cluster)| in_set(cluster, &[VERBATIM_BUCKET]))
    else {
        failures.push(Failure::new(
            "recall_quality",
            format!(
                "a cluster spans {files:?} but no *shown* `{VERBATIM_BUCKET}` cluster does \
                 — the verified byte-identical pair was demoted to {buckets:?}, or one of \
                 its occurrences is hidden and the user never sees the pair. Verified \
                 duplicate: {why}",
                buckets = ranked
                    .iter()
                    .map(|(_, cluster)| bucket_of(cluster))
                    .collect::<Vec<_>>(),
            ),
        ));
        return;
    };
    check_rank_ceiling(entry, *rank, cluster, &files, why, failures);
}

/// Applies the entry's optional `max_rank`. A curated 137-line clone ranking
/// below the scaffolding is a ranking defect the gate should name, not a
/// number it should print.
fn check_rank_ceiling(
    entry: &Value,
    rank: usize,
    cluster: &Value,
    files: &[String],
    why: &str,
    failures: &mut Vec<Failure>,
) {
    let Some(ceiling) = entry.get("max_rank").and_then(Value::as_u64) else {
        return;
    };
    let ceiling = usize::try_from(ceiling).unwrap_or(usize::MAX);
    if rank > ceiling {
        failures.push(Failure::new(
            "recall_quality",
            format!(
                "the verified duplicate spanning {files:?} is reported, but ranks {rank} \
                 against a curated ceiling of {ceiling}. Ranking is the product: a finding \
                 a user never scrolls to is a finding they do not get. Bucket \
                 {bucket}, size {size}. Verified duplicate: {why}",
                bucket = bucket_of(cluster),
                size = cluster
                    .get("size")
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
            ),
        ));
    }
}

/// The entry's curated file list. An entry naming fewer than two files
/// yields an empty list, which every predicate here refuses.
fn curated_files(entry: &Value) -> Vec<String> {
    let files: Vec<String> = entry
        .get("files")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|file| file.as_str().map(ToOwned::to_owned))
        .collect();
    if files.len() < 2 {
        return Vec::new();
    }
    files
}

/// The cluster's rendered bucket label, or a placeholder.
fn bucket_of(cluster: &Value) -> &str {
    cluster
        .get("bucket")
        .and_then(Value::as_str)
        .unwrap_or("<unlabelled>")
}

/// The rendered cluster field carrying its extent, in normalized AST nodes.
/// [CORPUS-RECALL] compares it against a curated entry's `min_nodes` floor.
const CANONICAL_NODE_COUNT: &str = "canonical_node_count";

/// The manifest field a `must_find_type2` entry must curate: the smallest
/// extent, in [`CANONICAL_NODE_COUNT`] nodes, that can credibly be the
/// curated duplicate. A floor, not a pin — the correct extent moves between
/// builds while the orders of magnitude separating it from boilerplate
/// fragments do not (gh #439).
const CURATED_EXTENT_FIELD: &str = "min_nodes";

/// [CORPUS-RECALL] `type2_recall` — every hand-verified Type-2 rename in the
/// manifest's `must_find_type2` list must be reported as a visible,
/// gate-vouched cluster spanning its curated files **at the curated extent**.
///
/// This is the actual recall assertion [`check_type2_gate_liveness`] is not:
/// each entry names a pair a human verified is a rename-duplicate by diffing
/// the code, so a miss is a false negative on known ground truth, a hidden
/// cluster is a claim the user never sees, and a demoted or sub-floor bucket
/// means the gate failed to vouch for a proven rename. A spanning cluster
/// below the entry's `min_nodes` floor is a fragment or a boilerplate family
/// touching the curated paths, not the curated duplicate — gh #439 shows the
/// check staying green with its ground truth deleted when extent is ignored.
/// An empty list asserts nothing — `must_find_status` in the manifest says so
/// explicitly — but an entry that curates no extent fails rather than passing
/// on a path overlap, the stance [CORPUS-SCOPE] takes on a missing
/// `expect_files_min`.
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

/// Judges one curated `must_find_type2` entry against the rendered report.
fn check_one_curated_type2(entry: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let files = curated_files(entry);
    let why = entry.get("why").and_then(Value::as_str).unwrap_or("");
    let Some(min_nodes) = entry.get(CURATED_EXTENT_FIELD).and_then(Value::as_u64) else {
        failures.push(missing_extent_curation(&files, why));
        return;
    };
    if !reports_clone_spanning(report, &files) {
        failures.push(Failure::new(
            "type2_recall",
            format!("no cluster spans {files:?}. Hand-verified Type-2 rename: {why}"),
        ));
        return;
    }
    check_vouched_at_extent(report, &files, min_nodes, why, failures);
}

/// The extent clause of one curated entry: among the shown, gate-vouched
/// clusters spanning the curated files, the widest must reach the entry's
/// `min_nodes` floor. Path overlap alone let a 31-node accessor family
/// answer for a deleted 395-node module rename (gh #439 witness 1) and a
/// 39-node buried fragment answer for the whole-module view (witness 2).
fn check_vouched_at_extent(
    report: &Value,
    files: &[String],
    min_nodes: u64,
    why: &str,
    failures: &mut Vec<Failure>,
) {
    let widest = visible_clusters(report)
        .into_iter()
        .filter(|cluster| gate_vouched(cluster) && cluster_shows_span(cluster, files))
        .map(|cluster| field_u64(cluster, CANONICAL_NODE_COUNT))
        .max();
    match widest {
        None => failures.push(unvouched_span(files, why)),
        Some(widest) if widest < min_nodes => {
            failures.push(below_curated_extent(files, widest, min_nodes, why));
        }
        Some(_) => {}
    }
}

/// The failure for an entry that curates no extent. Without `min_nodes` the
/// entry cannot tell the module from a fragment, so it must fail rather than
/// pass on the strength of a path overlap — otherwise gh #439 reopens the
/// next time a manifest adds an entry.
fn missing_extent_curation(files: &[String], why: &str) -> Failure {
    Failure::new(
        "type2_recall",
        format!(
            "entry for {files:?} curates no `{CURATED_EXTENT_FIELD}`, so nothing pins the \
             extent of the curated duplicate and any cluster touching its paths would \
             satisfy the check, however small (gh #439). Curate the smallest \
             `{CANONICAL_NODE_COUNT}` that can credibly be this rename. Hand-verified \
             Type-2 rename: {why}"
        ),
    )
}

/// The failure for a curated pair whose only spanning evidence is not a
/// shown, gate-vouched cluster.
fn unvouched_span(files: &[String], why: &str) -> Failure {
    Failure::new(
        "type2_recall",
        format!(
            "a cluster spans {files:?} but no shown gate-vouched \
             `{CONTENT_VOUCHED_BUCKET}` cluster does — the proven rename was demoted, \
             one of its occurrences is hidden, or its evidence did not come from the \
             gate. Hand-verified Type-2 rename: {why}"
        ),
    )
}

/// The failure for a curated pair whose spanning, vouched evidence is all
/// below the curated extent — too small to be the curated duplicate.
fn below_curated_extent(files: &[String], widest: u64, min_nodes: u64, why: &str) -> Failure {
    Failure::new(
        "type2_recall",
        format!(
            "the widest shown gate-vouched cluster spanning {files:?} has an extent of \
             {widest} nodes against the curated `{CURATED_EXTENT_FIELD}` floor of \
             {min_nodes} — too small to be the curated duplicate. A boilerplate family \
             or buried fragment touching both paths is not the module written twice \
             (gh #439). Hand-verified Type-2 rename: {why}"
        ),
    )
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
