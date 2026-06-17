//! E2E regression for GH #150: `pub(crate) mod e0001;` /
//! `pub use foo::bar;` top-level declarations cluster across registries
//! because Rust requires literal module statements. These are scaffolding,
//! not actionable duplication, and must never surface in the report.
//! Spec: [CLONE-NOISE-RUST-DECL].

use std::fs;

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

fn run_report(fixture_name: &str) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = deslop_cmd(&fixture(fixture_name), &output)?;
    let _assertion = cmd
        .args(["--min-nodes", "3", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn cluster_count(report: &Value) -> usize {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

#[test]
fn rust_mod_and_use_declarations_do_not_cluster_as_duplicates() -> Result<()> {
    let report = run_report("rust-issue-150-mod-declarations")?;
    let count = cluster_count(&report);
    assert_eq!(
        count, 0,
        "pub(crate) mod / pub use declarations are language scaffolding \
         and must not surface as duplicate clusters: {report:#}"
    );
    Ok(())
}
