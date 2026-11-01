//! [CORPUS-REPORT-VERDICT] Incorrect answers require explicit acceptance.

use super::super::{
    register, report, score_repo, the_pair, CLEARLY_OUT, FIRST_RANGE, REPO, SECOND_RANGE,
};
use super::{
    card, checked_by, checked_card, found_and_missed, scorecard, tracked_failure, Result,
    Scorecard, FAILED_HEADING, KNOWN_FAILURES, MISSED_DEFECT, NEW_ENGINE, TRACKED_CHECK,
};

const FALSE_POSITIVE: &str = "false positive";
const ONE_FAILURE: usize = 1;

fn assert_failed_gate(card: &Scorecard, what: &str) {
    let listed: Vec<&str> = card
        .verdict
        .failures
        .iter()
        .map(|failure| failure.what.as_str())
        .collect();
    assert_eq!(listed, [what], "the gate must retain the exact failure");
    assert!(
        !card.verdict.passed(),
        "an unaccepted failure must fail the corpus gate"
    );
    assert!(
        scorecard(card).starts_with(FAILED_HEADING),
        "the rendered scorecard must head this run FAILED"
    );
}

#[test]
fn an_unrecorded_missed_pair_fails_the_gate() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    assert_eq!(missed.false_negatives, ONE_FAILURE);
    assert_failed_gate(&card(&found, &missed), MISSED_DEFECT);
    Ok(())
}

#[test]
fn an_unrecorded_reported_clearly_out_pair_fails_the_gate() -> Result<()> {
    let judged = register(CLEARLY_OUT, &[FIRST_RANGE, SECOND_RANGE]);
    let reported = score_repo(REPO, &judged, &report(&the_pair()))?;
    assert_eq!(reported.false_positives, ONE_FAILURE);
    assert_failed_gate(&card(&reported, &reported), FALSE_POSITIVE);
    Ok(())
}

#[test]
fn a_tracked_failed_check_still_fails_the_gate() -> Result<()> {
    let (found, _) = found_and_missed()?;
    let checked = checked_card(
        &found,
        &found,
        checked_by(NEW_ENGINE, vec![tracked_failure()]),
    );
    let tracked: Vec<Option<&str>> = checked
        .verdict
        .failures
        .iter()
        .map(|failure| failure.tracked_in.as_deref())
        .collect();
    assert_eq!(tracked, [Some(KNOWN_FAILURES)]);
    assert_failed_gate(&checked, TRACKED_CHECK);
    Ok(())
}
