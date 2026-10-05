//! [CORPUS-REPORT] [CORPUS-BASELINE] How a `corpus_*` test turns one scan
//! and the checks it ran into a scorecard record and a verdict.
//!
//! Every corpus test ends here, in one order: file the scan into the corpus
//! scorecard ([`crate::corpus_record`]), then fail on whatever the
//! known-failures baseline does not excuse. The scorecard is written first, so
//! a failing test — a crashed scan included — still leaves the document that
//! says why.

use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::{
    corpus::{array, baseline_mode, classify, Baseline, Failure, ScanCrashed},
    corpus_measure::ProcessCost,
    corpus_record::{check_outcome, publish, Record},
};

/// Checks the main gate evaluates. Used to scope baseline reconciliation so
/// it never reports the determinism gate's entries as fixed.
pub const GATE_CHECKS: &[&str] = &[
    "files_analysed",
    "cluster_count_band",
    "recall",
    "recall_quality",
    "precision",
    "boilerplate_rank",
    "data_table_rank",
    "fused_bounded_max",
    "cluster_contract",
    "cluster_mass",
    "cluster_rank",
    "type2_recall",
    "wall",
    "memory",
];

/// The one check the determinism gate evaluates, and the list it is evaluated as.
pub const DETERMINISM_CHECK: &str = "determinism";
/// See [`DETERMINISM_CHECK`].
pub const DETERMINISM_CHECKS: &[&str] = &[DETERMINISM_CHECK];

/// The check a crashed scan fails: it produced no report to check anything in.
pub const SCAN_CHECK: &str = "scan";
/// See [`SCAN_CHECK`].
pub const SCAN_CHECKS: &[&str] = &[SCAN_CHECK];

/// Which of a repository's corpus tests made a scan.
#[derive(Debug, Clone, Copy)]
pub enum Scan {
    /// The main gate: scored against the register, held to every check.
    Main,
    /// The determinism gate: its own record, never scored a second time
    /// against the register the main gate already scored.
    Rescan,
}

impl Scan {
    /// The row label for a scan of `name`, whether it is scored against the
    /// register, and the checks it is held to.
    fn identity(self, name: &str) -> (String, bool, &'static [&'static str]) {
        match self {
            Self::Main => (name.to_owned(), true, GATE_CHECKS),
            Self::Rescan => (
                format!("{name} ({DETERMINISM_CHECK})"),
                false,
                DETERMINISM_CHECKS,
            ),
        }
    }
}

/// One corpus test's scan and the checks it was held to.
#[derive(Debug)]
pub struct Judged<'a> {
    /// The repository, as `corpus/known-failures.json` keys it.
    name: &'a str,
    /// The repository's row label in the scorecard.
    label: String,
    /// The manifest the test read.
    manifest: &'a Value,
    /// Whether the record is scored against the repository's clone register.
    scored: bool,
    /// The scan's report, absent when it crashed before writing one.
    report_path: Option<&'a Path>,
    /// What the scan cost, up to its exit.
    cost: &'a ProcessCost,
    /// Every check the test evaluated.
    evaluated: &'a [&'a str],
}

impl<'a> Judged<'a> {
    /// A scan `kind` made of `name`, before its report is known.
    #[must_use]
    pub fn new(kind: Scan, name: &'a str, manifest: &'a Value, cost: &'a ProcessCost) -> Self {
        let (label, scored, evaluated) = kind.identity(name);
        Self {
            name,
            label,
            manifest,
            scored,
            report_path: None,
            cost,
            evaluated,
        }
    }

    /// The same scan, with the report it wrote.
    #[must_use]
    pub fn with_report(self, report_path: &'a Path) -> Self {
        Self {
            report_path: Some(report_path),
            ..self
        }
    }

    /// The scorecard record of this scan, given what its checks found.
    fn record<'r>(
        &'r self,
        run_name: &'r str,
        failures: &[Failure],
        baseline: &Baseline,
    ) -> Record<'r> {
        Record {
            run_name,
            name: &self.label,
            manifest: self.manifest,
            scored: self.scored,
            report_path: self.report_path,
            cost: self.cost,
            checks: check_outcome(
                self.name,
                self.evaluated,
                failures,
                baseline,
                accuracy_curated(self.manifest),
            ),
        }
    }
}

/// [CORPUS-REPORT] Files the scan into the corpus scorecard, then fails the
/// test on whatever the baseline does not excuse. The scorecard is written
/// first, so a failing run still leaves the document that says why.
///
/// # Errors
///
/// Returns an error when the baseline cannot be read or the record cannot be
/// written.
///
/// # Panics
///
/// Fails the calling test — by assertion, as every corpus check does — when a
/// failure is not excused by the baseline.
pub fn publish_then_judge(judged: &Judged<'_>, failures: &[Failure]) -> Result<()> {
    let baseline = Baseline::load()?;
    let run_name = running_test()?;
    let _scorecard = publish(&judged.record(&run_name, failures, &baseline))?;
    fail_on(judged.name, judged.evaluated, failures, &baseline);
    Ok(())
}

/// [CORPUS-REPORT] A scan that crashed is still a result. It is filed with
/// the cost it reached and a failed `scan` check — so the scorecard says an
/// out-of-memory abort happened, and how much memory it took — and the test
/// then fails on it. Any other error is a harness fault and is returned as is.
///
/// # Errors
///
/// Returns `error` itself once the crash is filed, or at once when it is not a
/// crash.
pub fn publish_crash(error: anyhow::Error, kind: Scan, name: &str, manifest: &Value) -> Result<()> {
    let Some(crash) = error.downcast_ref::<ScanCrashed>() else {
        return Err(error);
    };
    let failures = [Failure::new(SCAN_CHECK, crash.reason.clone())];
    let judged = Judged {
        evaluated: SCAN_CHECKS,
        ..Judged::new(kind, name, manifest, &crash.cost)
    };
    publish_then_judge(&judged, &failures)?;
    Err(error)
}

/// The name of the test calling this — `corpus_flutter_dart` — which names its
/// scorecard. libtest runs every test on a thread named after it, so the name
/// is the test's own and can never drift from a copy typed at the call site.
fn running_test() -> Result<String> {
    std::thread::current()
        .name()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            anyhow!("a corpus scorecard is named after its test, but this thread has no name")
        })
}

/// [CORPUS-BASELINE] Classifies observed failures against
/// `corpus/known-failures.json` and fails the test on whatever survives.
/// Strict mode fails on everything; baseline mode fails only on checks that are
/// not already tracked, so CI reports the known defect list without blocking.
fn fail_on(name: &str, evaluated: &[&str], failures: &[Failure], baseline: &Baseline) {
    let fatal = classify(name, evaluated, failures, baseline);
    assert!(
        fatal.is_empty(),
        "{name} corpus gate failed {} {}check(s):\n  - {}",
        fatal.len(),
        if baseline_mode() { "NEW " } else { "" },
        fatal
            .iter()
            .map(|failure| format!("{}: {}", failure.check, failure.detail))
            .collect::<Vec<_>>()
            .join("\n  - ")
    );
}

/// [CORPUS-RECALL] Whether the manifest curates any accuracy claim at all: a
/// duplicate that must be found, or a ranking rule. Without one, a run checks
/// scope and resource ceilings only.
#[must_use]
pub fn accuracy_curated(manifest: &Value) -> bool {
    let recall =
        !array(manifest, "must_find").is_empty() || !array(manifest, "must_find_type2").is_empty();
    recall || manifest.get("must_not_rank_first").is_some()
}
