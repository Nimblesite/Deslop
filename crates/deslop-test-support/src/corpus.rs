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
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
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

/// [CORPUS-BASELINE] The set of checks already known to fail, per repository.
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
    let (fresh, carried): (Vec<Failure>, Vec<Failure>) = observed
        .iter()
        .cloned()
        .partition(|failure| !known.contains(&failure.check));

    print_failures("[KNOWN] ", repo, &carried);
    print_failures("[NEW]   ", repo, &fresh);
    print_possibly_fixed(repo, evaluated, observed, &known);

    if baseline_mode() {
        fresh
    } else {
        observed.to_vec()
    }
}

/// Prints each failure under a classification label.
fn print_failures(label: &str, repo: &str, failures: &[Failure]) {
    for failure in failures {
        println!("  {label} {repo}/{}: {}", failure.check, failure.detail);
    }
}

/// Prints the baseline entries that were evaluated this run and did not fire.
///
/// Scoped to `evaluated` on purpose: a repository's checks are split across
/// more than one test, so an unscoped reconciliation would announce the
/// determinism gate's live defect as fixed from inside the resource gate.
fn print_possibly_fixed(
    repo: &str,
    evaluated: &[&str],
    observed: &[Failure],
    known: &BTreeSet<String>,
) {
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
}

/// A scan's measured cost, alongside the parsed report it produced.
#[derive(Debug)]
pub struct CorpusRun {
    /// Parsed canonical JSON report.
    pub report: Value,
    /// Wall-clock duration of the scan process.
    pub wall: Duration,
    /// Peak resident set size in mebibytes, as reported by [`Measurement`].
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

/// Scans `scan_root` with the release `deslop` binary under this platform's
/// peak-RSS [`Measurement`], returning the parsed report plus measured wall
/// time and peak RSS.
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

/// [PAIR-COMPARE-CLI] Asks the measured binary for admission evidence on
/// exactly two occurrences, returning the engine's verdict.
///
/// Evidence is pair-scoped and recomputed on demand, so it appears in no
/// rendered report; the gate has to ask for it. Driven through the same
/// binary the scan measured, so the gate stays black-box (gh #488).
///
/// # Errors
///
/// Returns an error when the binary is missing, the comparison fails, or
/// the verdict is not JSON.
pub fn compare_pair(scan_root: &Path, left: &str, right: &str) -> Result<Value> {
    let binary = release_binary()?;
    let output = Command::new(&binary)
        .arg(scan_root)
        .arg("--compare")
        .arg(left)
        .arg("--compare")
        .arg(right)
        .args(SCAN_FLAGS)
        .output()
        .with_context(|| format!("failed to spawn {}", binary.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "pair comparison of {left} against {right} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "pair verdict is not JSON: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

/// The `<path>:<start_byte>:<end_byte>` endpoint of one occurrence.
///
/// # Errors
///
/// Returns an error when the occurrence lacks a path or either offset.
pub fn occurrence_endpoint(occurrence: &Value) -> Result<String> {
    let path = occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("occurrence has no path"))?;
    Ok(format!(
        "{path}:{}:{}",
        byte_offset(occurrence, "start_byte")?,
        byte_offset(occurrence, "end_byte")?
    ))
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
fn release_binary() -> Result<PathBuf> {
    let binary = release_binary_path();
    if binary.is_file() {
        return Ok(binary);
    }
    Err(anyhow!(
        "release binary missing at {}. Run `make test-corpus`, which builds it first.",
        binary.display()
    ))
}

/// How this platform measures a child process's peak resident set size.
///
/// [CORPUS-CEILINGS] needs a *true* peak, not a sampled one: a sample taken
/// every few hundred milliseconds is a lower bound, and a lower bound on a
/// ceiling assertion produces false passes. Both arms below read a counter
/// the kernel maintains, so neither can miss a spike.
#[derive(Debug)]
pub enum Measurement {
    /// POSIX: `/usr/bin/time <flag>` wraps the scan and reports the peak on
    /// stderr when it exits.
    PosixTime {
        /// The peak-RSS flag this platform's `time` accepts.
        flag: &'static str,
    },
    /// Windows has no `/usr/bin/time`. The scan is spawned directly and a
    /// PowerShell monitor watches `PeakWorkingSet64` — the OS's own
    /// monotonically increasing peak counter — for that pid.
    WindowsPeakMonitor {
        /// The monitor script this platform runs.
        script: PathBuf,
    },
}

/// The peak-RSS measurement this platform uses.
#[must_use]
pub fn measurement() -> Measurement {
    if cfg!(windows) {
        Measurement::WindowsPeakMonitor {
            script: windows_monitor_script(),
        }
    } else {
        Measurement::PosixTime {
            flag: PEAK_RSS_FLAG,
        }
    }
}

/// The PowerShell monitor that reports a pid's peak working set.
fn windows_monitor_script() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("corpus")
        .join("peak-working-set.ps1")
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

/// Runs one scan under this platform's peak-RSS measurement, capturing its
/// output. Both arms leave the peak on stderr in the form [`peak_rss_mb`]
/// reads, so everything downstream is platform-independent.
fn timed_scan(binary: &Path, scan_root: &Path, output_prefix: &Path) -> Result<Output> {
    match measurement() {
        Measurement::PosixTime { flag } => posix_scan(flag, binary, scan_root, output_prefix),
        Measurement::WindowsPeakMonitor { script } => {
            windows_scan(&script, binary, scan_root, output_prefix)
        }
    }
}

/// Runs the scan under `/usr/bin/time`, which reports the peak itself.
fn posix_scan(flag: &str, binary: &Path, scan_root: &Path, output_prefix: &Path) -> Result<Output> {
    Command::new("/usr/bin/time")
        .arg(flag)
        .arg(binary)
        .arg(scan_root)
        .arg("--output")
        .arg(output_prefix)
        .args(SCAN_FLAGS)
        .output()
        .context("failed to spawn /usr/bin/time")
}

/// Runs the scan directly and watches its peak working set from PowerShell.
///
/// The monitor takes only a pid, so no path has to survive a shell quoting
/// round-trip. Its reading is appended to the scan's own stderr, which is
/// where the POSIX arm leaves it too.
fn windows_scan(
    script: &Path,
    binary: &Path,
    scan_root: &Path,
    output_prefix: &Path,
) -> Result<Output> {
    let child = Command::new(binary)
        .arg(scan_root)
        .arg("--output")
        .arg(output_prefix)
        .args(SCAN_FLAGS)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", binary.display()))?;
    let monitor = spawn_peak_monitor(script, child.id())?;
    let mut output = child.wait_with_output().context("scan did not complete")?;
    let peak = monitor
        .wait_with_output()
        .context("peak-working-set monitor did not complete")?;
    output.stderr.extend_from_slice(&peak.stdout);
    Ok(output)
}

/// Starts the PowerShell monitor watching `process_id`.
fn spawn_peak_monitor(script: &Path, process_id: u32) -> Result<std::process::Child> {
    Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .arg("-ProcessId")
        .arg(process_id.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", script.display()))
}

/// Describes a non-zero scan, quoting stderr.
///
/// The failing process may be `deslop` or the measurement wrapper itself — a
/// flag the host's `time` does not accept dies here too — so the message
/// names the measurement rather than blaming the scan for a harness fault.
fn scan_failure(scan_root: &Path, output: &Output) -> anyhow::Error {
    anyhow!(
        "`{:?} deslop {}` exited {:?}: {}",
        measurement(),
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

/// [CORPUS-CEILINGS] Extracts peak RSS in mebibytes from `/usr/bin/time -l` (BSD/macOS, bytes)
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
        .ok_or_else(|| anyhow!("the measurement reported no maximum resident set size"))?;

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
