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
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

/// Environment variable that switches the suite from strict mode (any failure
/// fails the test) to baseline mode (only *new* failures fail).
pub const BASELINE_ENV: &str = "DESLOP_CORPUS_BASELINE";

/// `/usr/bin/time` flag that reports peak resident set size. BSD (macOS)
/// spells it `-l`; GNU (Linux, which is what the scheduled corpus workflow
/// runs on) has no `-l` at all and rejects the invocation outright, so a
/// hard-coded `-l` would kill every scan before a single check ran.
const PEAK_RSS_FLAG: &str = if cfg!(target_os = "macos") {
    "-l"
} else {
    "-v"
};

/// One failed check, keyed by a rank-independent id.
///
/// The id must not embed a cluster rank or count. #301 makes ranks move
/// between runs, so a rank-bearing key would churn the baseline and defeat
/// the whole mechanism.
#[derive(Debug, Clone)]
pub struct Failure {
    /// Stable check id, e.g. `memory` or `boilerplate_rank`.
    pub check: String,
    /// Human-readable detail for the report.
    pub detail: String,
}

impl Failure {
    /// Builds a failure for `check` with the given detail.
    pub fn new(check: &str, detail: impl Into<String>) -> Self {
        Self {
            check: check.to_owned(),
            detail: detail.into(),
        }
    }
}

/// The set of checks already known to fail, per repository.
///
/// This is a ratchet, not an excuse: entries record defects that already have
/// a tracked issue, so CI reports them without blocking. Anything not listed
/// is a regression and fails even in baseline mode.
#[derive(Debug, Default)]
pub struct Baseline {
    /// Check ids already known to fail, keyed by repository name.
    known: BTreeMap<String, BTreeSet<String>>,
}

impl Baseline {
    /// Loads `corpus/known-failures.json`.
    ///
    /// # Errors
    ///
    /// Returns an error when the file exists but is not valid JSON.
    pub fn load() -> Result<Self> {
        let path = repo_root().join("corpus").join("known-failures.json");
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(Self::default());
        };
        let parsed: Value = serde_json::from_str(&text)
            .with_context(|| format!("known-failures.json is not JSON: {}", path.display()))?;
        let known = parsed
            .get("known_failures")
            .and_then(Value::as_object)
            .map(|entries| {
                entries
                    .iter()
                    .map(|(repo, checks)| {
                        let checks = checks
                            .as_array()
                            .map(|list| {
                                list.iter()
                                    .filter_map(Value::as_str)
                                    .map(ToOwned::to_owned)
                                    .collect()
                            })
                            .unwrap_or_default();
                        (repo.clone(), checks)
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self { known })
    }

    /// Checks recorded as already failing for `repo`.
    #[must_use]
    pub fn known_for(&self, repo: &str) -> BTreeSet<String> {
        self.known.get(repo).cloned().unwrap_or_default()
    }
}

/// True when the suite should report known failures instead of failing on them.
#[must_use]
pub fn baseline_mode() -> bool {
    std::env::var(BASELINE_ENV).is_ok_and(|value| value != "0" && !value.is_empty())
}

/// Prints every observed failure, classified against the baseline, and returns
/// the failures that should fail the test.
///
/// In strict mode (the default, and what `make test-corpus` runs locally) that
/// is all of them. In baseline mode it is only the ones not already recorded.
/// Checks in the baseline that did *not* fire are reported as possibly fixed
/// but never fail a run — with #301 outstanding, a lucky pass is not proof.
/// `evaluated` names the checks this caller actually ran. It is required
/// because a repository's checks are split across more than one test: the
/// determinism gate cannot observe `memory`, and the main gate cannot observe
/// `determinism`. Without it, each test would report the other's baseline
/// entries as possibly fixed while they were still failing elsewhere.
#[must_use]
pub fn classify(
    repo: &str,
    evaluated: &[&str],
    observed: &[Failure],
    baseline: &Baseline,
) -> Vec<Failure> {
    let known = baseline.known_for(repo);
    let (mut fresh, mut carried) = (Vec::new(), Vec::new());
    for failure in observed {
        if known.contains(&failure.check) {
            carried.push(failure.clone());
        } else {
            fresh.push(failure.clone());
        }
    }

    for failure in &carried {
        println!("  [KNOWN]  {repo}/{}: {}", failure.check, failure.detail);
    }
    for failure in &fresh {
        println!("  [NEW]    {repo}/{}: {}", failure.check, failure.detail);
    }
    let observed_checks: BTreeSet<&str> = observed
        .iter()
        .map(|failure| failure.check.as_str())
        .collect();
    let evaluated: BTreeSet<&str> = evaluated.iter().copied().collect();
    for check in known
        .iter()
        .filter(|check| evaluated.contains(check.as_str()))
        .filter(|check| !observed_checks.contains(check.as_str()))
    {
        println!(
            "  [FIXED?] {repo}/{check}: baseline expects this to fail but it passed. \
             Confirm, then remove it from corpus/known-failures.json."
        );
    }

    if baseline_mode() {
        fresh
    } else {
        observed.to_vec()
    }
}

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
    serde_json::from_str(&text)
        .with_context(|| format!("corpus manifest is not JSON: {}", path.display()))
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
    let binary = release_binary()?;

    let started = Instant::now();
    let output = timed_scan(&binary, scan_root, output_prefix)?;
    let wall = started.elapsed();

    if !output.status.success() {
        return Err(scan_failure(scan_root, &output));
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

/// Locates the release binary the suite measures.
fn release_binary() -> Result<PathBuf> {
    let binary = repo_root().join("target").join("release").join("deslop");
    if binary.is_file() {
        return Ok(binary);
    }
    Err(anyhow!(
        "release binary missing at {}. Run `make test-corpus`, which builds it first.",
        binary.display()
    ))
}

/// Runs one scan under `/usr/bin/time`, capturing its output.
fn timed_scan(binary: &Path, scan_root: &Path, output_prefix: &Path) -> Result<Output> {
    Command::new("/usr/bin/time")
        .arg(PEAK_RSS_FLAG)
        .arg(binary)
        .arg(scan_root)
        .arg("--output")
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
        .context("failed to spawn /usr/bin/time")
}

/// Describes a non-zero scan, quoting stderr.
///
/// The failing process may be `deslop` or `/usr/bin/time` itself — a flag the
/// host's `time` does not accept dies here too — so the message names both
/// rather than blaming the scan for a harness fault.
fn scan_failure(scan_root: &Path, output: &Output) -> anyhow::Error {
    anyhow!(
        "`/usr/bin/time {PEAK_RSS_FLAG} deslop {}` exited {:?}: {}",
        scan_root.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join(" | ")
    )
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
        .find(|line| {
            line.to_ascii_lowercase()
                .contains("maximum resident set size")
        })
        .ok_or_else(|| anyhow!("/usr/bin/time did not report a maximum resident set size"))?;

    let value: u64 = line
        .split_whitespace()
        .find_map(|token| token.parse().ok())
        .ok_or_else(|| anyhow!("no numeric peak RSS in {line:?}"))?;

    let in_kbytes = line.to_ascii_lowercase().contains("kbytes");
    Ok(if in_kbytes {
        value / 1024
    } else {
        value / (1024 * 1024)
    })
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
///
/// An empty `files` list is false, never true. `all()` over nothing is
/// vacuously true, which would turn a manifest entry that lists no files into
/// a recall assertion that always passes — the exact shape of a test that
/// asserts nothing.
#[must_use]
pub fn reports_clone_spanning(report: &Value, files: &[String]) -> bool {
    if files.is_empty() {
        return false;
    }
    cluster_paths(report).iter().any(|paths| {
        files
            .iter()
            .all(|file| paths.iter().any(|path| path == file))
    })
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
