//! [CORPUS-REPORT-VERDICT] A run's verdict, driven end to end through a run
//! manifest on disk: a clean measured run opens with its passing verdict, and
//! a judged run fails only on a pair that passed before.

use serde_json::{json, Value};

use super::{
    judged_run, one_untracked_failure, report, the_pair, unregistered_run, write, write_scorecard,
    CheckOutcome, Result, WrittenScorecard, CAUTION, CHECKS, FAILED_HEADING, KNOWN_DEFECT,
    MISSED_DEFECT, PASSED_HEADING, PASSING_CHECK, RECORD_FAIL, RECORD_PASS, REPO, RUN, STEM,
    STRICT, UNGATED, UNRECORDED_DETAIL, VERDICT_FAILURES,
};

/// [CORPUS-REPORT-VERDICT] The line the known-defect note lists one untracked
/// failure that never passed on.
fn known_line(what: &str, detail: &str) -> String {
    format!("> - ⚠️ **{REPO}** — `{what}`: {detail}")
}

/// [CORPUS-REPORT-VERDICT] Every check passed, so no failure banner is carried.
#[test]
fn a_run_with_no_failure_opens_with_passed() -> Result<()> {
    let root = tempfile::tempdir()?;
    unregistered_run(root.path())?;
    let clean = CheckOutcome {
        evaluated: vec![PASSING_CHECK.to_owned()],
        accuracy_curated: true,
        failures: Vec::new(),
    };
    write(root.path(), CHECKS, &clean)?;
    let written = write_scorecard(
        &root.path().join(RUN),
        root.path(),
        root.path(),
        STEM,
        UNGATED,
    )?;
    assert!(written.markdown.starts_with(PASSED_HEADING));
    assert!(!written.markdown.contains(CAUTION) && !written.markdown.contains(FAILED_HEADING));
    assert!(written.card.verdict.passed(), "the card records the pass");
    Ok(())
}

/// Scores a judged run of the fixture repository whose scan wrote `scanned`
/// under the strict gate, returning the scorecard and the JSON it left on disk.
fn judged_scorecard(scanned: &Value) -> Result<(WrittenScorecard, Value)> {
    let root = tempfile::tempdir()?;
    judged_run(root.path(), scanned)?;
    let written = write_scorecard(
        &root.path().join(RUN),
        root.path(),
        root.path(),
        STEM,
        STRICT,
    )?;
    let recorded = crate::read_json(&written.json_path)?;
    Ok((written, recorded))
}

/// [CORPUS-REPORT-VERDICT] A judged run is headed by whether anything that
/// passed before fails now, in the markdown and the JSON alike: the same
/// repository passes while its judged pair is reported, and still passes —
/// listing the pair as a known defect — when it is missed with no pass on
/// record, because a pair that never passed cannot regress.
#[test]
fn a_judged_run_fails_only_on_a_pair_that_passed_before() -> Result<()> {
    let (clean, clean_json) = judged_scorecard(&report(&the_pair()))?;
    assert_eq!(
        clean.markdown.lines().next(),
        Some(PASSED_HEADING),
        "a run that kept to its record opens by saying it passed:\n{}",
        clean.markdown
    );
    assert!(
        clean.card.verdict.passed(),
        "the judged pair was reported, so nothing failed"
    );
    assert!(
        clean.markdown.contains(RECORD_PASS),
        "the record section agrees with the heading:\n{}",
        clean.markdown
    );
    assert_eq!(
        clean_json.pointer(VERDICT_FAILURES),
        Some(&json!([])),
        "the JSON scorecard carries the same empty verdict"
    );

    let (missed, missed_json) = judged_scorecard(&report(&[]))?;
    assert_eq!(
        missed.markdown.lines().next(),
        Some(PASSED_HEADING),
        "a missed CLEARLY IN that never passed is no regression, so the run passes:\n{}",
        missed.markdown
    );
    for row in [
        known_line(MISSED_DEFECT, UNRECORDED_DETAIL).as_str(),
        RECORD_PASS,
    ] {
        assert!(
            missed.markdown.contains(row),
            "the scorecard must carry `{row}`:\n{}",
            missed.markdown
        );
    }
    for claim in [FAILED_HEADING, CAUTION, RECORD_FAIL] {
        assert!(
            !missed.markdown.contains(claim),
            "a known defect is never shown as a regression (`{claim}`)"
        );
    }
    assert_eq!(
        missed_json.pointer(VERDICT_FAILURES),
        Some(&one_untracked_failure(
            REPO,
            MISSED_DEFECT,
            UNRECORDED_DETAIL,
            KNOWN_DEFECT
        )),
        "the JSON scorecard lists the pair as a known defect, not a regression"
    );
    Ok(())
}
