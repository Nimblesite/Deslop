//! E2E regression for GH #147: `xs.iter().map(|x| x.field.as_str()).collect()`
//! is a pure Rust language idiom that clusters across unrelated element
//! types. Extracting it would require a trait on 10+ unrelated structs.
//! The cluster must not surface as actionable duplication.
//! Spec: [CLONE-NOISE-RUST-ITER-COLLECT].

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_report(fixture_name: &str) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(fixture(fixture_name))
        .arg("--min-nodes")
        .arg("4")
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(&output)
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn clusters(report: &Value) -> &[Value] {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

fn cluster_occurrence_paths(cluster: &Value) -> Vec<String> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(|occurrence| {
                    occurrence
                        .get("path")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
}

#[test]
fn rust_iter_map_collect_idiom_does_not_cluster_across_unrelated_types() -> Result<()> {
    let report = run_report("rust-issue-147-iter-collect-idiom")?;
    let cross_file_clusters: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| {
            let paths = cluster_occurrence_paths(cluster);
            let distinct: std::collections::BTreeSet<&String> = paths.iter().collect();
            distinct.len() >= 2
        })
        .collect();
    assert!(
        cross_file_clusters.is_empty(),
        "the `.iter().map(|x| x.field.method()).collect()` idiom must not \
         cluster across unrelated element types: {report:#}"
    );
    Ok(())
}
