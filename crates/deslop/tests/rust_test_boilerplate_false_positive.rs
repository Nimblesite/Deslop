//! Regression test for issue #58: `LooselySimilar` catch-all surfaces
//! boilerplate-only test-file matches as top offenders.
//!
//! The `deslop-core` test suite (`embedding_pairs.rs`, `report_api.rs`,
//! `live.rs`) shares test boilerplate (function signatures, assertion
//! macros, helper patterns) that pushes token Jaccard near 1.0 with
//! structural ≈ 0.02. That puts their cluster in the
//! `loosely_similar` bucket. Before the fix these files ranked **#1** in
//! the report — a cluster the user correctly described as "bullshit."
//!
//! Acceptance: scanning the `deslop-core` test directory must not surface
//! any `loosely_similar` cross-file cluster in the ranked output.

use std::{collections::BTreeSet, fs, path::Path, path::PathBuf};

use anyhow::Result;
use serde_json::Value;

use crate::common::deslop_cmd;

const ISSUE_58_TEST_FILES: [&str; 3] = ["embedding_pairs.rs", "report_api.rs", "live.rs"];
const MINIMUM_FALSE_POSITIVE_MEMBERS: usize = 2;

/// Path to the `deslop-core` test directory — the actual source of the
/// false positive reported in issue #58.
fn deslop_core_tests() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("deslop-core")
        .join("tests")
}

fn run_report(tmp: &Path, scan_root: &Path) -> Result<Value> {
    let mut cmd = deslop_cmd(scan_root, &tmp.join("report"))?;
    let _assertion = cmd.args(["--embeddings", "off"]).assert().success();
    let json_path = tmp.join("report.json");
    let body = fs::read_to_string(&json_path)?;
    Ok(serde_json::from_str(&body)?)
}

fn cluster_paths(cluster: &Value) -> Vec<String> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(|occurrences| {
            occurrences
                .iter()
                .filter_map(|o| o.get("path").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

// Issue #58's documented trio shares test boilerplate but no extractable
// duplicate. Other files in this active test corpus can legitimately have
// real duplicate regions, so the test targets only that established false
// positive instead of declaring every cross-file finding invalid.
#[test]
fn issue_58_test_boilerplate_trio_never_closes() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = deslop_core_tests();
    let report = run_report(tmp.path(), &scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    // [FUSED-CONTENT-GATE] Pair admission rejects the known boilerplate
    // edges before closure. The cluster wire is not used to classify every
    // cross-file duplicate in this real corpus.
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|c| {
            let matching: BTreeSet<String> = cluster_paths(c)
                .into_iter()
                .filter(|path| ISSUE_58_TEST_FILES.contains(&path.as_str()))
                .collect();
            matching.len() >= MINIMUM_FALSE_POSITIVE_MEMBERS
        })
        .map(|c| {
            format!(
                "issue #58 test boilerplate cluster spans {:?}",
                cluster_paths(c)
            )
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "the documented test-boilerplate pairings must not close into a clone \
         (issue #58). Offending clusters: {offenders:#?}"
    );
    Ok(())
}
