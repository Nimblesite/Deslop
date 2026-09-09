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

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

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

const CALL_SCAFFOLD_FIXTURE: &str = "rust-call-scaffolding";
const CALL_SCAFFOLD_MIN_NODES: u32 = 30;
const CONTROL_FILES: [&str; 2] = ["control_first.rs", "control_second.rs"];
const CONTROL_GROUP_COUNT: usize = 1;
const CONTROL_OCCURRENCE_COUNT: u64 = 2;
const CONTROL_RANK: u64 = 1;
const CONTROL_NODE_COUNT: u64 = 71;
const CONTROL_FIRST_LINE: u64 = 1;
const CONTROL_LAST_LINE: u64 = 13;
const CONTROL_KIND: &str = "identical";

// [CLONE-NOISE-LITERAL-VARIATION-CALLS] The registry only feeds varying
// path payloads to register; the independent authored clone must survive.
#[test]
fn registry_call_payload_variation_keeps_only_authored_control() -> Result<()> {
    let report = crate::common::run_report(
        &crate::common::fixture(CALL_SCAFFOLD_FIXTURE),
        CALL_SCAFFOLD_MIN_NODES,
    )?;
    let clusters = report["clusters"]
        .as_array()
        .expect("rendered cluster array");
    let control = clusters
        .iter()
        .find(|cluster| cluster_paths(cluster).contains(&CONTROL_FILES[0].to_owned()))
        .expect("the independently authored positive control must be reported");
    assert_authored_control(control);
    assert_only_authored_control(clusters);
    Ok(())
}

fn assert_authored_control(control: &Value) {
    assert_eq!(control["rank"].as_u64(), Some(CONTROL_RANK));
    assert_eq!(control["kind"].as_str(), Some(CONTROL_KIND));
    assert_eq!(
        control["occurrence_count"].as_u64(),
        Some(CONTROL_OCCURRENCE_COUNT)
    );
    assert_eq!(
        control["canonical_node_count"].as_u64(),
        Some(CONTROL_NODE_COUNT)
    );
    assert_eq!(control["mass"].as_u64(), Some(CONTROL_NODE_COUNT));
    assert_eq!(cluster_paths(control), CONTROL_FILES);
    assert_control_occurrences(control);
}

fn assert_control_occurrences(control: &Value) {
    for occurrence in control["occurrences"]
        .as_array()
        .expect("control occurrences")
    {
        assert_eq!(occurrence["hidden"].as_bool(), Some(false));
        assert_eq!(occurrence["start_line"].as_u64(), Some(CONTROL_FIRST_LINE));
        assert_eq!(occurrence["end_line"].as_u64(), Some(CONTROL_LAST_LINE));
    }
}

fn assert_only_authored_control(clusters: &[Value]) {
    let reported_paths: Vec<String> = clusters.iter().flat_map(cluster_paths).collect();
    assert_eq!(
        reported_paths, CONTROL_FILES,
        "registry setup only varies path payloads; it is call scaffolding, not a clone"
    );
    assert_eq!(clusters.len(), CONTROL_GROUP_COUNT);
}
