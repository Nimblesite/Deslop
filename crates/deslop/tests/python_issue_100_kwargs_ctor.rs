//! E2E regression for GH #100 [CLONE-NOISE-PY-KWARGS-CTOR].
//!
//! ORM / dataclass / Pydantic model constructors with kwargs-only field
//! lists are bounded by the model's required columns. Two constructors
//! sharing the same arity but distinct keyword names cannot share a
//! refactor — extraction would collapse the per-model field contract.
//! The cluster filter must drop those clusters from the rendered report.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(scan_root)
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
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn occurrences(cluster: &Value) -> &[Value] {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
    let path = occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("reported occurrence is missing path"))?;
    let source = fs::read_to_string(scan_root.join(path))?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    source
        .get(start..end)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("reported occurrence range is invalid"))
}

fn occurrence_byte(occurrence: &Value, field: &str) -> Result<usize> {
    occurrence
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("reported occurrence is missing {field}"))
}

fn cluster_summaries_mentioning(
    report: &Value,
    scan_root: &Path,
    needle: &str,
) -> Result<Vec<String>> {
    let mut summaries = Vec::new();
    for cluster in clusters(report) {
        for occurrence in occurrences(cluster) {
            let text = occurrence_text(scan_root, occurrence)?;
            if text.contains(needle) {
                summaries.push(text.replace('\n', " "));
            }
        }
    }
    Ok(summaries)
}

#[test]
fn message_vs_agentlog_kwargs_constructors_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-100-kwargs-ctor");
    let report = run_report(&scan_root)?;
    let message_hits = cluster_summaries_mentioning(&report, &scan_root, "Message(")?;
    let agent_log_hits = cluster_summaries_mentioning(&report, &scan_root, "AgentLog(")?;
    assert!(
        message_hits.is_empty(),
        "Message(...) constructor calls must not surface as duplicates: {message_hits:#?}"
    );
    assert!(
        agent_log_hits.is_empty(),
        "AgentLog(...) constructor calls must not surface as duplicates: {agent_log_hits:#?}"
    );
    Ok(())
}
