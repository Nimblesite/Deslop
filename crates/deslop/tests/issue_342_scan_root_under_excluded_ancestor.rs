//! Scan roots nested under a directory named like a built-in exclude
//! component (#342, `[CONFIG-EXCLUDE-BUILTIN]`).
//!
//! `built_in_excluded` (`crates/deslop-core/src/config.rs`) tests **every**
//! component of the absolute discovered path against
//! `BUILTIN_EXCLUDE_COMPONENTS`, including components *above* the scan
//! root. A repository that merely lives under a folder named `dist`,
//! `build`, `target`, `vendor` or `node_modules` therefore analyses as zero
//! files: `files_analysed: 0`, `clusters: []`, exit code success. That is a
//! total, silent false negative — the user scans their repo, is told it has
//! no duplication, and nothing anywhere signals that nothing was read.
//!
//! The user explicitly chose the scan root; its ancestors are not part of
//! the analysed corpus. The sibling path already encodes exactly this
//! principle — `is_report_hidden` passes `self.scan_root` down to
//! `built_in_report_hidden` so the root is exempt — while `built_in_excluded`
//! takes no scan root at all and so has no equivalent carve-out.
//!
//! The corpus is seeded from the `ts-type1-identical` fixture at the same
//! `min-nodes` its own bucket test uses, so these assertions fail for the
//! ancestry defect alone and not because a hand-rolled snippet stopped
//! clustering.

use anyhow::Result;
use std::path::Path;

use serde_json::Value;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
use crate::common::*;

/// The two byte-identical files seeded into every scan root below.
const CLONE_FILES: [&str; 2] = ["tax_alpha.ts", "tax_beta.ts"];

/// `min-nodes` for the seeded fixture, matching
/// `typescript_byte_identical_pair_is_identical_bucket`.
const MIN_NODES: u32 = 12;

/// Built-in exclude components that plausibly name a directory a real
/// checkout sits beneath: `~/dist/…`, a Chromium-style `out`/`build`
/// workspace, a Go `vendor` tree, a JS monorepo under `node_modules`.
const ANCESTOR_NAMES: [&str; 5] = ["dist", "build", "target", "vendor", "node_modules"];

/// Seeds the identical-pair fixture into
/// `<tmp>/<ancestors…>/innocent-repo/src` and returns the report for that
/// scan root. An empty `ancestors` gives the control root, whose path
/// contains no built-in exclude component.
fn report_under(ancestors: &[&str]) -> Result<(tempfile::TempDir, std::path::PathBuf, Value)> {
    let tmp = tempfile::tempdir()?;
    let root = ancestors
        .iter()
        .fold(tmp.path().to_path_buf(), |path, name| path.join(name))
        .join("innocent-repo")
        .join("src");
    seed(&fixture("ts-type1-identical"), &root)?;
    let report = run_report(&root, MIN_NODES)?;
    Ok((tmp, root, report))
}

/// Asserts `report` contains the seeded clone pair in full — both files
/// read, a cluster spanning them, and the byte-proven verbatim fact that
/// pair earns from any other scan root ([PIPELINE-CLUSTER-CLOSURE]).
fn assert_clone_reported(scan_root: &Path, report: &Value, label: &str) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(2),
        "{label}: both seeded files must be analysed: {report:#}"
    );
    assert!(
        !clusters(report).is_empty(),
        "{label}: the seeded clone must be reported: {report:#}"
    );
    let clone = expect_cluster_spanning(report, &CLONE_FILES)?;
    assert!(
        has_verbatim_pair(scan_root, clone)?,
        "{label}: a byte-identical pair must be byte-proven from the seeded \
         source: {report:#}"
    );
    assert_structural_only_contract(clone, label);
    assert_no_pair_surface_on_cluster(clone, label);
    Ok(())
}

#[test]
fn a_scan_root_with_no_excluded_ancestor_reports_the_seeded_clone() -> Result<()> {
    // The control. If this fails the fixture or the harness moved, and the
    // ancestry assertions below would be vacuous rather than wrong.
    let (_tmp, root, report) = report_under(&[])?;
    assert_clone_reported(&root, &report, "control (no excluded ancestor)")
}

#[test]
fn a_repo_under_an_excluded_ancestor_still_reports_its_duplicates() -> Result<()> {
    for ancestor in ANCESTOR_NAMES {
        let (_tmp, root, report) = report_under(&[ancestor])?;
        assert_clone_reported(&root, &report, ancestor)?;
    }
    Ok(())
}

#[test]
fn nested_excluded_ancestors_are_ignored_to_any_depth() -> Result<()> {
    let (_tmp, root, report) = report_under(&["build", "dist"])?;
    assert_clone_reported(&root, &report, "build/dist")?;
    let (_tmp, root, report) = report_under(&["node_modules", "pkg", "target"])?;
    assert_clone_reported(&root, &report, "node_modules/pkg/target")
}

/// Asserts no rendered occurrence sits under a `node_modules` component.
/// Split on both separators so the assertion holds on Windows runners.
fn assert_no_dependency_leaked(report: &Value) {
    for cluster in clusters(report) {
        for path in occurrence_paths(cluster) {
            assert!(
                !path.split(['/', '\\']).any(|part| part == "node_modules"),
                "a dependency tree inside the scan root leaked into the report: {path}",
            );
        }
    }
}

#[test]
fn a_dependency_tree_inside_the_scan_root_is_still_excluded() -> Result<()> {
    // The other direction, and the one a careless fix breaks: ignoring
    // ancestors must not also stop excluding a real dependency tree
    // *below* the root. Both corpora are the same fixture, so a leak
    // shows up as four analysed files rather than two.
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("dist").join("innocent-repo").join("src");
    seed(&fixture("ts-type1-identical"), &root)?;
    seed(
        &fixture("ts-type1-identical"),
        &root.join("node_modules").join("pkg"),
    )?;
    let report = run_report(&root, MIN_NODES)?;
    assert_clone_reported(&root, &report, "excluded ancestor with inner node_modules")?;
    assert_no_dependency_leaked(&report);
    Ok(())
}

#[test]
fn a_scan_root_named_like_an_excluded_component_is_analysed() -> Result<()> {
    // Pointing deslop at a directory *is* the request to analyse it. The
    // root's own name is therefore never grounds to exclude its contents,
    // exactly as `scan_root_contains_component_pair` already exempts a
    // root that sits inside a hidden component pair.
    for name in ANCESTOR_NAMES {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path().join(name);
        seed(&fixture("ts-type1-identical"), &root)?;
        let report = run_report(&root, MIN_NODES)?;
        assert_clone_reported(&root, &report, name)?;
    }
    Ok(())
}

#[test]
fn the_ancestor_directory_name_cannot_change_the_report() -> Result<()> {
    // The equivalence is the load-bearing assertion: a single-root test
    // would still pass if exclusion silently dropped one of the two files,
    // and asserting the whole report is invariant to a directory name the
    // user never asked deslop to reason about is what stops a future
    // addition to BUILTIN_EXCLUDE_COMPONENTS from reintroducing this.
    let (_tmp, _, control) = report_under(&[])?;
    for ancestor in ANCESTOR_NAMES {
        let (_tmp, _, nested) = report_under(&[ancestor])?;
        assert_eq!(
            field(&nested, "files_analysed"),
            field(&control, "files_analysed"),
            "{ancestor}: files analysed must not depend on the ancestor name: {nested:#}"
        );
        assert_eq!(
            cluster_count(&nested),
            cluster_count(&control),
            "{ancestor}: cluster count must not depend on the ancestor name: {nested:#}"
        );
        assert_eq!(
            metric_field(&nested, "duplication_percent"),
            metric_field(&control, "duplication_percent"),
            "{ancestor}: duplication_percent must not depend on the ancestor name: {nested:#}"
        );
    }
    Ok(())
}
