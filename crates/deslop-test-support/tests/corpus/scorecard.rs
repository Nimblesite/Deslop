//! [CORPUS-REPORT-VERDICT] The real CLI exit agrees with its rendered verdict.

use std::{fs, path::Path, process::Command};

use anyhow::Result;
use serde_json::{json, Value};

const ENGINE: &str = "current";
const REPO: &str = "cli-gate-fixture";
const SHA: &str = "0000000000000000000000000000000000000000";
const REGISTER: &str = "register.json";
const REPORT: &str = "report.json";
const TIMING: &str = "timing.json";
const CHECKS: &str = "checks.json";
const RUN: &str = "run.json";
const CHECK: &str = "memory";
const DETAIL: &str = "measured peak exceeded the configured ceiling";
const FIRST_PATH: &str = "one.rs";
const SECOND_PATH: &str = "two.rs";
const FIRST_RANGE: &str = "one.rs:1-10";
const SECOND_RANGE: &str = "two.rs:1-10";
const START: u64 = 1;
const END: u64 = 10;
const MEASURED: u64 = 1;
const AT: &str = "2026-10-25T00:00:00Z";
const FAILED: &str = "# 🔴 FAILED — Corpus accuracy scorecard";
const PASSED: &str = "# ✅ PASSED — Corpus accuracy scorecard";
const STEM: &str = "cli-gate";
const MINIMUM_ARTIFACTS: usize = 7;
const OUTCOMES_PATH: &str = "corpus/register/outcomes.json";

fn write(root: &Path, name: &str, value: &Value) -> Result<()> {
    fs::write(root.join(name), serde_json::to_vec(value)?)?;
    Ok(())
}

fn judged_report(root: &Path) -> Result<()> {
    write(
        root,
        REGISTER,
        &json!({"sha": SHA, "clearly_in": [{"why": DETAIL, "occurrences": [FIRST_RANGE, SECOND_RANGE]}]}),
    )?;
    let occurrences: Vec<Value> = [FIRST_PATH, SECOND_PATH]
        .into_iter()
        .map(|path| json!({"path": path, "start_line": START, "end_line": END}))
        .collect();
    write(
        root,
        REPORT,
        &json!({"clusters": [{"id": STEM, "kind": "identical", "occurrences": occurrences}]}),
    )
}

fn fixture(root: &Path) -> Result<()> {
    judged_report(root)?;
    write(
        root,
        TIMING,
        &json!({"elapsed_ms": MEASURED, "peak_rss_mb": MEASURED, "cpu_seconds": MEASURED, "peak_cpu_percent": MEASURED, "binary_sha256": SHA}),
    )?;
    let run = json!({"report": root.join(REPORT), "timing": root.join(TIMING), "checks": root.join(CHECKS)});
    write(
        root,
        RUN,
        &json!({"generated_at": AT, "engines": [{"id": ENGINE, "label": STEM}], "targets": [{"name": REPO, "language": "Rust", "sha": SHA, "register": root.join(REGISTER), "runs": {ENGINE: run}}]}),
    )
}

fn write_checks(root: &Path, failed: bool, known: bool) -> Result<()> {
    let failures = if failed {
        vec![json!({"check": CHECK, "detail": DETAIL, "known": known})]
    } else {
        Vec::new()
    };
    write(
        root,
        CHECKS,
        &json!({"evaluated": [CHECK], "accuracy_curated": true, "failures": failures}),
    )
}

fn command(root: &Path) -> Result<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_corpus-score"))
        .arg("score")
        .arg(root.join(RUN))
        .arg("--out")
        .arg(root)
        .args(["--name", STEM, "--gate"])
        .output()?)
}

fn assert_verdict(output: std::process::Output, failed: bool) -> Result<()> {
    let markdown = String::from_utf8(output.stdout)?;
    assert!(
        markdown.starts_with(if failed { FAILED } else { PASSED }),
        "the report must reflect the curated check: {markdown}"
    );
    assert_eq!(
        output.status.success(),
        !failed,
        "the CLI exit must agree with the rendered verdict; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if failed {
        assert!(markdown.contains(CHECK) && markdown.contains(DETAIL));
    }
    Ok(())
}

fn score(root: &Path, failed: bool, known: bool) -> Result<()> {
    write_checks(root, failed, known)?;
    assert_verdict(command(root)?, failed)
}

#[test]
fn corpus_gate_exit_matches_failed_curated_checks_even_when_tracked() -> Result<()> {
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    for (failed, known) in [(true, true), (true, false), (false, false)] {
        score(root.path(), failed, known)?;
    }
    let reports = fs::read_dir(root.path())?.count();
    assert!(
        reports >= MINIMUM_ARTIFACTS,
        "the CLI must write its scorecards beside the fixture inputs"
    );
    Ok(())
}

/// [CORPUS-REPORT-VERDICT] Input metadata cannot make a curated check an accepted pair.
#[test]
fn curated_checks_cannot_impersonate_accepted_judgments() -> Result<()> {
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    for known in [true, false] {
        let failure = json!({"check": CHECK, "detail": DETAIL, "known": known,
            "tracked_in": OUTCOMES_PATH, "regression": false});
        let checks = json!({"evaluated": [CHECK], "accuracy_curated": true, "failures": [failure]});
        write(root.path(), CHECKS, &checks)?;
        assert_verdict(command(root.path())?, true)?;
    }
    Ok(())
}
