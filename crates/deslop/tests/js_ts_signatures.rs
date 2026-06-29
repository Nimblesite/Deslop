//! End-to-end signature/detection tests for JavaScript, TypeScript, and
//! TSX ([LANG-CAND-JAVASCRIPT], [LANG-CAND-TYPESCRIPT]).
//!
//! The parsers use separate tree-sitter grammars but share the same
//! normalisation function. These black-box CLI tests pin that contract:
//! renamed Type-2 clones reach identical structural and token signals,
//! and Type-3 near misses still surface through shared subtrees.

use std::{collections::BTreeSet, path::Path};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

#[test]
fn javascript_type2_clone_has_structural_signal_of_one() -> Result<()> {
    assert_type2_clone(
        "javascript-small",
        10,
        "alpha.js",
        "beta.js",
        TokenCheck::StructuralOnly,
    )
}

#[test]
fn typescript_type2_clone_has_structural_and_token_jaccard_of_one() -> Result<()> {
    assert_type2_clone(
        "typescript-small",
        12,
        "alpha.ts",
        "beta.ts",
        TokenCheck::Required,
    )
}

#[test]
fn tsx_type2_clone_has_structural_and_token_jaccard_of_one() -> Result<()> {
    assert_type2_clone(
        "tsx-small",
        10,
        "Card.tsx",
        "Tile.tsx",
        TokenCheck::Required,
    )
}

#[test]
fn javascript_near_miss_produces_cross_file_structural_cluster() -> Result<()> {
    assert_type3_clone("javascript-type3", 8, "delta.js", "epsilon.js")
}

#[test]
fn typescript_near_miss_produces_cross_file_structural_cluster() -> Result<()> {
    assert_type3_clone("typescript-type3", 8, "delta.ts", "epsilon.ts")
}

/// Asserts that a renamed Type-2 fixture has perfect structural and
/// optional token signals in its top-ranked cluster.
fn assert_type2_clone(
    fixture_name: &str,
    min_nodes: u32,
    left: &str,
    right: &str,
    token: TokenCheck,
) -> Result<()> {
    let report = run_report(&fixture(fixture_name), min_nodes)?;
    let top = top_cluster(&report, fixture_name)?;
    assert!(is_exact_one(signal(top, "structural")));
    if matches!(token, TokenCheck::Required) {
        assert!(is_exact_one(signal(top, "token_jaccard")));
    }
    assert!(spans_both(top, left, right));
    Ok(())
}

/// Whether a Type-2 assertion requires the `MinHash` layer to have paired
/// the top cluster in addition to the structural Merkle layer.
#[derive(Clone, Copy)]
enum TokenCheck {
    /// Structural identity is enough for this fixture.
    StructuralOnly,
    /// The fixture must also have exact token Jaccard.
    Required,
}

/// Asserts that a Type-3 near miss surfaces a cross-file shared
/// subtree cluster with a positive token signal.
fn assert_type3_clone(fixture_name: &str, min_nodes: u32, left: &str, right: &str) -> Result<()> {
    let report = run_report(&fixture(fixture_name), min_nodes)?;
    let Some(cluster) = clusters(&report)
        .iter()
        .find(|cluster| spans_both(cluster, left, right))
    else {
        anyhow::bail!("{fixture_name} must cluster {left} with {right}: {report:#}");
    };
    assert!(is_exact_one(signal(cluster, "structural")));
    assert!(signal(cluster, "token_jaccard") > 0.0);
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
