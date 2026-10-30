//! [CORPUS-HISTORY] Persist the same verdict without inventing prior passes.

use super::{
    judged_run, last, manifest, pinned_sha, report, run, the_pair, write, write_scorecard,
    CheckFailure, CheckOutcome, Path, Result, RunRecord, WrittenScorecard, CHECKS, FAILED_HEADING,
    FAILING_CHECK, FAILURE_DETAIL, FIRST_RUN, HELD, REGISTER, REPORT, RUN, STEM, TIMING,
};
use serde_json::json;

const NO_REGRESSIONS: usize = 0;
const ONE_DEFECT: usize = 1;

fn assert_unaccepted_history(written: &WrittenScorecard, recorded: &RunRecord) {
    assert!(!written.card.verdict.passed());
    assert_eq!(recorded.passed, written.card.verdict.passed());
    assert_eq!(
        recorded.regressions, NO_REGRESSIONS,
        "no earlier pass exists"
    );
    assert_eq!(
        recorded.known_defects, ONE_DEFECT,
        "the incorrect judgment is still counted"
    );
    assert!(recorded
        .judgments
        .iter()
        .all(|judgment| !judgment.regression));
}

#[test]
fn history_records_unaccepted_failure_without_calling_it_a_regression() -> Result<()> {
    let root = tempfile::tempdir()?;
    let written = run(root.path(), FIRST_RUN, false)?;
    assert_unaccepted_history(&written, &last(root.path())?);
    Ok(())
}

fn tracked_checks() -> CheckOutcome {
    CheckOutcome {
        evaluated: vec![FAILING_CHECK.to_owned()],
        accuracy_curated: true,
        failures: vec![CheckFailure {
            check: FAILING_CHECK.to_owned(),
            detail: FAILURE_DETAIL.to_owned(),
            known: true,
        }],
    }
}

fn checked_run(root: &Path) -> Result<WrittenScorecard> {
    judged_run(root, &report(&the_pair()))?;
    write(root, CHECKS, &tracked_checks())?;
    let engine_run = json!({"report": REPORT, "timing": TIMING, "checks": CHECKS});
    write(
        root,
        RUN,
        &manifest(&pinned_sha(), Some(REGISTER), &engine_run),
    )?;
    write_scorecard(&root.join(RUN), root, root, STEM, HELD)
}

#[test]
fn history_records_tracked_check_failure_with_the_scorecard_verdict() -> Result<()> {
    let root = tempfile::tempdir()?;
    let written = checked_run(root.path())?;
    let recorded = last(root.path())?;
    assert!(
        !written.card.verdict.passed(),
        "the tracked failed check blocks the scorecard"
    );
    assert_eq!(
        recorded.passed,
        written.card.verdict.passed(),
        "history must persist the same verdict"
    );
    assert!(written.markdown.starts_with(FAILED_HEADING));
    Ok(())
}
