//! Shared helpers for the standalone (non-`cli`) end-to-end regression
//! tests. Each `tests/<name>.rs` integration binary is its own crate, so
//! this module is pulled in with `mod common;` and used through
//! `use crate::common::*;`. It centralises the fixture-path lookup, the
//! `deslop` invocation, and the report-walking helpers that every
//! per-issue false-positive test would otherwise copy verbatim.

use std::{fs, path::Path, path::PathBuf};

use anyhow::anyhow;
pub(crate) use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

/// Absolute path to the named directory under `tests/fixtures`.
pub(crate) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Runs `deslop <scan_root> --min-nodes <min_nodes> --embeddings off` into
/// a throwaway temp dir and returns the parsed JSON report. Asserts the
/// process exits successfully before the report is read.
pub(crate) fn run_report(scan_root: &Path, min_nodes: u32) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(scan_root)
        .arg("--min-nodes")
        .arg(min_nodes.to_string())
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(&output)
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

/// The `clusters` array of a report, or an empty slice when absent.
pub(crate) fn clusters(report: &Value) -> &[Value] {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The `occurrences` array of a cluster, or an empty slice when absent.
pub(crate) fn occurrences(cluster: &Value) -> &[Value] {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Reads the source slice an occurrence points at, resolving its `path`
/// relative to `scan_root` and slicing `[start_byte, end_byte)`.
pub(crate) fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
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

/// Reads a `usize` byte-offset field (`start_byte` / `end_byte`) off an
/// occurrence.
pub(crate) fn occurrence_byte(occurrence: &Value, field: &str) -> Result<usize> {
    occurrence
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("reported occurrence is missing {field}"))
}

/// Collapses every reported occurrence whose source text satisfies
/// `predicate` into a one-line summary. Tests assert the returned list is
/// empty to prove a benign pattern was not surfaced as a duplicate.
pub(crate) fn summaries_where(
    report: &Value,
    scan_root: &Path,
    predicate: impl Fn(&str) -> bool,
) -> Result<Vec<String>> {
    let mut summaries = Vec::new();
    for cluster in clusters(report) {
        for occurrence in occurrences(cluster) {
            let text = occurrence_text(scan_root, occurrence)?;
            if predicate(&text) {
                summaries.push(text.replace('\n', " "));
            }
        }
    }
    Ok(summaries)
}
