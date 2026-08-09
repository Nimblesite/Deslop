//! [CORPUS] Harness for the `corpus_*` accuracy and resource suite.
//!
//! The suite scans real public repositories, pinned to a commit by
//! `corpus/*.json`, and asserts two things the small fixture suites cannot:
//! that genuine hand-verified duplicates are actually reported, and that a
//! scan of a real codebase stays inside a wall-clock and memory budget.
//!
//! Clones live in git-ignored `.corpus/`, populated by
//! `scripts/fetch-corpus.mjs` (which `make test-corpus` runs first). Nothing
//! here touches the network: a missing clone is a hard error naming the
//! target to run, never a silent skip.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

/// A scan's measured cost, alongside the parsed report it produced.
#[derive(Debug)]
pub struct CorpusRun {
    /// Parsed canonical JSON report.
    pub report: Value,
    /// Wall-clock duration of the scan process.
    pub wall: Duration,
    /// Peak resident set size in mebibytes, as reported by `/usr/bin/time`.
    pub peak_rss_mb: u64,
}

/// Repository root, derived from this crate's manifest directory.
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Loads `corpus/<name>.json`.
///
/// # Errors
///
/// Returns an error when the manifest is missing or is not valid JSON.
pub fn manifest(name: &str) -> Result<Value> {
    let path = repo_root().join("corpus").join(format!("{name}.json"));
    let text = fs::read_to_string(&path)
        .with_context(|| format!("corpus manifest not readable: {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("corpus manifest is not JSON: {}", path.display()))
}

/// Resolves the clone directory for a manifest, erroring when it is absent.
///
/// # Errors
///
/// Returns an error naming `make test-corpus` when the clone is missing, so a
/// developer running `cargo test corpus_` directly gets an actionable failure
/// instead of a mysterious one.
pub fn clone_dir(manifest: &Value) -> Result<PathBuf> {
    let name = string_field(manifest, "name")?;
    let tag = string_field(manifest, "tag")?;
    let dir = repo_root().join(".corpus").join(format!("{name}-{tag}"));
    if !dir.is_dir() {
        return Err(anyhow!(
            "corpus clone missing at {}. Run `make test-corpus` (it clones pinned repositories first).",
            dir.display()
        ));
    }
    Ok(dir)
}

/// Reads a required string field off a manifest value.
///
/// # Errors
///
/// Returns an error when the field is absent or not a string.
pub fn string_field<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("corpus manifest field `{name}` is missing or not a string"))
}

/// Reads a required unsigned field off a manifest value.
///
/// # Errors
///
/// Returns an error when the field is absent or not an unsigned integer.
pub fn u64_field(value: &Value, name: &str) -> Result<u64> {
    value
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("corpus manifest field `{name}` is missing or not an integer"))
}

/// Scans `scan_root` with the release `deslop` binary under `/usr/bin/time`,
/// returning the parsed report plus measured wall time and peak RSS.
///
/// Embeddings are off and the fingerprint cache is disabled so the measurement
/// reflects a cold analytical run and never writes into the clone.
///
/// # Errors
///
/// Returns an error when the binary is missing, the scan exits non-zero, or
/// the rendered report cannot be read.
pub fn scan(scan_root: &Path, output_prefix: &Path) -> Result<CorpusRun> {
    let binary = repo_root().join("target").join("release").join("deslop");
    if !binary.is_file() {
        return Err(anyhow!(
            "release binary missing at {}. Run `make test-corpus`, which builds it first.",
            binary.display()
        ));
    }

    let started = Instant::now();
    let output = Command::new("/usr/bin/time")
        .arg("-l")
        .arg(&binary)
        .arg(scan_root)
        .args(["--output"])
        .arg(output_prefix)
        .args([
            "--no-incremental",
            "--embeddings",
            "off",
            "--no-fail-over",
            "--no-color",
            "--notext",
            "--nohtml",
        ])
        .output()
        .context("failed to spawn /usr/bin/time")?;
    let wall = started.elapsed();

    if !output.status.success() {
        return Err(anyhow!(
            "deslop exited {:?} scanning {}",
            output.status.code(),
            scan_root.display()
        ));
    }

    let report_path = with_json_extension(output_prefix);
    let text = fs::read_to_string(&report_path)
        .with_context(|| format!("report not readable: {}", report_path.display()))?;

    Ok(CorpusRun {
        report: serde_json::from_str(&text).context("report is not valid JSON")?,
        wall,
        peak_rss_mb: peak_rss_mb(&String::from_utf8_lossy(&output.stderr))?,
    })
}

/// Appends `.json` to an `--output` prefix.
fn with_json_extension(prefix: &Path) -> PathBuf {
    let mut name = prefix.file_name().unwrap_or_default().to_os_string();
    name.push(".json");
    prefix.with_file_name(name)
}

/// Extracts peak RSS in mebibytes from `/usr/bin/time -l` (BSD/macOS, bytes)
/// or `/usr/bin/time -v` (GNU, kbytes) output. The unit is decided by the
/// label itself rather than by the host, so a mislabelled build cannot be
/// silently misread by three orders of magnitude.
fn peak_rss_mb(stderr: &str) -> Result<u64> {
    let line = stderr
        .lines()
        .find(|line| line.to_ascii_lowercase().contains("maximum resident set size"))
        .ok_or_else(|| anyhow!("/usr/bin/time did not report a maximum resident set size"))?;

    let value: u64 = line
        .split_whitespace()
        .find_map(|token| token.parse().ok())
        .ok_or_else(|| anyhow!("no numeric peak RSS in {line:?}"))?;

    let in_kbytes = line.to_ascii_lowercase().contains("kbytes");
    Ok(if in_kbytes { value / 1024 } else { value / (1024 * 1024) })
}

/// Every occurrence path in the report's `clusters`, grouped per cluster.
#[must_use]
pub fn cluster_paths(report: &Value) -> Vec<Vec<String>> {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| clusters.iter().map(occurrence_paths).collect())
        .unwrap_or_default()
}

/// The occurrence paths of a single cluster.
fn occurrence_paths(cluster: &Value) -> Vec<String> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(|occurrences| {
            occurrences
                .iter()
                .filter_map(|occurrence| occurrence.get("path").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// True when some reported cluster covers every path in `files`. This is the
/// recall predicate: a curated duplicate that no cluster spans is a false
/// negative.
#[must_use]
pub fn reports_clone_spanning(report: &Value, files: &[String]) -> bool {
    cluster_paths(report)
        .iter()
        .any(|paths| files.iter().all(|file| paths.iter().any(|path| path == file)))
}

/// Reads the source slice a cluster's first occurrence points at.
///
/// # Errors
///
/// Returns an error when the occurrence is malformed or the file is unreadable.
pub fn first_occurrence_text(scan_root: &Path, cluster: &Value) -> Result<String> {
    let occurrence = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .and_then(|occurrences| occurrences.first())
        .ok_or_else(|| anyhow!("cluster has no occurrences"))?;

    let path = occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("occurrence has no path"))?;
    let start = byte_offset(occurrence, "start_byte")?;
    let end = byte_offset(occurrence, "end_byte")?;

    let source = fs::read_to_string(scan_root.join(path))
        .with_context(|| format!("occurrence source unreadable: {path}"))?;
    source
        .get(start..end)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("occurrence range {start}..{end} is outside {path}"))
}

/// Reads a byte-offset field off an occurrence.
fn byte_offset(occurrence: &Value, name: &str) -> Result<usize> {
    occurrence
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("occurrence is missing {name}"))
}
