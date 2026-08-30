//! The `verbatim-subgroup` fixture vocabulary
//! ([CLONE-NOISE-VERBATIM-SUBGROUP]).
//!
//! Two suites read this corpus — the one pinning that a copy survives an
//! unrelated member joining its cluster, and the one pinning the price
//! the cross-file arbitration accepts. They must agree on what the copy
//! *is*: same node floor, same files, same lines, same definition of
//! "survived". Restating that per binary is how two pins over one
//! fixture drift into asserting different things about the same bytes.

use serde_json::Value;

use super::{
    approx, cluster_bucket, cluster_file_set, cluster_id, cluster_size, clusters,
    expect_cluster_spanning, field, fixture, occurrence_files, per_file_metrics, run_report,
    signal,
    signals::{signal_dump, IDENTICAL_BUCKET},
    Result,
};

/// Node floor low enough that a run of four sibling constant
/// declarations, a run of four calls, or one collection cell qualifies
/// as a candidate window — the geometry all three issues report.
pub(crate) const MIN_NODES: u32 = 8;

/// The corpus holding the copied call run: the same source the
/// `idiom-price` case holds, laid out across files instead of within
/// one.
pub(crate) const CALL_CASE: &str = "literal-calls";

/// The file the copied call run lives in. `literal-calls/` holds it
/// once beside a byte-identical twin; `idiom-price/` holds the same run
/// twice over inside this one file.
pub(crate) const CALL_ORIGIN: &str = "invoice_emitter.py";
/// The byte-identical twin, present only in the cross-file layout.
pub(crate) const CALL_TWIN: &str = "invoice_emitter_copy.py";

/// The two byte-identical call runs, and the run whose literals vary.
pub(crate) const CALL_COPY: [&str; 2] = [CALL_ORIGIN, CALL_TWIN];
/// The stranger whose only relation to [`CALL_COPY`] is its shape.
pub(crate) const CALL_STRANGER: &str = "refund_emitter.py";

/// Anchor positions of the call-run pair's elected measurement: the two
/// five-line emitters carry enough consistent literals and explained
/// identifiers to clear the ten-anchor certification point, so the rename
/// proof certifies to `1.0` ([FUSED-CONTENT-GATE]). Ten is the floor the
/// anchor mass reaches `content_gate.support_floor` at — `10 / 14 ≥ 0.70`.
pub(crate) const CALL_PAIR_ANCHORS: usize = 10;

/// Lines each copy of the call run covers — the whole five-line `emit`
/// function, not only the four `persist` calls inside it.
///
/// `invoice_emitter.py` and `invoice_emitter_copy.py` are byte-identical
/// files, so `def emit():` is as duplicated as the calls under it, and
/// the published `identical` cluster spans L1-5 of both. Four was what
/// the same-file overlap collapse elected while it ranked an
/// overlapping run by cross-file edge strength: the four-call window
/// scored higher than the function enclosing it purely by carrying less
/// code, and the `def` line went uncounted
/// ([PIPELINE-CLUSTER-EXACT-SCOPE], gh #408). The undercount was the
/// artifact; five is what the two files actually share.
pub(crate) const CALL_LOC_PER_FILE: u64 = 5;

/// Every elected-pair axis fixed by a byte-identical copy with embeddings off.
/// `literal_fraction` is corpus content, not evidence strength, so each fixture
/// asserts its own authored value where that value matters.
///
/// `pair_rename_consistency` is deliberately absent: a byte-identical pair
/// carries perfect literal consistency and coverage, but the rename proof is
/// scaled by the anchor mass `anchors / (anchors + 4)`, certified to `1.0` only
/// at ten anchors ([FUSED-CONTENT-GATE]). A small byte-identical table (four
/// assignments = eight anchors) therefore renders `0.6667` — honest, not
/// saturated — so each caller asserts its own authored value.
const VERBATIM_PAIR_SIGNALS: &[(&str, f64)] = &[
    ("structural", 1.0),
    ("token_jaccard", 1.0),
    ("shape", 1.0),
    ("embedding_cos", 0.0),
    ("pair_agreement", 1.0),
];

/// The exact rendered `pair_rename_consistency` for a byte-identical copy
/// whose elected pair carries `anchors` consistent positions: the anchor mass
/// `anchors / (anchors + 4)`, certified to 1.0 at or above ten anchors
/// ([FUSED-CONTENT-GATE]). The ten-anchor certification point is where the
/// mass reaches `content_gate.support_floor` (0.70) — the same operating
/// point `deslop_core::buckets::CONTENT_SUPPORT_FLOOR` names.
pub(crate) fn rename_consistency_for(anchors: usize) -> f64 {
    let weight = (anchors as f64) / ((anchors as f64) + 4.0);
    if weight >= 0.70 {
        1.0
    } else {
        weight
    }
}

/// Renders one `verbatim-subgroup` case.
pub(crate) fn render(case: &str, min_nodes: u32) -> Result<Value> {
    run_report(&fixture("verbatim-subgroup").join(case), min_nodes)
}

/// Per-file duplicated LOC as the report renders it, `0` when the file
/// carries no row at all.
pub(crate) fn duplicated_loc_for(report: &Value, file: &str) -> u64 {
    per_file_metrics(report)
        .iter()
        .find(|metric| {
            field(metric, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(file))
        })
        .map_or(0, |metric| {
            field(metric, "duplicated_loc").as_u64().unwrap_or_default()
        })
}

/// Every visible cluster as `id [bucket] files` — the smallest dump
/// that diagnoses a failure without re-running the scan.
pub(crate) fn published(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .map(|cluster| {
            format!(
                "{id} [{bucket}] {files:?}",
                id = cluster_id(cluster),
                bucket = cluster_bucket(cluster),
                files = occurrence_files(cluster),
            )
        })
        .collect()
}

/// Asserts the copy spanning `copy` survives as one saturated,
/// `identical`, size-2 cluster that the stranger is not part of.
pub(crate) fn assert_copy_survives_alone(
    report: &Value,
    label: &str,
    copy: &[&str; 2],
    stranger: &str,
    rename_consistency: f64,
) -> Result<()> {
    let cluster = expect_cluster_spanning(report, copy)?;
    let dump = signal_dump(cluster);
    assert_eq!(
        cluster_bucket(cluster),
        IDENTICAL_BUCKET,
        "{label}: the pair is copied byte for byte, so `{IDENTICAL_BUCKET}` is \
         the only label it may carry, whatever else joined its cluster — {dump}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "{label}: exactly the two copies are shown — {dump}"
    );
    for (name, expected) in VERBATIM_PAIR_SIGNALS {
        assert!(
            approx(signal(cluster, name), *expected),
            "{label}: byte-proven signal `{name}` must be {expected} — {dump}"
        );
    }
    assert!(
        approx(
            signal(cluster, "pair_rename_consistency"),
            rename_consistency
        ),
        "{label}: the byte-identical pair's rename consistency must be the \
         anchor-scaled value {rename_consistency}, never a saturated stand-in — \
         {dump}"
    );
    assert_eq!(
        cluster_file_set(cluster),
        copy.iter().map(|name| (*name).to_owned()).collect(),
        "{label}: the copy's cluster spans exactly its own two files"
    );
    assert!(
        !occurrence_files(cluster)
            .iter()
            .any(|file| file == stranger),
        "{label}: {stranger} is not a copy of anything — it shares only the \
         shape normalisation leaves behind, so it must not be an occurrence \
         of the copy's cluster: {files:?}",
        files = occurrence_files(cluster),
    );
    Ok(())
}
