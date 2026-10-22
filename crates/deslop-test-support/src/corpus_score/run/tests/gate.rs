//! [CORPUS-SCORE-GATE-JUDGED] A gated run holds the engine to judged pairs, so
//! a gate that scored none — no repository scanned, or none a register judges
//! — is refused outright instead of being headed PASSED.

use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::json;

use super::{
    judged_run, unregistered_run, write, write_scorecard, Scoring, WrittenScorecard, ENGINE_ID,
    ENGINE_LABEL, GENERATED_AT, PASSED_HEADING, REGISTER, RUN, STEM, STRICT, UNGATED,
};
use crate::corpus_score::{
    run::nothing_judged,
    tests::{register, report, the_pair, FIRST_RANGE, SECOND_RANGE},
    CLEARLY_IN,
};

/// The verdict a register holding only undecided pairs records them under.
const NOT_CLEAR: &str = "not_clear";

/// Scores the run manifest under `root` as `scoring` asks.
fn scored(root: &Path, scoring: Scoring) -> Result<WrittenScorecard> {
    write_scorecard(&root.join(RUN), root, root, STEM, scoring)
}

/// The gate refuses the run under `root`, which scanned `scanned`
/// repositories, naming why — and leaves no scorecard that could read as a
/// pass.
fn assert_refused(root: &Path, scanned: usize) -> Result<()> {
    let Err(refusal) = scored(root, STRICT) else {
        return Err(anyhow!(
            "a gate that judged no pair passed over {scanned} repositories"
        ));
    };
    assert_eq!(refusal.to_string(), nothing_judged(scanned));
    assert!(
        !root.join(format!("{STEM}.md")).exists() && !root.join(format!("{STEM}.json")).exists(),
        "a refused gate writes no scorecard"
    );
    Ok(())
}

#[test]
fn a_gate_that_judged_no_pair_fails_instead_of_passing() -> Result<()> {
    let empty = tempfile::tempdir()?;
    let no_targets = json!({
        "generated_at": GENERATED_AT,
        "engines": [{ "id": ENGINE_ID, "label": ENGINE_LABEL }],
        "targets": [],
    });
    write(empty.path(), RUN, &no_targets)?;
    assert_refused(empty.path(), 0)?;

    let unjudged = tempfile::tempdir()?;
    unregistered_run(unjudged.path())?;
    assert_refused(unjudged.path(), 1)?;
    let reported = scored(unjudged.path(), UNGATED)?;
    assert_eq!(
        reported.card.targets.len(),
        1,
        "an ungated corpus test still reports the repository no register judges"
    );
    assert!(reported.path.exists(), "and writes its scorecard");

    let undecided = tempfile::tempdir()?;
    judged_run(undecided.path(), &report(&the_pair()))?;
    let undecided_register = register(NOT_CLEAR, &[FIRST_RANGE, SECOND_RANGE]);
    write(undecided.path(), REGISTER, &undecided_register)?;
    assert_refused(undecided.path(), 1)?;

    let judged = tempfile::tempdir()?;
    judged_run(judged.path(), &report(&the_pair()))?;
    let passed = scored(judged.path(), STRICT)?;
    assert_eq!(passed.markdown.lines().next(), Some(PASSED_HEADING));
    assert_eq!(
        passed.card.outcomes.len(),
        1,
        "the gate scored the judged pair"
    );
    assert_eq!(
        passed
            .card
            .outcomes
            .first()
            .map(|pair| pair.key.verdict.as_str()),
        Some(CLEARLY_IN)
    );
    Ok(())
}
