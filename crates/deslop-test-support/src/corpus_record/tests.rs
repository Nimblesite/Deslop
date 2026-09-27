//! [CORPUS-REPORT] How a corpus test's scan is filed into its scorecard.

use std::{path::Path, time::Duration};

use anyhow::Result;
use serde_json::{json, Value};

use super::{check_outcome, run_manifest, Record, CHECKS_FILE, REPORT_FILE, TIMING_FILE};
use crate::{
    corpus::{Baseline, Failure},
    corpus_measure::ProcessCost,
    corpus_score::checks::CheckOutcome,
};

const REPO: &str = "fsharp";
const TRACKED_CHECK: &str = "memory";
const UNTRACKED_CHECK: &str = "wall";
const PASSING_CHECK: &str = "recall";
const RUN_NAME: &str = "corpus_flutter_dart";
const LABEL: &str = "flutter";
const LANGUAGE: &str = "dart";
const SHA: &str = "67323de285b00232883f53b84095eb72be97d35c";
const BINARY_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ENGINE_ID: &str = "aaaaaaaaaaaa";
const RUN_DIR: &str = ".corpus/test-corpus/corpus_flutter_dart-2026-09-27T03-35-02Z";
const SCANNED_REPORT: &str = "flutter.json";
const REGISTER: &str = "corpus/register/flutter.json";

/// A failure is marked known exactly when `corpus/known-failures.json` tracks
/// that check for that repository; the rest are regressions.
#[test]
fn a_tracked_failure_is_known_and_an_untracked_one_is_new() {
    let baseline = Baseline::parse(&json!({ "known_failures": { REPO: [TRACKED_CHECK] } }));
    let failures = [
        Failure::new(TRACKED_CHECK, "peak over ceiling"),
        Failure::new(UNTRACKED_CHECK, "scan over ceiling"),
    ];
    let outcome = check_outcome(
        REPO,
        &[PASSING_CHECK, TRACKED_CHECK, UNTRACKED_CHECK],
        &failures,
        &baseline,
        true,
    );
    let known: Vec<(&str, bool)> = outcome
        .failures
        .iter()
        .map(|failure| (failure.check.as_str(), failure.known))
        .collect();
    assert_eq!(
        known,
        [(TRACKED_CHECK, true), (UNTRACKED_CHECK, false)],
        "known follows the baseline, check by check"
    );
    assert_eq!(outcome.passed(), 1, "only the check with no failure passed");
    assert_eq!(outcome.new_failures(), 1, "one failure is untracked");
    assert_eq!(
        outcome.evaluated.len(),
        3,
        "every evaluated check is recorded"
    );
}

/// The run manifest one record renders from.
fn manifest_for(report_path: Option<&Path>, scored: bool) -> Result<Value> {
    let manifest = json!({ "name": LABEL, "language": LANGUAGE, "sha": SHA });
    let cost = ProcessCost {
        wall: Duration::from_secs(1),
        peak_rss_mb: 1,
        cpu_seconds: 1.0,
        peak_cpu_percent: None,
    };
    let record = Record {
        run_name: RUN_NAME,
        name: LABEL,
        manifest: &manifest,
        scored,
        report_path,
        cost: &cost,
        checks: CheckOutcome::default(),
    };
    run_manifest(&record, RUN_DIR, BINARY_SHA)
}

/// Every file a completed scan's manifest names sits in the run's own
/// directory, relative to the repository root as the scorer reads it, and the
/// engine is named by the fingerprint of the binary that ran.
#[test]
fn a_completed_scan_names_its_report_cost_and_checks_in_its_own_directory() -> Result<()> {
    let run = manifest_for(Some(Path::new(SCANNED_REPORT)), true)?;
    assert_eq!(
        run.pointer("/engines/0/id"),
        Some(&Value::from(ENGINE_ID)),
        "the engine is the fingerprint's first twelve characters"
    );
    for (field, name) in [
        ("report", REPORT_FILE),
        ("timing", TIMING_FILE),
        ("checks", CHECKS_FILE),
    ] {
        assert_eq!(
            run.pointer(&format!("/targets/0/runs/{ENGINE_ID}/{field}")),
            Some(&Value::from(format!("{RUN_DIR}/{name}"))),
            "the {field} lives in this run's directory, never a shared one"
        );
    }
    assert_eq!(
        run.pointer("/targets/0/register"),
        Some(&Value::from(REGISTER)),
        "a scored scan is scored against its repository's register"
    );
    Ok(())
}

/// A crashed scan names no report rather than a file that does not exist, and
/// still carries the cost it reached. A second scan of a repository is never
/// scored against its register again.
#[test]
fn a_crashed_scan_names_no_report_and_keeps_its_cost() -> Result<()> {
    let run = manifest_for(None, false)?;
    assert_eq!(
        run.pointer(&format!("/targets/0/runs/{ENGINE_ID}/report")),
        Some(&Value::Null),
        "no report was written, so none is named"
    );
    assert_eq!(
        run.pointer(&format!("/targets/0/runs/{ENGINE_ID}/timing")),
        Some(&Value::from(format!("{RUN_DIR}/{TIMING_FILE}"))),
        "the cost it reached is still named"
    );
    assert_eq!(
        run.pointer("/targets/0/register"),
        Some(&Value::Null),
        "an unscored scan names no register"
    );
    Ok(())
}
