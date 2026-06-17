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

use std::{fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

/// Path to the `deslop-core` test directory — the actual source of the
/// false positive reported in issue #58.
fn deslop_core_tests() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("deslop-core")
        .join("tests")
}

fn run_report(tmp: &Path, scan_root: &Path) -> Result<Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(scan_root)
        .args(["--embeddings", "off", "--output"])
        .arg(tmp.join("report"))
        .assert()
        .success();
    let json_path = tmp.join("report.json");
    let body = fs::read_to_string(&json_path)?;
    Ok(serde_json::from_str(&body)?)
}

fn cluster_bucket(cluster: &Value) -> &str {
    cluster.get("bucket").and_then(Value::as_str).unwrap_or("?")
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

// Issue #58: test files that share only boilerplate (assert macros, function
// signatures, helper patterns) must not surface as `loosely_similar` top
// offenders. The fused gate lets token-only matches through because the
// additive formula allows token_jaccard alone to clear the threshold. The
// fix excludes LooselySimilar clusters from the ranked report so users are
// never misled by boilerplate-only noise at position #1.
#[test]
fn rust_test_boilerplate_files_never_surface_as_loosely_similar() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = deslop_core_tests();
    let report = run_report(tmp.path(), &scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|c| cluster_bucket(c) == "loosely_similar")
        .filter(|c| cluster_paths(c).len() >= 2)
        .map(|c| {
            let paths = cluster_paths(c);
            let structural = c
                .pointer("/signals/structural")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let token_j = c
                .pointer("/signals/token_jaccard")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            format!(
                "loosely_similar cluster spans {paths:?} (structural={structural:.3}, \
                 token_jaccard={token_j:.3})"
            )
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "test-boilerplate-only matches must not appear as loosely_similar top offenders \
         (issue #58). Offending clusters: {offenders:#?}"
    );
    Ok(())
}
