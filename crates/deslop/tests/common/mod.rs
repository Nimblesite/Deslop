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

/// Fused-signal bands and assertion vocabulary ([FUSED-THRESHOLD]).
pub(crate) mod signals;
pub(crate) use signals::*;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

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

/// Builds `deslop <scan_root> --output <output_prefix> --no-incremental`,
/// ready for the caller to append scenario-specific flags before
/// `.assert()`. Centralises the `cargo_bin` lookup + scan-root +
/// output-prefix prefix that every invocation shares.
///
/// `--no-incremental` is part of the shared prefix because the fingerprint
/// cache is on by default ([PIPELINE-INCREMENTAL]) and always lands in the
/// *scan root* ([OUTPUT-DIR]) — `--output` cannot redirect it. These
/// binaries assert on detection output, and most point straight at a
/// checked-in `tests/fixtures/` directory, so without the opt-out every
/// run would write into the fixture tree and concurrent binaries sharing a
/// fixture would race on the same cache. Cache behaviour itself is covered
/// in `tests/cli/cache_and_debug.rs`, against scan roots it owns.
pub(crate) fn deslop_cmd(scan_root: &Path, output_prefix: &Path) -> Result<Command> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _cmd = cmd
        .arg(scan_root)
        .arg("--output")
        .arg(output_prefix)
        .arg("--no-incremental");
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
        // Source files only. A fixture directory can pick up a nested
        // `.deslop/` output directory ([OUTPUT-DIR]) from a stray run, and
        // `fs::copy` on a directory fails outright — seeding must not be
        // hostage to whatever else happens to be sitting there.
        if !entry.file_type()?.is_file() {
            continue;
        }
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

/// A cluster's `bucket` label (e.g. `identical`), or `"?"` when absent.
pub(crate) fn cluster_bucket(cluster: &Value) -> &str {
    field(cluster, "bucket").as_str().unwrap_or("?")
}

/// A cluster's stable `id`, or `"?"` when absent.
pub(crate) fn cluster_id(cluster: &Value) -> &str {
    field(cluster, "id").as_str().unwrap_or("?")
}

/// A cluster's `size` (its occurrence count), or `0` when absent.
pub(crate) fn cluster_size(cluster: &Value) -> u64 {
    field(cluster, "size").as_u64().unwrap_or(0)
}

/// One component of a cluster's fused `signals` block (e.g. `token_jaccard`),
/// or `0.0` when the named signal is absent.
pub(crate) fn signal(cluster: &Value, key: &str) -> f64 {
    cluster
        .pointer(&format!("/signals/{key}"))
        .and_then(Value::as_f64)
        .unwrap_or_default()
}

/// Number of clusters the report rendered — the [METRICS-REPO] visible set.
pub(crate) fn cluster_count(report: &Value) -> usize {
    array_len(report, "clusters")
}

/// True when `value` lies within `1e-9` of `target`. Lets a test pin an
/// exact fused-signal value (typically `0.0` or `1.0`) without tripping
/// the float-equality lint or carrying its own epsilon.
pub(crate) fn approx(value: f64, target: f64) -> bool {
    (value - target).abs() <= 1e-9
}

/// The sorted, de-duplicated bare file names a cluster's occurrences span.
pub(crate) fn cluster_file_set(cluster: &Value) -> BTreeSet<String> {
    occurrence_files(cluster).into_iter().collect()
}

/// The first cluster whose occurrences cover every name in `files`, or
/// `None`. Lets a test target the specific cross-file clone it cares about
/// regardless of report ordering or unrelated clusters.
pub(crate) fn cluster_spanning<'a>(report: &'a Value, files: &[&str]) -> Option<&'a Value> {
    clusters(report).iter().find(|cluster| {
        files
            .iter()
            .all(|name| cluster_file_set(cluster).contains(*name))
    })
}

/// Like [`cluster_spanning`] but fails the test with the full report when
/// no matching cluster exists, so the failure message is actionable.
pub(crate) fn expect_cluster_spanning<'a>(report: &'a Value, files: &[&str]) -> Result<&'a Value> {
    cluster_spanning(report, files)
        .ok_or_else(|| anyhow!("expected a clone spanning {files:?}: {report:#}"))
}

/// Drives `deslop` over `fixture_dir` at `min_nodes` and asserts the
/// cross-file clone spanning `files` exists with `structural == 1.0`, the
/// expected `bucket`, and the token signal that bucket implies: a full
/// token signal for `identical` / `nearly_identical`, a near-zero one for
/// the structural-only routing (#134). Centralises the renamed-clone
/// assertion every per-feature E2E test would otherwise repeat.
pub(crate) fn assert_bucketed_clone(
    fixture_dir: &str,
    min_nodes: u32,
    files: &[&str],
    bucket: &str,
) -> Result<()> {
    let report = run_report(&fixture(fixture_dir), min_nodes)?;
    let clone = expect_cluster_spanning(&report, files)?;
    assert_eq!(
        cluster_bucket(clone),
        bucket,
        "{fixture_dir} clone bucket mismatch: {report:#}"
    );
    assert!(
        approx(signal(clone, "structural"), 1.0),
        "{fixture_dir} renamed clone must reach structural identity: {report:#}"
    );
    if bucket == "structural_only" {
        assert!(
            signal(clone, "token_jaccard") < 0.05,
            "{fixture_dir} structural_only routing needs a near-zero token signal: {report:#}"
        );
    } else {
        assert!(
            approx(signal(clone, "token_jaccard"), 1.0),
            "{fixture_dir} token layer must also be rename-invariant: {report:#}"
        );
    }
    Ok(())
}

/// The report's `clusters_hidden` count (suppressed-cluster telemetry), or
/// `0` when absent.
pub(crate) fn clusters_hidden(report: &Value) -> u64 {
    field(report, "clusters_hidden").as_u64().unwrap_or(0)
}

/// The relative path of a single occurrence, erroring when absent.
pub(crate) fn occurrence_path(occurrence: &Value) -> Result<&str> {
    field(occurrence, "path")
        .as_str()
        .ok_or_else(|| anyhow!("reported occurrence is missing path"))
}

/// The relative paths of every occurrence in `cluster`, in report order.
pub(crate) fn occurrence_paths(cluster: &Value) -> Vec<String> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| field(occurrence, "path").as_str().map(ToOwned::to_owned))
        .collect()
}

/// The bare file names (no directory) of every occurrence in `cluster`.
pub(crate) fn occurrence_files(cluster: &Value) -> Vec<String> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| {
            field(occurrence, "path")
                .as_str()
                .and_then(|path| Path::new(path).file_name())
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect()
}

/// The source slice each occurrence in `cluster` points at, in report order.
pub(crate) fn occurrence_texts(scan_root: &Path, cluster: &Value) -> Result<Vec<String>> {
    occurrences(cluster)
        .iter()
        .map(|occurrence| occurrence_text(scan_root, occurrence))
        .collect()
}

/// Reads the source slice an occurrence points at, resolving its `path`
/// relative to `scan_root` and slicing `[start_byte, end_byte)`.
pub(crate) fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
    let path = occurrence_path(occurrence)?;
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

/// Reads a scalar field off the report's `metrics` block ([METRICS-REPO]).
pub(crate) fn metric_field<'a>(report: &'a Value, name: &str) -> &'a Value {
    field(field(report, "metrics"), name)
}

/// `metrics.per_file` rows, or an empty slice when absent.
pub(crate) fn per_file_metrics(report: &Value) -> &[Value] {
    metric_field(report, "per_file")
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Count of a cluster's non-hidden (live) occurrences.
pub(crate) fn live_occurrences(cluster: &Value) -> usize {
    occurrences(cluster)
        .iter()
        .filter(|occurrence| !field(occurrence, "hidden").as_bool().unwrap_or(false))
        .count()
}

/// Line-set cardinality as the `u64` the wire metric uses.
pub(crate) fn line_count(lines: &BTreeSet<u64>) -> u64 {
    u64::try_from(lines.len()).unwrap_or(u64::MAX)
}

/// Per-file set of line numbers covered by the non-hidden occurrences of the
/// report's *visible* clusters — the exact line set [METRICS-REPO] requires
/// the duplication metric to count. Keyed by the relative occurrence path so
/// callers match it against the absolute metric path with `ends_with`.
pub(crate) fn visible_duplicated_lines(report: &Value) -> BTreeMap<String, BTreeSet<u64>> {
    let mut per_file: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    for cluster in clusters(report) {
        for occurrence in occurrences(cluster) {
            if field(occurrence, "hidden").as_bool().unwrap_or(false) {
                continue;
            }
            let Some(path) = field(occurrence, "path").as_str() else {
                continue;
            };
            let start = field(occurrence, "start_line").as_u64().unwrap_or(0);
            let end = field(occurrence, "end_line").as_u64().unwrap_or(0);
            let entry = per_file.entry(path.to_owned()).or_default();
            for line in start..=end {
                let _inserted = entry.insert(line);
            }
        }
    }
    per_file
}

/// Total lines the report's visible clusters cover across every file — the
/// repo-level `duplicated_loc` the metric must report ([METRICS-REPO]).
pub(crate) fn visible_duplicated_loc(report: &Value) -> u64 {
    visible_duplicated_lines(report)
        .values()
        .map(line_count)
        .sum()
}

/// Writes two byte-identical source files (`a.<extension>`, `b.<extension>`)
/// into a freshly created `dir`: the minimal corpus for a fully-duplicated
/// repo, used to prove the duplication metric is language-agnostic.
pub(crate) fn write_identical_pair(dir: &Path, extension: &str, source: &str) -> Result<()> {
    fs::create_dir_all(dir)?;
    for stem in ["a", "b"] {
        fs::write(dir.join(format!("{stem}.{extension}")), source)?;
    }
    Ok(())
}

/// A genuine copy-pasted F# function — byte-identical across two files.
/// Shared recall-guard source for the #331/#336 shape-only fixtures.
pub(crate) const FSHARP_GENUINE_CLONE: &str = "module ParseHelpers\n\n\
    let accumulate (values: int list) (floor: int) =\n\
    \x20   let mutable total = 0\n\
    \x20   for value in values do\n\
    \x20       if value > floor then\n\
    \x20           total <- total + value * 2\n\
    \x20       else\n\
    \x20           total <- total - 1\n\
    \x20   total\n";

/// One F# module holding a numeric array literal. Same length (same
/// shape) across modules, entirely different values — the #336
/// false-positive family.
pub(crate) fn fsharp_table_file(module_name: &str, seed: usize) -> String {
    let values: Vec<String> = (0_usize..24)
        .map(|index| {
            let mixed = seed
                .saturating_mul(37)
                .saturating_add(index.saturating_mul(13));
            (mixed % 97).to_string()
        })
        .collect();
    format!(
        "module {module_name}\n\nlet lookup = [| {} |]\n",
        values.join("; ")
    )
}

/// The four distinct-value F# table files plus the byte-identical
/// genuine clone pair — the canonical #336 corpus.
pub(crate) fn fsharp_tables_corpus() -> Vec<(String, String)> {
    let modules = ["TablesAlpha", "TablesBeta", "TablesGamma", "TablesDelta"];
    let mut files: Vec<(String, String)> = modules
        .iter()
        .enumerate()
        .map(|(index, module_name)| {
            (
                format!("tables_{index}.fs"),
                fsharp_table_file(module_name, index),
            )
        })
        .collect();
    files.extend(genuine_pair(
        "parse_a.fs",
        "parse_b.fs",
        FSHARP_GENUINE_CLONE,
    ));
    files
}

/// The two byte-identical files forming a genuine-clone recall guard.
pub(crate) fn genuine_pair(first: &str, second: &str, source: &str) -> [(String, String); 2] {
    [
        (first.to_owned(), source.to_owned()),
        (second.to_owned(), source.to_owned()),
    ]
}

/// Writes `(file_name, source)` pairs into a temp scan root and returns
/// the rendered report at `min_nodes`. Config rides along as an ordinary
/// `.deslop.toml` entry in `files` when a test needs a policy override.
pub(crate) fn report_for(files: &[(String, String)], min_nodes: u32) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("src");
    fs::create_dir_all(&root)?;
    for (file_name, source) in files {
        fs::write(root.join(file_name), source)?;
    }
    run_report(&root, min_nodes)
}

/// Zero-based rank of the first cluster whose occurrences include a file
/// whose name satisfies `matches`, or `None` when no visible cluster does.
pub(crate) fn rank_where(report: &Value, matches: impl Fn(&str) -> bool) -> Option<usize> {
    clusters(report)
        .iter()
        .position(|cluster| cluster_file_set(cluster).iter().any(|name| matches(name)))
}

/// The `category` wire label of the first cluster touching a matching
/// file name, or `""` when no visible cluster does. Resolves the cluster
/// through [`rank_where`] so the two lookups can never disagree.
pub(crate) fn category_where(report: &Value, matches: impl Fn(&str) -> bool) -> String {
    rank_where(report, matches)
        .and_then(|rank| clusters(report).get(rank))
        .and_then(|cluster| field(cluster, "category").as_str())
        .unwrap_or_default()
        .to_owned()
}
