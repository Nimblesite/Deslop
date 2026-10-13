//! [CORPUS-REPORT] Every `corpus_*` test writes its own scorecard — the same
//! document `score-gate.sh` and `compare-versions.sh` write, through the same
//! code ([`crate::corpus_score::run`]).
//!
//! A test's scorecard is named after the test and the moment it ran —
//! `.corpus/test-corpus/corpus_flutter_dart-2026-09-27T03-35-02Z.md` — so no
//! run overwrites another. Beside it, a directory of the same name keeps what
//! the scorecard was rendered from: the scan's report, its measured cost, its
//! curated-check outcome and the run manifest, so any scorecard can be
//! re-rendered and audited later.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    corpus::{release_binary, repo_root, string_field, Baseline, Failure},
    corpus_measure::{binary_sha256, ProcessCost},
    corpus_score::{
        checks::{CheckFailure, CheckOutcome},
        run::{report_stem, write_scorecard},
        RunCost,
    },
};

/// Where the corpus tests' scorecards live, relative to the repository root.
/// Git-ignored with the rest of `.corpus/`.
pub const RECORD_DIR: &str = ".corpus/test-corpus";
/// The run manifest the scorer reads.
const RUN_MANIFEST: &str = "run.json";
/// The scan's report, kept beside the scorecard it feeds.
const REPORT_FILE: &str = "report.json";
/// What the scan cost, stamped with the binary that ran it.
const TIMING_FILE: &str = "timing.json";
/// The curated checks the test evaluated, and what failed.
const CHECKS_FILE: &str = "checks.json";
/// Where a repository's clone register would be, relative to the root.
const REGISTER_DIR: &str = "corpus/register";
/// How much of the binary's fingerprint names its column — the same length
/// the corpus scripts use, so both spell one engine the same way.
const ENGINE_ID_LENGTH: usize = crate::corpus::SHORT_SHA_LENGTH;

/// One corpus test's scan, ready to be filed.
#[derive(Debug)]
pub struct Record<'a> {
    /// What ran — the test's own name — which names the scorecard.
    pub run_name: &'a str,
    /// The repository's row label in the scorecard.
    pub name: &'a str,
    /// The corpus manifest the test read, for the language and pinned commit.
    pub manifest: &'a Value,
    /// Whether this record is scored against the repository's clone register.
    /// A second scan of one repository leaves it off, so one register is
    /// never counted twice.
    pub scored: bool,
    /// The scan's report, absent when the scan crashed before writing one.
    pub report_path: Option<&'a Path>,
    /// What the scan cost — up to the crash, when it crashed.
    pub cost: &'a ProcessCost,
    /// What the test checked, and what failed.
    pub checks: CheckOutcome,
}

/// The outcome of every curated check a test evaluated, each failure marked
/// with whether `corpus/known-failures.json` already tracks it.
#[must_use]
pub fn check_outcome(
    repo: &str,
    evaluated: &[&str],
    failures: &[Failure],
    baseline: &Baseline,
    accuracy_curated: bool,
) -> CheckOutcome {
    let known = baseline.known_for(repo);
    CheckOutcome {
        evaluated: evaluated.iter().map(|check| (*check).to_owned()).collect(),
        accuracy_curated,
        failures: failures
            .iter()
            .map(|failure| recorded(failure, &known))
            .collect(),
    }
}

/// One failure as the scorecard records it.
fn recorded(failure: &Failure, known: &std::collections::BTreeSet<String>) -> CheckFailure {
    CheckFailure {
        check: failure.check.clone(),
        detail: failure.detail.clone(),
        known: known.contains(&failure.check),
    }
}

/// Files `record` and renders its scorecard, returning the scorecard's path.
///
/// # Errors
///
/// Returns an error when the release binary cannot be fingerprinted, or the
/// record, the run manifest or the scorecard cannot be written.
pub fn publish(record: &Record<'_>) -> Result<PathBuf> {
    let root = repo_root();
    let dir = root.join(RECORD_DIR);
    let stem = report_stem(record.run_name, SystemTime::now());
    let run_dir = dir.join(&stem);
    let binary_sha = binary_sha256(&release_binary()?)?;
    write_record(&run_dir, record, &binary_sha)?;
    let run_path = run_dir.join(RUN_MANIFEST);
    let relative_dir = format!("{RECORD_DIR}/{stem}");
    write_pretty(
        &run_path,
        &run_manifest(record, &relative_dir, &binary_sha)?,
    )?;
    let written = write_scorecard(&run_path, &root, &dir, &stem, false)?;
    println!("==> corpus scorecard: {}", written.path.display());
    Ok(written.path)
}

/// Writes one record's report, cost and checks.
fn write_record(dir: &Path, record: &Record<'_>, binary_sha: &str) -> Result<()> {
    fs::create_dir_all(dir)?;
    if let Some(report) = record.report_path {
        let _bytes = fs::copy(report, dir.join(REPORT_FILE))
            .with_context(|| format!("cannot keep {}", report.display()))?;
    }
    write_pretty(
        &dir.join(TIMING_FILE),
        &RunCost::measured(record.cost, binary_sha),
    )?;
    write_pretty(&dir.join(CHECKS_FILE), &record.checks)
}

/// Writes `value` as pretty JSON with a trailing newline.
fn write_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")
        .with_context(|| format!("cannot write {}", path.display()))
}

/// The run manifest for one record, its paths relative to the repository root
/// under `relative_dir`. A crashed scan names no report rather than a file
/// that does not exist.
fn run_manifest(record: &Record<'_>, relative_dir: &str, binary_sha: &str) -> Result<Value> {
    let engine_id: String = binary_sha.chars().take(ENGINE_ID_LENGTH).collect();
    let file = |name: &str| format!("{relative_dir}/{name}");
    let register = string_field(record.manifest, "name")?;
    Ok(json!({
        "generated_at": humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
        "engines": [{ "id": engine_id, "label": format!("deslop (binary {engine_id})") }],
        "targets": [{
            "name": record.name,
            "language": string_field(record.manifest, "language")?,
            "sha": string_field(record.manifest, "sha")?,
            "register": record.scored.then(|| format!("{REGISTER_DIR}/{register}.json")),
            "runs": { engine_id: {
                "report": record.report_path.map(|_| file(REPORT_FILE)),
                "timing": file(TIMING_FILE),
                "checks": file(CHECKS_FILE),
            }},
        }],
    }))
}

#[cfg(test)]
mod tests;
