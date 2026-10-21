//! [CORPUS-REPORT-VERDICT] A clean measured run opens with its passing verdict.

use super::{
    unregistered_run, write, write_scorecard, CheckOutcome, Result, CAUTION, CHECKS,
    FAILED_HEADING, PASSED_HEADING, PASSING_CHECK, RUN, STEM, STRICT,
};

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
        STRICT,
    )?;
    assert!(written.markdown.starts_with(PASSED_HEADING));
    assert!(!written.markdown.contains(CAUTION) && !written.markdown.contains(FAILED_HEADING));
    assert!(written.card.verdict.passed(), "the card records the pass");
    Ok(())
}
