//! 🛑 RED PIN — [CLONE-BUCKETS-ROUTING] row 4 is unimplemented outside C#.
//!
//! The spec routes `structural ≤ 0.01 ∧ token_jaccard ≥ 0.90` to
//! `NearlyIdentical` (docs/specs/taxonomy.md, no language condition).
//! `deslop-core::buckets::classify_signals` has no such arm, so the
//! triple falls to its else-arm — `LooselySimilar`, which the renderer
//! hides. `report_render::is_csharp_lsh_type3_near_miss` patches the
//! missing row for C# members only, so every other language's LSH-only
//! Type-3 pair — one that passed the `LSH_ONLY_MIN_JACCARD = 0.90` and
//! `LSH_ONLY_MIN_NODE_COUNT = 40` survival floors as "real near-miss
//! duplication" — reports **zero** duplication: a silent false
//! negative.
//!
//! Reproduced (release CLI, `--min-nodes 35 --embeddings off`): the
//! Python pair below clusters at `structural=0.0,
//! token_jaccard=0.9296875`, then renders `clusters_total=0,
//! duplicated_loc=0, clusters_hidden=1` with the cluster demoted to
//! `loosely_similar`. This test asserts the spec's routing and fails
//! until the row is implemented for every language.

mod common;

use std::fs;
use std::path::Path;

use crate::common::{
    approx, cluster_bucket, cluster_size, expect_cluster_spanning, field, metric_field, run_report,
    signal, verdict::loc_as_f64, Result,
};

/// Subtree floor at which only the two function roots (and whole-body
/// windows straddling the reorder) fingerprint — probed so exactly one
/// candidate cluster exists and no sibling window matches structurally.
const MIN_NODES: u32 = 35;

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
/// subtree or sibling window survives structurally identical
/// (`structural = 0.0`) while the token k-gram overlap stays at the
/// measured `0.9296875`, above the `LSH_ONLY_MIN_JACCARD = 0.90` floor.
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

/// Token-Jaccard the pair measures (`MinHash` estimate, deterministic per
/// [PIPELINE-DETERMINISM]) — captured from the reproducing run. Above
/// the 0.90 LSH-only floor, below the 0.95 saturating-shape line, so
/// the spec's row 4 is the *only* row that admits it.
const MEASURED_JACCARD: f64 = 0.929_687_5;

/// Seeds the two-file Python corpus.
fn seed(scan_root: &Path) -> Result<()> {
    fs::create_dir_all(scan_root)?;
    fs::write(scan_root.join("ledger_left.py"), LEFT_SOURCE)?;
    fs::write(scan_root.join("ledger_right.py"), RIGHT_SOURCE)?;
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

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both seeded files must be analysed: {report}"
    );
    let cluster = expect_cluster_spanning(&report, &["ledger_left.py", "ledger_right.py"])?;
    assert_eq!(
        cluster_bucket(cluster),
        "nearly_identical",
        "spec row `structural ≤ 0.01 ∧ token_jaccard ≥ 0.90` routes to \
         NearlyIdentical with no language condition (taxonomy.md \
         [CLONE-BUCKETS-ROUTING]); the C#-only carve-out must not decide \
         recall: {cluster:#}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "exactly the two reordered functions form the pair: {cluster:#}"
    );
    assert_signal_triple(cluster);
    assert_recall_metrics(&report)
}

/// The pair's exact signal triple: no structural anchor, the measured
/// token overlap, embeddings off.
fn assert_signal_triple(cluster: &serde_json::Value) {
    let structural = signal(cluster, "structural");
    assert!(
        structural <= 0.01,
        "a pure statement reorder has no exact structural anchor, got \
         {structural}: {cluster:#}"
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
