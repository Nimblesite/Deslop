//! [CLONE-BUCKETS-ROUTING] row 4 recall — in every language (gh #390).
//!
//! The spec routes `structural ≤ 0.01 ∧ token_jaccard ≥ 0.90` to
//! `NearlyIdentical` with no language condition. `classify_signals` had
//! no such arm, so the triple fell to its else-arm — `LooselySimilar`,
//! which the renderer hides — while
//! `report_render::is_csharp_lsh_type3_near_miss` patched the missing
//! row for C# members only. Every other language's LSH-only Type-3 pair,
//! one that passed the `LSH_ONLY_MIN_JACCARD = 0.90` and
//! `LSH_ONLY_MIN_NODE_COUNT = 40` survival floors as "real near-miss
//! duplication", reported **zero** duplication: a silent false negative.
//! Reproduced (release CLI, `--min-nodes 35 --embeddings off`) as
//! `clusters_total=0, duplicated_loc=0, clusters_hidden=1` on the Python
//! pair below. The carve-out is dissolved into the router; this test
//! pins the recall it owed.
//!
//! **The precision half lives in `issue_331_336_shape_only_saturation.rs`
//! and this file is its counterweight.** Row 4 admits on token overlap
//! alone, so it passes through [FUSION-CONTENT-GATE] like every other
//! shape-saturating route: a framework-mandated declaration family
//! measures the same anchor-free triple (`structural=0.00,
//! token_jaccard=0.93` across six distinct Flutter widgets) and is
//! demoted there on measured content evidence. That gate must not cost
//! this pair its confidence, so the assertions below pin the *fused*
//! value at act-now grade, not merely the bucket label — a fix that
//! bought #331's precision by widening the gate over genuine duplicates
//! fails here.

use std::fs;
use std::path::Path;

use crate::common::{
    approx, cluster_bucket, cluster_size, expect_cluster_spanning, field,
    incremental::{
        assert_pass, assert_reports_equal, assert_warm_pass, cold_then_warm,
        edit_preserving_offsets, run_store_on, ColdThenWarm,
    },
    metric_field, run_report, signal,
    verdict::loc_as_f64,
    Result,
};

/// Subtree floor at which only the two function roots (and whole-body
/// windows straddling the reorder) fingerprint — probed so exactly one
/// candidate cluster exists and no sibling window matches structurally.
///
/// It must sit **above** the coincidental window, not merely above the
/// statements. At 35 the scan published a 38-node three-statement window
/// instead of the reordered pair: `closing_total = 0, carried_balance = 0,
/// for …` against `opening_total = 0, closing_total = 0, for …` is
/// `[assign, assign, for]` on both sides, which normalisation collapses to
/// one tree, so it rendered a legitimate `structural = 1.0` and the fixture
/// stopped exercising the LSH-only route it exists for. Kept at the same
/// value as [`LSH_ONLY_NODE_FLOOR`], which the surviving endpoints must
/// clear anyway.
const MIN_NODES: u32 = 40;

/// `pair::LSH_ONLY_MIN_NODE_COUNT`: both endpoints of an LSH-only pair
/// must carry at least this many nodes to survive clustering.
const LSH_ONLY_NODE_FLOOR: u64 = 40;

/// Statements shared verbatim by both functions: three initialisers,
/// the accumulation loop, and the settlement tail.
const LEFT_SOURCE: &str = "import os\nimport sys\n\n\ndef reconcile(entries, floor):\n\
    \x20   opening_total = 0\n\
    \x20   closing_total = 0\n\
    \x20   carried_balance = 0\n\
    \x20   for entry in entries:\n\
    \x20       if entry > floor:\n\
    \x20           opening_total = opening_total + entry * 3\n\
    \x20       else:\n\
    \x20           closing_total = closing_total - entry\n\
    \x20   spread_margin = opening_total - closing_total\n\
    \x20   weighted_margin = spread_margin * 2\n\
    \x20   settlement_value = weighted_margin + carried_balance\n\
    \x20   return settlement_value + opening_total\n";

/// The same statements reordered — `carried_balance` moved across the
/// loop and the settlement tail swapped — so no ≥[`MIN_NODES`]-node
/// subtree or sibling window survives structurally identical, while the
/// token k-gram overlap stays at the measured [`MEASURED_JACCARD`], above
/// the `LSH_ONLY_MIN_JACCARD = 0.90` floor.
///
/// The reordered pair itself still shares every statement subtree, so it
/// measures a graded overlap rather than nothing ([FUSION-SHARED-SUBTREE]);
/// what it does not have is an *exact* anchor, which is what makes the
/// token axis the only route that admits it.
const RIGHT_SOURCE: &str = "import os\nimport sys\n\n\ndef settle(entries, floor):\n\
    \x20   opening_total = 0\n\
    \x20   closing_total = 0\n\
    \x20   for entry in entries:\n\
    \x20       if entry > floor:\n\
    \x20           opening_total = opening_total + entry * 3\n\
    \x20       else:\n\
    \x20           closing_total = closing_total - entry\n\
    \x20   carried_balance = 0\n\
    \x20   spread_margin = opening_total - closing_total\n\
    \x20   settlement_value = weighted_margin + carried_balance\n\
    \x20   weighted_margin = spread_margin * 2\n\
    \x20   return settlement_value + opening_total\n";

/// The file the mixed pass edits — the right-hand member of the pair.
const RIGHT_FILE: &str = "ledger_right.py";

/// Token-Jaccard the pair measures (`MinHash` estimate, deterministic per
/// [PIPELINE-DETERMINISM]) — captured from the reproducing run. Measured
/// again after #408 made `structural` a subtree-overlap grade and
/// [PIPELINE-NORMALIZE-AST-OPERATOR] added operator kinds to the token
/// stream: the pair now admits through the **anchored** near-miss row
/// (`structural` 0.85 shared-subtree overlap) rather than the anchor-free
/// row 4, and the richer token stream lowers the k-gram estimate to
/// 95/128 components. The recall contract this file pins — reported,
/// `nearly_identical`, act-now fused — is unchanged.
const MEASURED_JACCARD: f64 = 0.742_187_5;

/// Seeds the two-file Python corpus.
fn seed(scan_root: &Path) -> Result<()> {
    fs::create_dir_all(scan_root)?;
    fs::write(scan_root.join("ledger_left.py"), LEFT_SOURCE)?;
    fs::write(scan_root.join(RIGHT_FILE), RIGHT_SOURCE)?;
    Ok(())
}

// [CLONE-BUCKETS-ROUTING] `structural ≤ 0.01 ∧ token_jaccard ≥ 0.90` →
// `NearlyIdentical`, for every language. A pure statement-reorder clone
// is exactly the Type-3 population the LSH-only path exists to recall;
// hiding it renders a fully-duplicated pair as zero duplication.
#[test]
fn a_python_lsh_only_type3_pair_is_reported_as_nearly_identical() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&scan_root)?;
    let report = run_report(&scan_root, MIN_NODES)?;

    assert_pair_verdict(&report, "store-off pass")
}

/// The whole verdict for one pass: both files analysed, exactly one
/// cluster spanning them, the spec's bucket, the exact signal triple, and
/// every duplication figure off zero and self-consistent. Applied to
/// *every* state of the persistence matrix below — a warm pass that
/// merely renders "some cluster" is not the recall this row owes.
fn assert_pair_verdict(report: &serde_json::Value, label: &str) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(2),
        "{label}: both seeded files must be analysed: {report}"
    );
    let cluster = expect_cluster_spanning(report, &["ledger_left.py", "ledger_right.py"])?;
    assert_eq!(
        cluster_bucket(cluster),
        "nearly_identical",
        "{label}: spec row `structural ≤ 0.01 ∧ token_jaccard ≥ 0.90` routes \
         to NearlyIdentical with no language condition (taxonomy.md \
         [CLONE-BUCKETS-ROUTING]); the C#-only carve-out must not decide \
         recall: {cluster:#}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "{label}: exactly the two reordered functions form the pair: {cluster:#}"
    );
    assert_signal_triple(cluster);
    assert_recall_metrics(report)
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] The LSH-only route runs on
// *reused* signatures on a warm pass, and a signature is the only
// evidence this pair has — there is no structural anchor to fall back on.
// So the whole persistence matrix owes this verdict identically: cold,
// fully warm, a mixed pass where one file's signatures are rebuilt and
// the other's are served from the store, and a revert that full-hits.
#[test]
fn the_lsh_only_pair_keeps_its_verdict_across_the_persistence_matrix() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&scan_root)?;
    let right_fingerprints = right_file_fingerprint_count(tmp.path())?;

    let ColdThenWarm {
        cold,
        cold_events,
        warm,
        warm_events,
    } = cold_then_warm(&scan_root, &tmp.path().join("matrix"), MIN_NODES, 2)?;
    assert_pair_verdict(&cold, "cold pass")?;
    assert_pair_verdict(&warm, "fully warm pass")?;
    assert!(
        cold_events.fingerprints > right_fingerprints,
        "the pair's fingerprint total must exceed the right file's alone, or \
         the split below asserts nothing: total {} right {right_fingerprints}",
        cold_events.fingerprints
    );

    // The mixed pass: `import os` → `import io` in the right file only.
    // Identifiers collapse under normalisation, so every fingerprint,
    // span, and token k-gram is untouched while the file's content hash —
    // the store key — changes. One file must miss and rebuild, the other
    // must be served, and the report must not move a byte.
    edit_preserving_offsets(&scan_root, RIGHT_FILE, "import os", "import io")?;
    let (edited, edit_events) = run_store_on(&scan_root, &tmp.path().join("edit"), MIN_NODES, &[])?;
    assert_pass(&edited, &edit_events, 1, 1, "mixed pass");
    edit_events.assert_signatures(
        right_fingerprints,
        warm_events.fingerprints.saturating_sub(right_fingerprints),
        "mixed pass",
    );
    assert_pair_verdict(&edited, "mixed pass")?;
    assert_reports_equal(&edited, &cold, "mixed pass vs cold pass");

    edit_preserving_offsets(&scan_root, RIGHT_FILE, "import io", "import os")?;
    let (reverted, revert_events) =
        run_store_on(&scan_root, &tmp.path().join("revert"), MIN_NODES, &[])?;
    assert_warm_pass(&reverted, &revert_events, 2, "revert pass");
    assert_pair_verdict(&reverted, "revert pass")?;
    assert_reports_equal(&reverted, &cold, "revert pass vs cold pass");
    Ok(())
}

/// Fingerprint count of `ledger_right.py` alone, measured by a cold pass
/// over a one-file corpus. Derived rather than hardcoded so the mixed
/// pass can assert the *exact* rebuild/reuse split without a magic
/// number that drifts when the fixture or `min_nodes` changes.
fn right_file_fingerprint_count(tmp: &Path) -> Result<u64> {
    let solo = tmp.join("solo");
    fs::create_dir_all(&solo)?;
    fs::write(solo.join(RIGHT_FILE), RIGHT_SOURCE)?;
    let (_report, events) = run_store_on(&solo, &tmp.join("solo-out"), MIN_NODES, &[])?;
    Ok(events.fingerprints)
}

/// The agent-facing act-now line ([FUSED-THRESHOLD]) this pair must
/// stay at or above: a verbatim statement-reorder clone is duplication
/// an agent may act on, and [FUSION-CONTENT-GATE] measures real content
/// agreement here, so the gate that demotes shape-only families
/// ([CLONE-NOISE-DART-WIDGET-SCAFFOLD], #331) must leave this pair alone.
const ACT_NOW_FUSED: f64 = 0.85;

/// The pair's exact signal triple: no structural anchor, the measured
/// token overlap, embeddings off, and a fused confidence the content
/// gate left at act-now grade.
fn assert_signal_triple(cluster: &serde_json::Value) {
    let structural = signal(cluster, "structural");
    assert!(
        structural < 0.99,
        "a pure statement reorder has no *exact* structural anchor — reordering \
         rehashes the enclosing Merkle node, so an exact match here would mean \
         the reported view is not the reordered pair at all, got {structural}: \
         {cluster:#}"
    );
    // The bound is two-sided. `<= 0.01` was the old form, and it read as
    // "no structural evidence" — which was only ever true because
    // `structural` was Merkle equality. A statement reorder shares every
    // statement subtree; only their order differs, so it measures real
    // overlap ([FUSION-SHARED-SUBTREE]). Asserting the zero asserted that
    // the shared statements did not exist.
    assert!(
        structural >= deslop_core::pair::SHARED_SUBTREE_MIN_OVERLAP,
        "the reordered statements are shared subtrees and must register as \
         shape evidence, got {structural}: {cluster:#}"
    );
    let jaccard = signal(cluster, "token_jaccard");
    assert!(
        approx(jaccard, MEASURED_JACCARD),
        "token_jaccard must be the measured {MEASURED_JACCARD}, got \
         {jaccard}: {cluster:#}"
    );
    let cosine = signal(cluster, "embedding_cos");
    assert!(
        approx(cosine, 0.0),
        "embeddings are off, so the cosine must be 0.0, got {cosine}: {cluster:#}"
    );
    let fused = signal(cluster, "fused");
    assert!(
        fused >= ACT_NOW_FUSED,
        "the content gate must leave a genuine reorder clone at act-now \
         confidence (>= {ACT_NOW_FUSED}), got {fused} — demoting this pair \
         is how a #331 precision fix silently costs recall: {cluster:#}"
    );
    let nodes = field(cluster, "canonical_node_count").as_u64().unwrap_or(0);
    assert!(
        nodes >= LSH_ONLY_NODE_FLOOR,
        "both endpoints cleared the {LSH_ONLY_NODE_FLOOR}-node LSH-only \
         floor to survive, so the canonical count must too, got {nodes}: \
         {cluster:#}"
    );
}

/// [METRICS-REPO] The recall half: a reported pair must move every
/// duplication figure off zero, and the figures must agree with each
/// other — re-derived percentage, both files counted, nothing hidden.
fn assert_recall_metrics(report: &serde_json::Value) -> Result<()> {
    assert_eq!(
        (
            metric_field(report, "clusters_total").as_u64(),
            metric_field(report, "duplicated_files").as_u64(),
            field(report, "clusters_hidden").as_u64(),
        ),
        (Some(1), Some(2), Some(0)),
        "the pair is the corpus's only cluster and it must be visible, \
         not hidden: {report}"
    );
    let duplicated = metric_field(report, "duplicated_loc").as_u64().unwrap_or(0);
    assert!(
        duplicated > 0,
        "a reported clone pair must contribute duplicated LOC — zero here \
         is the false negative this test pins: {report}"
    );
    let analysed = metric_field(report, "analysed_loc").as_u64().unwrap_or(0);
    let reported = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    let expected = 100.0 * loc_as_f64(duplicated)? / loc_as_f64(analysed)?;
    assert!(
        approx(reported, expected),
        "duplication_percent must be duplicated/analysed × 100 \
         ({duplicated}/{analysed}), got {reported}: {report}"
    );
    Ok(())
}
