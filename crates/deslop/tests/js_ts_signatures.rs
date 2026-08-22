//! End-to-end signature/detection tests for JavaScript, TypeScript, and
//! TSX ([LANG-CAND-JAVASCRIPT], [LANG-CAND-TYPESCRIPT]).
//!
//! The parsers use separate tree-sitter grammars but share the same
//! normalisation function. These black-box CLI tests pin that contract:
//! renamed Type-2 clones reach identical structural and token signals and,
//! because measured content evidence — preserved anchors plus
//! Baker-corroborated substitutions ([TECH-PMATCH-BAKER]) — proves the
//! identifier mapping ([FUSION-CONTENT-GATE]), route to the act-now
//! `nearly_identical` bucket; Type-3 near misses still surface through
//! shared subtrees.

use std::{collections::BTreeSet, path::Path};

use anyhow::Result;
use deslop_core::pair::SHARED_SUBTREE_MIN_OVERLAP;
use serde_json::Value;

use crate::common::*;

#[test]
fn javascript_type2_clone_has_structural_and_token_jaccard_of_one() -> Result<()> {
    // `summarizeOrders` → `collectInvoices` is a maximal rename with all
    // four numeric anchors (`0`, `100`, `0.9`, `0`) preserved in position,
    // so the content gate proves the bijective mapping and promotes the
    // pair out of the old #134 `structural_only` demotion.
    assert_type2_clone("javascript-small", 10, "alpha.js", "beta.js")
}

#[test]
fn typescript_type2_clone_has_structural_and_token_jaccard_of_one() -> Result<()> {
    assert_type2_clone("typescript-small", 12, "alpha.ts", "beta.ts")
}

#[test]
fn tsx_type2_clone_has_structural_and_token_jaccard_of_one() -> Result<()> {
    assert_type2_clone("tsx-small", 10, "Card.tsx", "Tile.tsx")
}

#[test]
fn javascript_near_miss_produces_cross_file_structural_cluster() -> Result<()> {
    // GH #427: JavaScript publishes the nested `let running = 0; for (…)`
    // run rather than the enclosing function pair TypeScript reports off
    // the same source shape, and a fragment view is Merkle-equal. This
    // pins what the engine does today; #427 tracks making it agree with
    // every other language, at which point this becomes `Overlap::Graded`.
    assert_type3_clone(
        "javascript-type3",
        8,
        "delta.js",
        "epsilon.js",
        Overlap::MerkleEqual,
    )
}

#[test]
fn typescript_near_miss_produces_cross_file_structural_cluster() -> Result<()> {
    assert_type3_clone(
        "typescript-type3",
        8,
        "delta.ts",
        "epsilon.ts",
        Overlap::Graded,
    )
}

/// Asserts that a renamed Type-2 fixture's top-ranked cluster has perfect
/// structural identity, routes to the act-now `nearly_identical` bucket,
/// and renders an exact token Jaccard — the content gate corrects the
/// placeholder-dominated token fallback once the preserved anchors prove
/// the rename ([FUSION-CONTENT-GATE]).
fn assert_type2_clone(fixture_name: &str, min_nodes: u32, left: &str, right: &str) -> Result<()> {
    let report = run_report(&fixture(fixture_name), min_nodes)?;
    let top = top_cluster(&report, fixture_name)?;
    assert!(is_exact_one(signal(top, "structural")));
    assert_eq!(
        cluster_bucket(top),
        "nearly_identical",
        "{fixture_name} top cluster bucket mismatch: {report:#}"
    );
    assert!(is_exact_one(signal(top, "token_jaccard")));
    assert!(spans_both(top, left, right));
    Ok(())
}

/// What [FUSION-SHARED-SUBTREE] licenses a cluster's `structural` to be.
///
/// `structural` is a graded alignment — `1 - TED / max(nodes)` — and only
/// Merkle-equal endpoints short-circuit to `1.0`. Which of the two a
/// fixture lands on is a fact about the view the pipeline elected, so the
/// caller states it rather than the helper assuming it: asserting `1.0`
/// everywhere pinned the nested fragment that gh #408 stopped publishing
/// in place of the method, and no longer holds for the languages that
/// report the enclosing pair.
#[derive(Clone, Copy)]
enum Overlap {
    /// The elected view is Merkle-equal across files, so the measure
    /// short-circuits and nothing less than `1.0` is correct.
    MerkleEqual,
    /// The elected view differs by a real subtree — a Type-3 near miss —
    /// so the graded alignment must land strictly below `1.0` and at or
    /// above [`SHARED_SUBTREE_MIN_OVERLAP`], the floor row 4b admitted the
    /// pair on.
    Graded,
}

/// Asserts that a Type-3 near miss surfaces a token-supported
/// `nearly_identical` cross-file cluster whose measured overlap is what
/// `overlap` says it must be, corroborated by a saturated token axis.
fn assert_type3_clone(
    fixture_name: &str,
    min_nodes: u32,
    left: &str,
    right: &str,
    overlap: Overlap,
) -> Result<()> {
    let report = run_report(&fixture(fixture_name), min_nodes)?;
    let Some(cluster) = clusters(&report).iter().find(|cluster| {
        spans_both(cluster, left, right) && cluster_bucket(cluster) == "nearly_identical"
    }) else {
        anyhow::bail!("{fixture_name} must report a nearly_identical clone spanning {left} and {right}: {report:#}");
    };
    let structural = signal(cluster, "structural");
    assert!(
        structural >= SHARED_SUBTREE_MIN_OVERLAP,
        "{fixture_name}: a shared-subtree near miss is admitted on \
         `structural >= {SHARED_SUBTREE_MIN_OVERLAP}` and must still measure \
         at least that once rendered, or the report is showing a cluster the \
         pipeline would not admit: got {structural}: {report:#}"
    );
    match overlap {
        Overlap::MerkleEqual => assert!(
            is_exact_one(structural),
            "{fixture_name}: the elected view is Merkle-equal across \
             {left} and {right}, and [FUSION-SHARED-SUBTREE] short-circuits \
             those to exactly 1.0 — a graded value here means a different, \
             wider view was elected: got {structural}: {report:#}"
        ),
        Overlap::Graded => assert!(
            structural < 1.0,
            "{fixture_name}: {left} and {right} differ by a real subtree, so \
             `1 - TED / max(nodes)` cannot reach 1.0 — measuring exactly one \
             means the fragment view is being published in place of the \
             enclosing pair, the gh #408 recall hole: got {structural}: \
             {report:#}"
        ),
    }
    assert!(
        is_exact_one(signal(cluster, "token_jaccard")),
        "{fixture_name}: normalisation is rename-invariant, so the token axis \
         must saturate across {left} and {right}: {report:#}"
    );
    Ok(())
}

/// Returns the top-ranked visible cluster, or an actionable test error.
fn top_cluster<'a>(report: &'a Value, fixture_name: &str) -> Result<&'a Value> {
    clusters(report)
        .first()
        .ok_or_else(|| anyhow::anyhow!("{fixture_name} must produce at least one cluster"))
}

/// Returns true when a floating-point signal is exactly one.
fn is_exact_one(value: f64) -> bool {
    (value - 1.0).abs() <= f64::EPSILON
}

/// Returns true when `cluster` contains occurrences in both files.
fn spans_both(cluster: &Value, left: &str, right: &str) -> bool {
    let files: BTreeSet<String> = occurrence_files(cluster)
        .into_iter()
        .filter_map(|path| {
            Path::new(&path)
                .file_name()
                .map(std::borrow::ToOwned::to_owned)
        })
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    files.contains(left) && files.contains(right)
}
