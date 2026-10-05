//! [CORPUS-REPORT] The scorecard every corpus run writes, driven end to end
//! through a run manifest on disk — the same entry `score-gate.sh`,
//! `compare-versions.sh` and the `corpus_*` tests all use.

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::{json, Value};

use super::{engine_run, report_stem, write_scorecard, RunContext, WrittenScorecard};
use crate::corpus_score::{
    checks::{CheckFailure, CheckOutcome},
    tests::{
        pinned_sha, register, report, the_pair,
        verdict::{
            one_untracked_failure, BREACHED_MEASURE, BREACH_DETAIL, FAILED_HEADING, PASSED_HEADING,
            VERDICT_FAILURES,
        },
        FIRST_RANGE, SECOND_RANGE,
    },
    RunCost, CLEARLY_IN,
};

const ENGINE_ID: &str = "current";
const ENGINE_LABEL: &str = "deslop (binary current)";
const REPO: &str = "flutter";
const LANGUAGE: &str = "dart";
const SHA: &str = "67323de285b00232883f53b84095eb72be97d35c";
const REPORT: &str = "report.json";
const TIMING: &str = "timing.json";
const CHECKS: &str = "checks.json";
const ABSENT_TIMING: &str = "absent-timing.json";
const RUN: &str = "run.json";
/// The register a judged fixture run is scored against, and a register path
/// nobody wrote, which leaves the repository unjudged.
const REGISTER: &str = "register.json";
const ABSENT_REGISTER: &str = "corpus/register/flutter.json";
/// When the fixture runs were scored.
const GENERATED_AT: &str = "2026-09-27T00:00:00Z";
/// The name a fixture run's scorecard is written under.
const STEM: &str = STAMPED_STEM;
/// A moment, and the name a run of `corpus_flutter_dart` at it is written as.
const STAMPED_SECONDS: u64 = 1_790_480_102;
const STAMPED_STEM: &str = "corpus_flutter_dart-2026-09-27T03-35-02Z";
const RUN_NAME: &str = "corpus_flutter_dart";
/// What the fixture scan cost.
const ELAPSED_MS: u64 = 295_000;
const CPU_SECONDS: f64 = 1_210.5;
const PEAK_CPU_PERCENT: f64 = 781.0;
const PEAK_RSS_MB: u64 = 7_947;
/// The curated checks it evaluated, and the one that failed.
const PASSING_CHECK: &str = "recall";
const FAILING_CHECK: &str = "memory";
const FAILURE_DETAIL: &str = "peak RSS 9100MB exceeds the 9000MB ceiling";
/// The rows the scorecard must carry for that scan.
const ACCURACY_ROW: &str = "| flutter (dart) | no register | — | — | — | — |";
const CHECKS_ROW: &str = "| flutter (dart) | 1/2 pass | `memory` (NEW) |";
const COST_ROW: &str = "| flutter (dart) | 2 | 295.00 s | 1210.50 s | 781% | 7947 MB |";
const WALL_ROW: &str = "| wall | 295.00 s |";
const CPU_TIME_ROW: &str = "| CPU time | 1210.50 s |";
const PEAK_CPU_ROW: &str = "| peak CPU | 781% |";
const PEAK_MEMORY_ROW: &str = "| peak memory | 7947 MB |";
const CHECKS_STANDING_ROW: &str = "| curated checks passed | 1/2 |";
const NEW_FAILURES_ROW: &str = "| new curated check failures | 1 |";
const SCOPE_LINE: &str = "Scope: **1 repository** across **1 language** — dart.";

/// The fixture's measured cost.
fn measured() -> RunCost {
    RunCost {
        elapsed_ms: ELAPSED_MS,
        peak_rss_mb: Some(PEAK_RSS_MB),
        cpu_seconds: Some(CPU_SECONDS),
        peak_cpu_percent: Some(PEAK_CPU_PERCENT),
        binary_sha256: ENGINE_ID.to_owned(),
    }
}

/// The fixture's curated checks: one passed, one failed and untracked.
fn checked() -> CheckOutcome {
    CheckOutcome {
        evaluated: vec![PASSING_CHECK.to_owned(), FAILING_CHECK.to_owned()],
        accuracy_curated: true,
        failures: vec![CheckFailure {
            check: FAILING_CHECK.to_owned(),
            detail: FAILURE_DETAIL.to_owned(),
            known: false,
        }],
    }
}

/// Writes `value` as JSON under `root`.
fn write(root: &Path, name: &str, value: &impl serde::Serialize) -> Result<()> {
    fs::write(root.join(name), serde_json::to_string(value)?)?;
    Ok(())
}

/// A run manifest for one scan of the fixture repository at `sha`: the
/// register it is scored against, when it names one, and the files the scan
/// left behind.
fn manifest(sha: &str, register: Option<&str>, run: &Value) -> Value {
    json!({
        "generated_at": GENERATED_AT,
        "engines": [{ "id": ENGINE_ID, "label": ENGINE_LABEL }],
        "targets": [{
            "name": REPO, "language": LANGUAGE, "sha": sha, "register": register,
            "runs": { ENGINE_ID: run },
        }],
    })
}

/// A run manifest naming one unregistered repository with a report of two
/// clusters, its measured cost and its curated checks.
fn unregistered_run(root: &Path) -> Result<()> {
    write(root, REPORT, &json!({ "clusters": [{}, {}] }))?;
    write(root, TIMING, &measured())?;
    write(root, CHECKS, &checked())?;
    let run = json!({ "report": REPORT, "timing": TIMING, "checks": CHECKS });
    write(root, RUN, &manifest(SHA, Some(ABSENT_REGISTER), &run))
}

/// [CORPUS-REPORT-VERDICT] The line a failed run's verdict lists one failure
/// the baseline does not track on.
fn failure_line(what: &str, detail: &str) -> String {
    format!("> - 🔴 **{REPO}** — `{what}`: {detail}")
}

/// [CORPUS-REPORT] A repository no register judged is still reported — its
/// accuracy as curated checks, its wall time, CPU time, peak CPU and peak
/// memory — rather than dropped from the scorecard, which is what a flutter
/// run used to produce: nothing at all.
#[test]
fn an_unregistered_repository_reports_its_checks_and_its_cost() -> Result<()> {
    let root = tempfile::tempdir()?;
    unregistered_run(root.path())?;
    let written = write_scorecard(&root.path().join(RUN), root.path(), root.path(), STEM, true)?;
    assert_eq!(
        written.path,
        root.path().join(format!("{STEM}.md")),
        "the markdown is named after the run"
    );
    let markdown = fs::read_to_string(&written.path)?;
    assert_eq!(
        markdown, written.markdown,
        "the markdown scorecard is what was rendered"
    );
    assert_eq!(
        markdown.lines().next(),
        Some(FAILED_HEADING),
        "a run with a failed check opens by saying it failed:\n{markdown}"
    );
    for row in [
        failure_line(FAILING_CHECK, FAILURE_DETAIL).as_str(),
        SCOPE_LINE,
        ACCURACY_ROW,
        CHECKS_ROW,
        COST_ROW,
        WALL_ROW,
        CPU_TIME_ROW,
        PEAK_CPU_ROW,
        PEAK_MEMORY_ROW,
        CHECKS_STANDING_ROW,
        NEW_FAILURES_ROW,
    ] {
        assert!(
            markdown.contains(row),
            "the scorecard must carry `{row}`:\n{markdown}"
        );
    }
    assert!(
        markdown.contains(FAILURE_DETAIL),
        "a failing check names what it saw:\n{markdown}"
    );
    assert!(
        written.card.thresholds.is_empty() && written.card.breaches.is_empty(),
        "an unjudged repository has no register gate to breach"
    );
    let recorded: Value = crate::read_json(&root.path().join(format!("{STEM}.json")))?;
    assert_eq!(
        recorded.pointer("/totals/current/peak_cpu_percent"),
        Some(&Value::from(PEAK_CPU_PERCENT)),
        "the JSON scorecard carries the same peak CPU the markdown shows"
    );
    assert_eq!(
        recorded.pointer("/targets/0/registered"),
        Some(&Value::Bool(false)),
        "the JSON scorecard says the repository was not judged"
    );
    assert_eq!(
        recorded.pointer(VERDICT_FAILURES),
        Some(&one_untracked_failure(REPO, FAILING_CHECK, FAILURE_DETAIL)),
        "the JSON scorecard lists the failure the heading reports"
    );
    Ok(())
}

/// A run context over `root` for the fixture engine.
fn context(root: &Path, require_timing: bool) -> RunContext<'_> {
    RunContext {
        root,
        engine_id: ENGINE_ID,
        name: REPO,
        require_timing,
    }
}

/// [CORPUS-SCORE-COST-COMPLETE] Missing required cost fails the strict gate.
#[test]
fn registered_run_with_missing_timing_fails_scoring() -> Result<()> {
    let root = tempfile::tempdir()?;
    write(root.path(), REPORT, &json!({ "clusters": [] }))?;
    let run = json!({ "report": REPORT, "timing": ABSENT_TIMING });
    let register = json!({ "clearly_in": [], "clearly_out": [] });
    let result = engine_run(&run, Some(&register), context(root.path(), true));
    assert!(
        result.is_err(),
        "a missing timing file must never yield a green scorecard"
    );
    Ok(())
}

/// [CORPUS-SCORE-COST-COMPLETE] Accuracy-only scoring accepts an unmeasured run.
#[test]
fn unmeasured_run_keeps_accuracy_score_without_a_timing_field() -> Result<()> {
    let root = tempfile::tempdir()?;
    write(root.path(), REPORT, &json!({ "clusters": [] }))?;
    let run = json!({ "report": REPORT });
    let register = json!({ "clearly_in": [], "clearly_out": [] });
    let result = engine_run(&run, Some(&register), context(root.path(), false))?;
    assert_eq!(
        result.score.map(|score| score.correct),
        Some(0),
        "empty register has no judged pairs"
    );
    assert!(result.cost.is_none(), "unmeasured run has no timing cost");
    Ok(())
}

/// The check a crashed scan fails, and what its record says: the reason it
/// stopped.
const SCAN_CHECK: &str = "scan";
const CRASH_REASON: &str =
    "deslop exited Some(-1073740791): memory allocation of 25769803776 bytes failed";
/// The rows a crashed scan must still produce.
const CRASHED_CHECKS_ROW: &str = "| flutter (dart) | 0/1 pass | `scan` (NEW) |";
const CRASHED_COST_ROW: &str = "| flutter (dart) | — | 295.00 s | 1210.50 s | 781% | 7947 MB |";
/// A crash published no clusters; the total says it was not counted, never zero.
const UNCOUNTED_CLUSTERS_ROW: &str = "| clusters | — |";
/// What the gate and defect list say when nothing in the run was judged.
const NO_REGISTER_GATE: &str = "**No register gate**";
const NO_JUDGED_PAIRS: &str = "No judged pairs";
/// What they must never say about a run that judged nothing.
const FALSE_PASS: &str = "**PASS**";
const FALSE_CLEAN: &str = "Every judged pair is answered correctly";

/// [CORPUS-REPORT] A scan that crashed wrote no report, and is reported
/// anyway: its failed `scan` check with the reason, and the wall time, CPU
/// and peak memory it reached. A crash that left no row would read as a
/// repository nobody scanned.
#[test]
fn a_crashed_scan_is_reported_with_its_cost_and_its_reason() -> Result<()> {
    let root = tempfile::tempdir()?;
    write(root.path(), TIMING, &measured())?;
    let crashed = CheckOutcome {
        evaluated: vec![SCAN_CHECK.to_owned()],
        accuracy_curated: true,
        failures: vec![CheckFailure {
            check: SCAN_CHECK.to_owned(),
            detail: CRASH_REASON.to_owned(),
            known: false,
        }],
    };
    write(root.path(), CHECKS, &crashed)?;
    let run = json!({ "report": null, "timing": TIMING, "checks": CHECKS });
    write(root.path(), RUN, &manifest(SHA, None, &run))?;
    let written = write_scorecard(&root.path().join(RUN), root.path(), root.path(), STEM, true)?;
    assert_eq!(
        written.markdown.lines().next(),
        Some(FAILED_HEADING),
        "a crashed scan fails the run, and the scorecard opens by saying so:\n{}",
        written.markdown
    );
    for row in [
        failure_line(SCAN_CHECK, CRASH_REASON).as_str(),
        SCOPE_LINE,
        CRASHED_CHECKS_ROW,
        CRASHED_COST_ROW,
        PEAK_MEMORY_ROW,
        CRASH_REASON,
        UNCOUNTED_CLUSTERS_ROW,
        NO_REGISTER_GATE,
        NO_JUDGED_PAIRS,
    ] {
        assert!(
            written.markdown.contains(row),
            "the scorecard must carry `{row}`:\n{}",
            written.markdown
        );
    }
    for claim in [FALSE_PASS, FALSE_CLEAN] {
        assert!(
            !written.markdown.contains(claim),
            "a run that judged nothing must not claim `{claim}`:\n{}",
            written.markdown
        );
    }
    Ok(())
}

/// A strict gate refuses a judged run with no report: an unscored register
/// must never read as one nobody breached.
#[test]
fn a_judged_run_with_no_report_fails_the_strict_gate() -> Result<()> {
    let root = tempfile::tempdir()?;
    write(root.path(), TIMING, &measured())?;
    let run = json!({ "timing": TIMING });
    let register = json!({ "clearly_in": [], "clearly_out": [] });
    let result = engine_run(&run, Some(&register), context(root.path(), true));
    assert!(
        result.is_err(),
        "a judged run without a report must fail the strict gate"
    );
    let relaxed = engine_run(&run, Some(&register), context(root.path(), false))?;
    assert!(
        relaxed.score.is_none() && relaxed.clusters.is_none(),
        "outside the strict gate it is reported unscored, never as a clean score"
    );
    assert!(relaxed.cost.is_some(), "its cost is still read");
    Ok(())
}

/// What the register gate says, by outcome.
const GATE_PASS: &str = "**PASS** — every scored repository is inside its gate.";
const GATE_FAIL: &str = "**FAIL** — 1 breach(es).";

/// A run manifest naming the fixture repository with a register that judges
/// one CLEARLY IN pair, the report its scan wrote and its measured cost.
fn judged_run(root: &Path, scanned: &Value) -> Result<()> {
    let judged = register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]);
    write(root, REGISTER, &judged)?;
    write(root, REPORT, scanned)?;
    write(root, TIMING, &measured())?;
    let run = json!({ "report": REPORT, "timing": TIMING });
    write(root, RUN, &manifest(&pinned_sha(), Some(REGISTER), &run))
}

/// Scores a judged run of the fixture repository whose scan wrote `scanned`
/// under the strict gate, returning the scorecard and the JSON it left on disk.
fn judged_scorecard(scanned: &Value) -> Result<(WrittenScorecard, Value)> {
    let root = tempfile::tempdir()?;
    judged_run(root.path(), scanned)?;
    let written = write_scorecard(&root.path().join(RUN), root.path(), root.path(), STEM, true)?;
    let recorded = crate::read_json(&written.json_path)?;
    Ok((written, recorded))
}

/// [CORPUS-REPORT-VERDICT] A judged run is headed by whether the engine
/// cleared its register gate, in the markdown and the JSON alike: the same
/// repository passes while its judged pair is reported, and fails — naming the
/// threshold — the moment it is not.
#[test]
fn a_judged_run_is_headed_by_whether_it_cleared_its_register_gate() -> Result<()> {
    let (clean, clean_json) = judged_scorecard(&report(&the_pair()))?;
    assert_eq!(
        clean.markdown.lines().next(),
        Some(PASSED_HEADING),
        "a run inside its gate opens by saying it passed:\n{}",
        clean.markdown
    );
    assert!(
        clean.card.verdict.passed(),
        "the judged pair was reported, so nothing failed"
    );
    assert!(
        clean.markdown.contains(GATE_PASS),
        "the register gate agrees with the heading:\n{}",
        clean.markdown
    );
    assert_eq!(
        clean_json.pointer(VERDICT_FAILURES),
        Some(&json!([])),
        "the JSON scorecard carries the same empty verdict"
    );

    let (breached, breached_json) = judged_scorecard(&report(&[]))?;
    assert_eq!(
        breached.markdown.lines().next(),
        Some(FAILED_HEADING),
        "a missed CLEARLY IN breaches the gate, and the scorecard opens by saying so:\n{}",
        breached.markdown
    );
    for row in [
        failure_line(BREACHED_MEASURE, BREACH_DETAIL).as_str(),
        GATE_FAIL,
    ] {
        assert!(
            breached.markdown.contains(row),
            "a failed scorecard must carry `{row}`:\n{}",
            breached.markdown
        );
    }
    assert!(
        !breached.markdown.contains(PASSED_HEADING),
        "a breached gate is never also headed as a pass"
    );
    assert_eq!(
        breached_json.pointer(VERDICT_FAILURES),
        Some(&one_untracked_failure(
            REPO,
            BREACHED_MEASURE,
            BREACH_DETAIL
        )),
        "the JSON scorecard lists the breach the heading reports"
    );
    Ok(())
}

/// [CORPUS-REPORT] Every run's scorecard is named after what ran and when, so
/// two runs never write the same file, and the name is a legal file name on
/// Windows — which has no room for the `:` a timestamp carries.
#[test]
fn a_scorecard_is_named_after_its_run_and_the_moment_it_ran() {
    let moment = std::time::UNIX_EPOCH + std::time::Duration::from_secs(STAMPED_SECONDS);
    let stem = report_stem(RUN_NAME, moment);
    assert_eq!(
        stem, STAMPED_STEM,
        "the run's name, then the UTC second it ran"
    );
    assert!(!stem.contains(':'), "a file name Windows accepts: {stem}");
    let later = report_stem(RUN_NAME, moment + std::time::Duration::from_secs(1));
    assert_ne!(stem, later, "a run a second later writes a different file");
    assert!(
        stem < later,
        "the names sort in the order the runs happened"
    );
}
