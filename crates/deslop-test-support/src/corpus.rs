//! [CORPUS-PIN] [CORPUS-CEILINGS] [CORPUS-BASELINE] Harness for the `corpus_*`
//! accuracy and resource suite. Spec: `docs/specs/corpus.md`.
//!
//! The suite scans real public repositories, pinned to a commit by
//! `corpus/*.json`, and asserts two things the small fixture suites cannot:
//! that genuine hand-verified duplicates are actually reported, and that a
//! scan of a real codebase stays inside a wall-clock and memory budget.
//!
//! Clones live in git-ignored `.corpus/`, populated by
//! `scripts/corpus/fetch-corpus.mjs` (which `make test-corpus` runs first). Nothing
//! here touches the network: a missing clone is a hard error naming the
//! target to run, never a silent skip.

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Output,
};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::corpus_measure::{measured_run, measurement, ProcessCost};

mod baseline;

pub use baseline::{baseline_mode, classify, Baseline, BASELINE_ENV};

/// [CORPUS-PIN] The one list naming the files under `corpus/` and
/// `corpus/register/` that describe no single upstream repository, so no
/// manifest, pin or clone contract applies to them.
///
/// Data, not code. `scripts/corpus/fetch-corpus.mjs` reads the same file, and
/// the fetch runs before anything is compiled, so a copy of the list held here
/// could only ever be a second one — and a second copy is what went stale.
const NOT_A_REPOSITORY: &str = include_str!("../../../corpus/not-a-repository.json");

/// The key under which that list holds the file names.
const NOT_A_REPOSITORY_KEY: &str = "files";

/// The extension every corpus manifest carries.
const MANIFEST_EXTENSION: &str = "json";

/// [CORPUS-PIN] Whether `path` is a corpus manifest: a `.json` file naming one
/// upstream repository, rather than one of the settings files the list above
/// names.
///
/// Answered by name rather than by shape, so a manifest missing a field is an
/// error where it is read, never a file quietly treated as settings.
///
/// # Errors
///
/// When `corpus/not-a-repository.json` is not valid JSON holding a `files`
/// array. Every caller reads a directory of manifests, so a list that cannot be
/// read must stop the caller rather than let it decide alone.
pub fn describes_a_repository(path: &Path) -> Result<bool> {
    let list: Value = serde_json::from_str(NOT_A_REPOSITORY)
        .context("corpus/not-a-repository.json must be valid JSON")?;
    let settings = list
        .get(NOT_A_REPOSITORY_KEY)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("corpus/not-a-repository.json has no `{NOT_A_REPOSITORY_KEY}`"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    Ok(path
        .extension()
        .is_some_and(|extension| extension == MANIFEST_EXTENSION)
        && !settings.iter().any(|listed| listed.as_str() == Some(name)))
}

/// The member-count field a rendered cluster actually carries.
///
/// Named once because reading a field the report does not emit is silent: the
/// lookup returns zero and every breach is reported as an empty cluster. The
/// wire model carries no `size` and no `category`; both were read anyway, in
/// two separate checks, for as long as nothing asserted what they said
/// (gh #540).
pub const OCCURRENCE_COUNT: &str = "occurrence_count";

/// [CORPUS-PIN] How much of a manifest's commit id names its clone directory.
/// The pin itself is always the full object name; this is only how it reads on
/// disk and in a log line.
pub const SHORT_SHA_LENGTH: usize = 12;

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

/// A scan's measured cost, alongside the report it produced.
#[derive(Debug)]
pub struct CorpusRun {
    /// Parsed canonical JSON report.
    pub report: Value,
    /// Where that report was written, so it can be kept beside the scorecard.
    pub report_path: PathBuf,
    /// What the scan cost, as [`crate::corpus_measure`] measured it.
    pub cost: ProcessCost,
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
    crate::read_json(&repo_root().join("corpus").join(format!("{name}.json")))
}

/// [CORPUS-PIN] Resolves the clone directory for a manifest, erroring when
/// it is absent.
///
/// # Errors
///
/// Returns an error naming `make test-corpus` when the clone is missing, so a
/// developer running `cargo test corpus_` directly gets an actionable failure
/// instead of a mysterious one.
pub fn clone_dir(manifest: &Value) -> Result<PathBuf> {
    let name = string_field(manifest, "name")?;
    let sha = string_field(manifest, "sha")?;
    let short = sha.get(..SHORT_SHA_LENGTH).unwrap_or(sha);
    let dir = repo_root().join(".corpus").join(format!("{name}-{short}"));
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

/// Scans `scan_root` with the release `deslop` binary under this platform's
/// [`crate::corpus_measure::Measurement`], returning the parsed report plus
/// what the scan cost.
///
/// Embeddings are off and the fingerprint cache is disabled so the measurement
/// reflects a cold analytical run and never writes into the clone.
///
/// # Errors
///
/// Returns an error when the binary is missing, or the rendered report cannot
/// be read. A scan that exits non-zero is a [`ScanCrashed`], which still
/// carries what the scan cost up to its exit.
pub fn scan(scan_root: &Path, output_prefix: &Path) -> Result<CorpusRun> {
    let binary = release_binary()?;
    let run = measured_run(&binary, &scan_args(scan_root, output_prefix))?;

    if !run.output.status.success() {
        return Err(ScanCrashed::new(scan_root, &run.output, run.cost).into());
    }

    let report_path = crate::with_ext(output_prefix, "json");
    Ok(CorpusRun {
        report: crate::read_json(&report_path).context("scan report")?,
        report_path,
        cost: run.cost,
    })
}

/// The argument list one measured corpus scan runs with.
fn scan_args(scan_root: &Path, output_prefix: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        scan_root.into(),
        OsString::from("--output"),
        output_prefix.into(),
    ];
    args.extend(SCAN_FLAGS.iter().map(OsString::from));
    args
}

/// Where the release binary the suite measures is expected to sit.
///
/// The stem carries [`std::env::consts::EXE_SUFFIX`]: cargo writes
/// `deslop.exe` on Windows, and a bare stem makes the existence check below
/// false with the binary sitting right beside it — every corpus test then
/// dies on "release binary missing" before it scans anything.
fn release_binary_path() -> PathBuf {
    repo_root()
        .join("target")
        .join("release")
        .join(format!("deslop{}", std::env::consts::EXE_SUFFIX))
}

/// Locates the release binary the suite measures.
///
/// # Errors
///
/// Returns an error naming the expected path when the binary is not there.
pub fn release_binary() -> Result<PathBuf> {
    let binary = release_binary_path();
    if binary.is_file() {
        return Ok(binary);
    }
    Err(anyhow!(
        "release binary missing at {}. Run `make test-corpus`, which builds it first.",
        binary.display()
    ))
}

/// The analysis flags every corpus scan runs with.
///
/// Embeddings are off and the fingerprint cache is disabled so the
/// measurement reflects a cold analytical run and never writes into the
/// clone. Shared by both measurement arms so the two platforms cannot drift
/// into scanning with different settings.
const SCAN_FLAGS: [&str; 7] = [
    "--no-incremental",
    "--embeddings",
    "off",
    "--no-fail-over",
    "--no-color",
    "--notext",
    "--nohtml",
];

/// A scan that exited non-zero: why it stopped, and what it cost up to then.
///
/// A crash is a result, not an absence of one — the scorecard records it with
/// the wall time, CPU and peak memory the scan reached, because a scan that
/// runs out of memory is exactly the one whose memory figure matters.
#[derive(Debug)]
pub struct ScanCrashed {
    /// What the scan cost before it exited.
    pub cost: ProcessCost,
    /// How the scan exited, and everything it wrote to stderr.
    pub reason: String,
}

impl ScanCrashed {
    /// Describes a non-zero scan, quoting all of its stderr: the line that
    /// says why a scan stopped is rarely the first, and cutting the quote
    /// short is how an out-of-memory abort was once reported as its banner.
    ///
    /// The failing process may be `deslop` or the measurement wrapper itself —
    /// a flag the host's `time` does not accept dies here too — so the message
    /// names the measurement rather than blaming the scan for a harness fault.
    fn new(scan_root: &Path, output: &Output, cost: ProcessCost) -> Self {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let quoted: Vec<&str> = stderr
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        Self {
            cost,
            reason: format!(
                "`{:?} deslop {}` exited {:?}: {}",
                measurement(),
                scan_root.display(),
                output.status.code(),
                quoted.join(" | ")
            ),
        }
    }
}

impl std::fmt::Display for ScanCrashed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl std::error::Error for ScanCrashed {}

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

/// [CORPUS-RECALL] True when some reported cluster covers every path in
/// `files`. This is the recall predicate: a curated duplicate that no cluster spans is a false
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

/// Clusters the report actually shows a user. A cluster whose every
/// occurrence is hidden carries no claim, so it can neither satisfy recall
/// nor breach precision.
#[must_use]
pub fn visible_clusters(report: &Value) -> Vec<&Value> {
    match report.get("clusters").and_then(Value::as_array) {
        None => Vec::new(),
        Some(clusters) => clusters
            .iter()
            .filter(|cluster| !all_occurrences_hidden(cluster))
            .collect(),
    }
}

/// True when every occurrence of a cluster is hidden, so nothing is rendered.
fn all_occurrences_hidden(cluster: &Value) -> bool {
    match cluster.get("occurrences").and_then(Value::as_array) {
        None => true,
        Some(occurrences) => occurrences
            .iter()
            .all(|occurrence| occurrence.get("hidden").and_then(Value::as_bool) == Some(true)),
    }
}

/// True when every path in `files` appears among the cluster's **shown**
/// occurrences.
///
/// One predicate, read in opposite directions: [CORPUS-RECALL] wants it
/// true for a curated duplicate, [CORPUS-PRECISION-CURATED] wants it false
/// for a curated non-duplicate. Both are claims about what the report
/// *shows*, so a hidden occurrence counts for neither — a suppressed side
/// is a pair the user never sees, and an unshown coincidence is not a false
/// positive anyone was told about.
///
/// An empty list is false, never true, for the same reason
/// [`reports_clone_spanning`] refuses one: `all()` over nothing is
/// vacuously true, and an entry naming no files would then assert nothing
/// while reading as a satisfied check.
#[must_use]
pub fn cluster_shows_span(cluster: &Value, files: &[String]) -> bool {
    if files.is_empty() {
        return false;
    }
    let paths: Vec<&str> = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|occurrence| occurrence.get("hidden").and_then(Value::as_bool) != Some(true))
        .filter_map(|occurrence| occurrence.get("path").and_then(Value::as_str))
        .collect();
    files.iter().all(|file| paths.contains(&file.as_str()))
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

/// Array-valued field of `value`, or an empty slice when absent or not an
/// array. Manifest and report readers share this so an absent curated list
/// reads as "asserts nothing" in exactly one place.
#[must_use]
pub fn array<'a>(value: &'a Value, name: &str) -> &'a [Value] {
    value
        .get(name)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Unsigned scalar field of `value`, or `0` when absent.
#[must_use]
pub fn field_u64(value: &Value, name: &str) -> u64 {
    value.get(name).and_then(Value::as_u64).unwrap_or_default()
}

#[cfg(test)]
mod tests;
