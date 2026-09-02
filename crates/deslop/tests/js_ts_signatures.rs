//! End-to-end signature/detection tests for JavaScript, TypeScript, and
//! TSX ([LANG-CAND-JAVASCRIPT], [LANG-CAND-TYPESCRIPT]).
//!
//! The parsers use separate tree-sitter grammars but share the same
//! normalisation function. These black-box CLI tests pin that contract:
//! renamed Type-2 clones are admitted and reported as byte-distinct
//! renames (never as verbatim copies), and Type-3 near misses still
//! surface through shared subtrees — with the *enclosing* view selected,
//! never the Merkle-equal fragment the gh #408 recall hole would publish
//! in its place. On the mass-only wire the byte-level fact is the honest
//! proof of both: a reported rename must slice to differing bytes
//! ([PIPELINE-CLUSTER-CLOSURE]).

use std::{collections::BTreeSet, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
use crate::common::*;

#[test]
fn javascript_type2_rename_clone_is_reported_byte_distinct() -> Result<()> {
    // `summarizeOrders` → `collectInvoices` is a maximal rename with all
    // four numeric anchors (`0`, `100`, `0.9`, `0`) preserved in position.
    assert_type2_clone("javascript-small", 10, "alpha.js", "beta.js")
}

#[test]
fn typescript_type2_rename_clone_is_reported_byte_distinct() -> Result<()> {
    assert_type2_clone("typescript-small", 12, "alpha.ts", "beta.ts")
}

#[test]
fn tsx_type2_rename_clone_is_reported_byte_distinct() -> Result<()> {
    assert_type2_clone("tsx-small", 10, "Card.tsx", "Tile.tsx")
}

#[test]
fn javascript_near_miss_produces_cross_file_structural_cluster() -> Result<()> {
    // GH #427, closed. JavaScript used to publish the nested
    // `let running = 0; for (…)` run rather than the enclosing function
    // pair TypeScript reports off the same source shape, and that
    // fragment view was Merkle-equal — so one language called the same
    // code an exact clone and the other a near miss. The fragment won
    // because the same-file overlap collapse ranked an overlapping run
    // by cross-file edge strength, and a window scores higher exactly to
    // the extent that it drops what the two copies disagree on
    // ([PIPELINE-CLUSTER-EXACT-SCOPE]). Both languages now select the
    // enclosing view, which is what #427 asked for. The byte-level pin
    // below keeps it honest: a byte-identical fragment selected in place
    // of the enclosing near-miss would slice to identical bytes and fail.
    assert_type3_clone("javascript-type3", 8, "delta.js", "epsilon.js")
}

#[test]
fn typescript_near_miss_produces_cross_file_structural_cluster() -> Result<()> {
    assert_type3_clone("typescript-type3", 8, "delta.ts", "epsilon.ts")
}

/// Asserts that a renamed Type-2 fixture's cluster spanning both files
/// is admitted, byte-distinct (a rename, not a copy), and free of any
/// pair-only surface.
fn assert_type2_clone(fixture_name: &str, min_nodes: u32, left: &str, right: &str) -> Result<()> {
    let scan_root = fixture(fixture_name);
    let report = run_report(&scan_root, min_nodes)?;
    let top = top_cluster(&report, fixture_name)?;
    assert_admitted_rename_cluster(&scan_root, top, fixture_name, left, right, &report)?;
    Ok(())
}

/// Asserts that a Type-3 near miss surfaces as a cross-file cluster whose
/// occurrences span both files and slice to differing bytes — the
/// enclosing view, not the byte-identical fragment gh #408 would publish.
fn assert_type3_clone(fixture_name: &str, min_nodes: u32, left: &str, right: &str) -> Result<()> {
    let scan_root = fixture(fixture_name);
    let report = run_report(&scan_root, min_nodes)?;
    let cluster = clusters(&report)
        .iter()
        .find(|cluster| spans_both(cluster, left, right))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{fixture_name} must report a clone spanning {left} and {right}: {report:#}"
            )
        })?;
    assert_admitted_rename_cluster(&scan_root, cluster, fixture_name, left, right, &report)?;
    Ok(())
}

/// The wire contract every renamed/near-miss clone must satisfy
/// ([PIPELINE-CLUSTER-CLOSURE]): admitted and mass-honest, no pair-only
/// surface on the cluster, and — the byte-level truth the deleted
/// `structural`/`token_jaccard` axes used to proxy — the occurrences
/// slice to *differing* source bytes. A Merkle-equal fragment selected in
/// place of the enclosing view would slice to identical bytes and fail
/// here, which is exactly the gh #408/#427 recall hole.
fn assert_admitted_rename_cluster(
    scan_root: &Path,
    cluster: &Value,
    fixture_name: &str,
    left: &str,
    right: &str,
    report: &Value,
) -> Result<()> {
    assert_structural_only_contract(cluster, fixture_name);
    assert_no_pair_surface_on_cluster(cluster, fixture_name);
    assert!(
        !has_verbatim_pair(scan_root, cluster)?,
        "{fixture_name}: {left} and {right} are a rename / near-miss and must \
         slice to differing bytes — a verbatim (byte-identical) reading means \
         the fragment view was selected in place of the enclosing pair, the \
         gh #408 recall hole: {report:#}"
    );
    assert!(
        spans_both(cluster, left, right),
        "{fixture_name}: the clone must span {left} and {right}: {report:#}"
    );
    Ok(())
}

/// Returns the top-ranked visible cluster, or an actionable test error.
fn top_cluster<'a>(report: &'a Value, fixture_name: &str) -> Result<&'a Value> {
    clusters(report)
        .first()
        .ok_or_else(|| anyhow::anyhow!("{fixture_name} must produce at least one cluster"))
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
