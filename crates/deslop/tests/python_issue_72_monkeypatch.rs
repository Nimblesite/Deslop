//! E2E regression for GH #72: `monkeypatch.setenv` setup patterns across
//! config tests are scaffolding, not duplicate logic.
//! Tests [CLONE-NOISE-PY-MONKEYPATCH]

use std::fs;

use anyhow::Result;

mod common;
use crate::common::*;

fn run_report(fixture_name: &str) -> Result<serde_json::Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(&fixture(fixture_name), &output)?
        .args(["--min-nodes", "4", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn cluster_count(report: &serde_json::Value) -> usize {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

#[test]
fn monkeypatch_setenv_setup_pattern_is_not_duplicate_code() -> Result<()> {
    let report = run_report("python-issue-72-monkeypatch-setenv")?;
    let count = cluster_count(&report);
    assert_eq!(
        count, 0,
        "monkeypatch.setenv scaffolding must not produce duplicate clusters: {report:#}"
    );
    Ok(())
}
