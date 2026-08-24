//! The committed `diff-scope` fixture's vocabulary ([CLI-ARG-DIFF],
//! [OUTPUT-SCHEMA-DIFF-TAGS], [METRICS-DIFF-SCOPE]).
//!
//! The fixture models the CI flow: the working directory is the
//! repository root, the scan root is its `repo/` subdirectory, and the
//! committed patches carry git-style `a/` / `b/` prefixes with
//! repo-root-relative paths. This module owns what the patch adds and
//! how to drive the CLI over it, so the reporting suite
//! (`diff_scoped_reporting.rs`) and the ingest suite
//! (`diff_scoped_ingest.rs`) cannot disagree about the fixture.

use std::path::PathBuf;

use anyhow::Context as _;
use assert_cmd::Command;
use serde_json::Value;

use super::{clusters, field, fixture, load_json, occurrences, Result};

/// New-side added-line spans the committed `change.patch` carries for
/// files inside the scan root, keyed by scan-root-relative path.
/// `docs/notes.md` is deliberately absent — it sits outside the root.
pub(crate) const ADDED_SPANS: &[(&str, u64, u64)] = &[
    ("src/caller.rs", 8, 21),
    ("src/fresh_a.rs", 1, 12),
    ("src/fresh_b.rs", 1, 12),
];

/// Total added lines inside the scan root: 14 in `caller.rs` plus 12 in
/// each fresh file. The 2 added lines of `docs/notes.md` must never be
/// counted ([METRICS-DIFF-SCOPE]).
pub(crate) const ADDED_LOC: u64 = 38;

/// Builds `deslop repo --output <out> --no-incremental <extra...>` with
/// the fixture root as the working directory, so diff paths resolve the
/// way they do in CI ([CLI-ARG-DIFF]).
pub(crate) fn diff_cmd(output_prefix: &std::path::Path, extra: &[&str]) -> Result<Command> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _ = cmd
        .current_dir(fixture("diff-scope"))
        .arg("repo")
        .arg("--output")
        .arg(output_prefix)
        .arg("--no-incremental")
        .args(extra);
    Ok(cmd)
}

/// Runs the CLI with `extra` flags into a fresh report prefix, asserts
/// the exit `code`, and returns the prefix (for `.json` / `.txt` /
/// `.html`), the stderr the run wrote, and the tempdir keeping both
/// alive.
pub(crate) fn run_code(extra: &[&str], code: i32) -> Result<(PathBuf, String, tempfile::TempDir)> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let assert = diff_cmd(&output, extra)?.assert().code(code);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    Ok((output, stderr, tmp))
}

/// Runs the CLI with `extra` flags, asserts success, and returns the
/// parsed JSON report plus the output prefix (for `.txt` / `.html`).
pub(crate) fn run_ok(extra: &[&str]) -> Result<(Value, PathBuf, tempfile::TempDir)> {
    let (output, _stderr, tmp) = run_code(extra, 0)?;
    let report = load_json(&output.with_extension("json"))?;
    Ok((report, output, tmp))
}

/// Finds the cluster whose occurrence paths exactly match `paths`
/// (order-insensitive), reporting the full cluster list otherwise.
pub(crate) fn cluster_with_paths<'a>(report: &'a Value, paths: &[&str]) -> Result<&'a Value> {
    let want: std::collections::BTreeSet<&str> = paths.iter().copied().collect();
    clusters(report)
        .iter()
        .find(|cluster| {
            let got: std::collections::BTreeSet<&str> = occurrences(cluster)
                .iter()
                .filter_map(|occ| occ.get("path").and_then(Value::as_str))
                .collect();
            got == want
        })
        .with_context(|| format!("no cluster with paths {paths:?} in {report:#}"))
}

/// The occurrence in `cluster` whose path is `path`.
pub(crate) fn occurrence_at<'a>(cluster: &'a Value, path: &str) -> Result<&'a Value> {
    occurrences(cluster)
        .iter()
        .find(|occ| occ.get("path").and_then(Value::as_str) == Some(path))
        .with_context(|| format!("no occurrence at {path} in {cluster:#}"))
}

/// `metrics` with the `diff` block removed, for mechanical-field
/// byte-identity comparisons across `--diff` on/off runs.
pub(crate) fn mechanical_metrics(report: &Value) -> Value {
    let mut metrics = field(report, "metrics").clone();
    if let Some(map) = metrics.as_object_mut() {
        let _ = map.remove("diff");
    }
    metrics
}

/// Sorted cluster-id set of a report.
pub(crate) fn id_set(report: &Value) -> std::collections::BTreeSet<String> {
    clusters(report)
        .iter()
        .filter_map(|cluster| cluster.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// True when 1-indexed `line` falls in any committed added span for
/// `path`.
pub(crate) fn in_added_span(path: &str, line: u64) -> bool {
    ADDED_SPANS
        .iter()
        .any(|(span_path, start, end)| *span_path == path && (*start..=*end).contains(&line))
}

/// Re-derives `duplicated_added_loc` from the report itself: the union,
/// per file, of non-hidden occurrence line ranges intersected with the
/// added spans, over clusters with >= 2 non-hidden occurrences — the
/// same projection as `duplicated_loc` ([METRICS-DIFF-SCOPE]).
pub(crate) fn rederive_duplicated_added(report: &Value) -> Result<u64> {
    let mut per_file: std::collections::BTreeMap<&str, std::collections::BTreeSet<u64>> =
        std::collections::BTreeMap::new();
    for cluster in clusters(report) {
        let visible: Vec<&Value> = occurrences(cluster)
            .iter()
            .filter(|occ| occ.get("hidden").and_then(Value::as_bool) == Some(false))
            .collect();
        if visible.len() < 2 {
            continue;
        }
        for occ in visible {
            let path = occ
                .get("path")
                .and_then(Value::as_str)
                .context("occurrence path")?;
            let start = occ
                .get("start_line")
                .and_then(Value::as_u64)
                .context("start_line")?;
            let end = occ
                .get("end_line")
                .and_then(Value::as_u64)
                .context("end_line")?;
            for line in start..=end {
                if in_added_span(path, line) {
                    let _ = per_file.entry(path).or_default().insert(line);
                }
            }
        }
    }
    per_file
        .values()
        .map(|lines| u64::try_from(lines.len()).context("line count fits u64"))
        .sum()
}
