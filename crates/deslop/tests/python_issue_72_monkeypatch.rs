//! E2E regression for GH #72: `monkeypatch.setenv` setup patterns across
//! config tests are scaffolding, not duplicate logic.

use std::fs;

use anyhow::Result;
use assert_cmd::Command;

mod common;
use crate::common::*;

fn run_report(fixture_name: &str) -> Result<serde_json::Value> {
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
