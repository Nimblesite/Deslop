//! End-to-end regression coverage for issue #44: unrelated C# xUnit
//! test classes get bucketed as `Nearly identical code` because the
//! third disjunct of [`buckets::classify_signals`] paints LSH-only
//! pairs (`structural <= 0.01 && token_jaccard >= 0.90`) as Type-3
//! near-misses. C#'s grammar saturates the kind-gram alphabet on
//! xUnit scaffolding (`using_directive`, `attribute_list`,
//! `method_declaration`, `await_expression`, `__ident__`,
//! `__literal__`, …), so two completely unrelated test classes reach
//! kind-gram Jaccard ≈ 1.0 with zero structural overlap.
//!
//! Acceptance from the issue: a fixture containing 3+ unrelated C#
//! xUnit test classes must produce **zero** `nearly_identical`
//! cross-class clusters. Type-3 should require some structural
//! anchor; LSH-only matches with `structural ≈ 0` belong in
//! `loosely_similar`, not `nearly_identical`.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use assert_cmd::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_report(tmp: &Path, scan_root: &Path) -> Result<serde_json::Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(scan_root)
        .arg("--min-nodes")
        .arg("30")
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(tmp.join("report"))
        .assert()
        .success();
    let mut json_path = tmp.join("report");
    let _replaced = json_path.set_extension("json");
    let body = fs::read_to_string(&json_path)?;
    Ok(serde_json::from_str(&body)?)
}

fn cluster_paths(cluster: &serde_json::Value) -> BTreeSet<String> {
    cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .map(|occurrences| {
            occurrences
                .iter()
                .filter_map(|occurrence| {
                    occurrence
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn cluster_id(cluster: &serde_json::Value) -> String {
    cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?")
        .to_owned()
}

fn cluster_bucket(cluster: &serde_json::Value) -> String {
    cluster
        .get("bucket")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?")
        .to_owned()
}

// Issue #44 acceptance: unrelated C# xUnit test classes must not
// merge into a single "Nearly identical code" cluster. Three
// completely unrelated test files share only generic xUnit
// scaffolding kinds; LSH-only kind-gram Jaccard saturation must not
// route the resulting pair into the [CLONE-BUCKETS] `NearlyIdentical`
// bucket. They may legitimately surface as `LooselySimilar` (LSH-only
// hint) but never as `NearlyIdentical`.
#[test]
fn unrelated_csharp_xunit_classes_are_never_nearly_identical() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-unrelated-xunit-tests");
    let report = run_report(tmp.path(), &scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "nearly_identical")
        .filter(|cluster| cluster_paths(cluster).len() >= 2)
        .map(|cluster| {
            let id = cluster_id(cluster);
            let paths: Vec<String> = cluster_paths(cluster).into_iter().collect();
            format!("cluster {id} spans {paths:?}")
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "unrelated C# xUnit test classes must not form a 'Nearly identical' \
         cross-class cluster (issue #44). Offending clusters: {offenders:#?}"
    );
    Ok(())
}
