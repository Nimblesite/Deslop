//! Shared helpers for the standalone (non-`cli`) end-to-end regression
//! tests. Each `tests/<name>.rs` integration binary is its own crate, so
//! this module is pulled in with `mod common;` and used through
//! `use crate::common::*;`. It centralises the fixture-path lookup, the
//! `deslop` invocation, and the report-walking helpers that every
//! per-issue false-positive test would otherwise copy verbatim.
//!
//! Each integration binary pulls in only the subset of helpers it needs,
//! so the unused-symbol lint is silenced for this shared module (matching
//! the `deslop-core` and `deslop-mcp` test commons).

#![allow(dead_code)]

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

/// Builds `deslop <scan_root> --output <output_prefix>`, ready for the
/// caller to append scenario-specific flags before `.assert()`. Centralises
/// the `cargo_bin` lookup + scan-root + output-prefix prefix that every
/// invocation shares.
pub(crate) fn deslop_cmd(scan_root: &Path, output_prefix: &Path) -> Result<Command> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _cmd = cmd.arg(scan_root).arg("--output").arg(output_prefix);
    Ok(cmd)
}

/// Runs `deslop <scan_root> <extra_args...> --output <tmp>/report` into a
/// throwaway temp dir and returns the parsed JSON report after asserting the
/// process exits successfully. The flexible-flag core behind [`run_report`].
pub(crate) fn run_report_args(scan_root: &Path, extra_args: &[&str]) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = deslop_cmd(scan_root, &output)?;
    let _assertion = cmd.args(extra_args).assert().success();
    load_json(&output.with_extension("json"))
}

/// Runs `deslop <scan_root> --min-nodes <min_nodes> --embeddings off` into
/// a throwaway temp dir and returns the parsed JSON report. Asserts the
/// process exits successfully before the report is read.
pub(crate) fn run_report(scan_root: &Path, min_nodes: u32) -> Result<Value> {
    let min_nodes = min_nodes.to_string();
    run_report_args(
        scan_root,
        &["--min-nodes", min_nodes.as_str(), "--embeddings", "off"],
    )
}

/// Parses the JSON document at `path` into a [`Value`].
pub(crate) fn load_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

/// Reads a named field off `value`, returning [`Value::Null`] when absent so
/// callers get a deterministic `!=` instead of a panic.
pub(crate) fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value.get(name).unwrap_or(&Value::Null)
}

/// Length of a named array-valued field, or `0` when missing / non-array (so
/// the assertion trips with the full JSON printed rather than panicking).
pub(crate) fn array_len(value: &Value, name: &str) -> usize {
    field(value, name).as_array().map_or(0, Vec::len)
}

/// Copies every top-level entry in `src` into a freshly created `dst`. Used
/// by tests that need a mutable scan root seeded from an immutable fixture.
pub(crate) fn seed(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
    }
    Ok(())
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
