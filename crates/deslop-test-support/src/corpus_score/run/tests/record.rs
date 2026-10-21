//! [CORPUS-SCORE-OUTCOMES] The judged pair record, driven end to end: the
//! fixture repository's judged pairs scored run after run in one checkout,
//! through the same run manifest every corpus run writes, with the record on
//! disk read back after each run.

use std::{fs, path::Path};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{
    judged_run, write, ENGINE_LABEL, RECORD_FAIL, RECORD_PASS, REGISTER, REPO, RUN, STEM, STRICT,
};
use crate::corpus_score::{
    outcomes::{Standing, OUTCOMES_PATH},
    run::{write_scorecard, Scoring, WrittenScorecard},
    tests::{
        pinned_sha, report, the_pair,
        verdict::{FAILED_HEADING, PASSED_HEADING},
        FIRST_RANGE, SECOND_RANGE,
    },
    CLEARLY_IN,
};

/// The run manifest field a run's timestamp is read from.
const GENERATED_AT_FIELD: &str = "generated_at";
/// When each run in a sequence was generated.
const FIRST_RUN: &str = "2026-10-01T00:00:00Z";
const SECOND_RUN: &str = "2026-10-02T00:00:00Z";
const THIRD_RUN: &str = "2026-10-03T00:00:00Z";
const FOURTH_RUN: &str = "2026-10-04T00:00:00Z";
const FIFTH_RUN: &str = "2026-10-05T00:00:00Z";
/// A run that puts every unrecorded failure on record.
const ACCEPTING: Scoring = Scoring {
    strict: false,
    accept_failures: true,
};
/// A second judged pair in the same register, sorted as the record keys it.
const THIRD_RANGE: &str = "src/three.rs:50-60";
const FOURTH_RANGE: &str = "src/two.rs:70-80";
/// Another repository the record holds, which a run of the fixture never scores.
const OTHER_REPO: &str = "other";
const OTHER_RANGES: [&str; 2] = ["lib/a.py:1-9", "lib/b.py:1-9"];
/// The engine a record entry from an earlier run names.
const EARLIER_ENGINE: &str = "deslop (binary earlier)";
/// The heading of a pair this run fixed.
const FIXED_LINE: &str = "- ✅ **FIXED** flutter — `src/one.rs:10-20` + `src/two.rs:30-40` — \
                          fixed; on record as failing since 2026-10-02T00:00:00Z";
/// How a tracked pair names the record that tracks it in the caution block.
const TRACKED_PAIR_LINE: &str = "> - 🔴 **flutter** — `false negative` (tracked in \
                                 `corpus/register/outcomes.json`): `src/one.rs:10-20` + \
                                 `src/two.rs:30-40`: on record as failing since \
                                 2026-10-02T00:00:00Z";

/// How the caution block names the fixture pair when it fails off the record,
/// and when it regressed after passing since the first run.
const UNRECORDED_LINE: &str = "> - 🔴 **flutter** — `false negative`: `src/one.rs:10-20` + \
                               `src/two.rs:30-40`: failing, and not on record as failing";
const REGRESSED_LINE: &str = "> - 🔴 **flutter** — `false negative`: `src/one.rs:10-20` + \
                              `src/two.rs:30-40`: regressed; on record as passing since \
                              2026-10-01T00:00:00Z";

/// The fixture's one judged pair, in the order the record keeps it.
fn the_ranges() -> [&'static str; 2] {
    [FIRST_RANGE, SECOND_RANGE]
}

/// Scores one run of the fixture's judged register, generated at `at`, whose
/// scan wrote `scanned`. A register already written under `root` is kept.
fn scored(root: &Path, at: &str, scanned: &Value, scoring: Scoring) -> Result<WrittenScorecard> {
    let register: Value = crate::read_json(&root.join(REGISTER)).unwrap_or(Value::Null);
    judged_run(root, scanned)?;
    if !register.is_null() {
        write(root, REGISTER, &register)?;
    }
    let mut run: Value = crate::read_json(&root.join(RUN))?;
    let _previous = run
        .as_object_mut()
        .ok_or_else(|| anyhow!("the run manifest is not an object"))?
        .insert(GENERATED_AT_FIELD.to_owned(), json!(at));
    write(root, RUN, &run)?;
    write_scorecard(&root.join(RUN), root, root, STEM, scoring)
}

/// The pairs the record on disk holds, or none when no run has written it.
fn recorded(root: &Path) -> Result<Value> {
    let path = root.join(OUTCOMES_PATH);
    if !path.exists() {
        return Ok(json!([]));
    }
    let record: Value = crate::read_json(&path)?;
    record
        .get("pairs")
        .cloned()
        .ok_or_else(|| anyhow!("the record names no pairs: {record}"))
}

/// One record entry for a CLEARLY IN in `repo`.
fn entry(repo: &str, ranges: &[&str], outcome: &str, since: &str, engine: &str) -> Value {
    json!({
        "repo": repo, "verdict": CLEARLY_IN, "occurrences": ranges,
        "outcome": outcome, "since": since, "engine": engine,
    })
}

/// The record holding only the fixture pair, written by this suite's engine.
fn holding(outcome: &str, since: &str) -> Value {
    json!([entry(REPO, &the_ranges(), outcome, since, ENGINE_LABEL)])
}

/// How the one judged pair stood in a run.
fn standing_of(written: &WrittenScorecard) -> Result<Standing> {
    match written.card.outcomes.as_slice() {
        [pair] => Ok(pair.standing),
        pairs => Err(anyhow!("expected one judged pair, scored {}", pairs.len())),
    }
}

/// One run in a sequence over one checkout, and everything it must leave.
struct Step<'a> {
    /// When the run was generated.
    at: &'a str,
    /// Whether its scan reported the judged pair.
    reported: bool,
    /// How it was scored.
    scoring: Scoring,
    /// How the pair must stand, and the scorecard's first line.
    standing: Standing,
    heading: &'a str,
    /// Lines the scorecard must carry.
    carries: &'a [&'a str],
    /// The pairs the record must hold afterwards.
    record: Value,
    /// Why, for the failure message.
    why: &'a str,
}

/// Runs every step in one checkout, asserting each as it goes.
fn run_steps(steps: &[Step<'_>]) -> Result<()> {
    let root = tempfile::tempdir()?;
    steps
        .iter()
        .try_for_each(|step| run_step(root.path(), step))
}

/// Runs one step and asserts its standing, its scorecard and its record.
fn run_step(root: &Path, step: &Step<'_>) -> Result<()> {
    let scanned = report(&if step.reported {
        the_pair()
    } else {
        Vec::new()
    });
    let written = scored(root, step.at, &scanned, step.scoring)?;
    let (at, why) = (step.at, step.why);
    assert_eq!(standing_of(&written)?, step.standing, "{at}: {why}");
    assert_eq!(
        written.markdown.lines().next(),
        Some(step.heading),
        "{at}: {why}"
    );
    for line in step.carries {
        let markdown = &written.markdown;
        assert!(markdown.contains(line), "{at}: {why}: `{line}`\n{markdown}");
    }
    assert_eq!(recorded(root)?, step.record, "{at}: {why}");
    Ok(())
}

#[test]
fn a_passing_pair_goes_on_record_and_a_regression_never_overwrites_it() -> Result<()> {
    run_steps(&[
        Step {
            at: FIRST_RUN,
            reported: true,
            scoring: STRICT,
            standing: Standing::Passing,
            heading: PASSED_HEADING,
            carries: &[RECORD_PASS],
            record: holding("pass", FIRST_RUN),
            why: "a newly judged pair that passes goes on record, stamped with the run and engine",
        },
        Step {
            at: SECOND_RUN,
            reported: false,
            scoring: STRICT,
            standing: Standing::Regressed,
            heading: FAILED_HEADING,
            carries: &[REGRESSED_LINE, RECORD_FAIL],
            record: holding("pass", FIRST_RUN),
            why: "a regression is named with how long the pair had passed, and is never written \
                  over the pass it broke",
        },
        Step {
            at: THIRD_RUN,
            reported: false,
            scoring: ACCEPTING,
            standing: Standing::Regressed,
            heading: FAILED_HEADING,
            carries: &[REGRESSED_LINE],
            record: holding("pass", FIRST_RUN),
            why: "accepting failures never moves a pass to a fail",
        },
        Step {
            at: FOURTH_RUN,
            reported: true,
            scoring: STRICT,
            standing: Standing::Passing,
            heading: PASSED_HEADING,
            carries: &[RECORD_PASS],
            record: holding("pass", FIRST_RUN),
            why: "a pair passing again keeps the date it first passed",
        },
    ])
}

#[test]
fn a_failure_is_tolerated_only_once_accepted_and_a_fix_is_recorded_for_good() -> Result<()> {
    run_steps(&[
        Step {
            at: FIRST_RUN,
            reported: false,
            scoring: STRICT,
            standing: Standing::Unrecorded,
            heading: FAILED_HEADING,
            carries: &[UNRECORDED_LINE, RECORD_FAIL],
            record: json!([]),
            why: "a failure nobody accepted is never put on record by an ordinary run",
        },
        Step {
            at: SECOND_RUN,
            reported: false,
            scoring: ACCEPTING,
            standing: Standing::Unrecorded,
            heading: FAILED_HEADING,
            carries: &[UNRECORDED_LINE],
            record: holding("fail", SECOND_RUN),
            why: "an accepting run puts the failure on record, from the run that accepted it",
        },
        Step {
            at: THIRD_RUN,
            reported: false,
            scoring: STRICT,
            standing: Standing::StillFailing,
            heading: FAILED_HEADING,
            carries: &[TRACKED_PAIR_LINE, RECORD_PASS],
            record: holding("fail", SECOND_RUN),
            why:
                "a tolerated defect still heads the scorecard as failed, is named as tracked, and \
                  keeps the date it started failing",
        },
        Step {
            at: FOURTH_RUN,
            reported: true,
            scoring: STRICT,
            standing: Standing::Fixed,
            heading: PASSED_HEADING,
            carries: &[FIXED_LINE, RECORD_PASS],
            record: holding("pass", FOURTH_RUN),
            why: "a fixed pair goes on record as passing, from the run that fixed it",
        },
        Step {
            at: FIFTH_RUN,
            reported: false,
            scoring: STRICT,
            standing: Standing::Regressed,
            heading: FAILED_HEADING,
            carries: &[RECORD_FAIL],
            record: holding("pass", FOURTH_RUN),
            why: "once fixed, failing again is a regression, not the old tolerated defect",
        },
    ])
}

/// A register judging the fixture pair and a second CLEARLY IN.
fn two_pair_register() -> Value {
    json!({
        "sha": pinned_sha(),
        CLEARLY_IN: [
            { "why": "judged", "occurrences": the_ranges() },
            { "why": "judged", "occurrences": [FOURTH_RANGE, THIRD_RANGE] },
        ],
    })
}

/// Writes `pairs` as the record under `root`.
fn seed(root: &Path, pairs: &Value) -> Result<()> {
    let path = root.join(OUTCOMES_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_string(&json!({ "spec": "", "pairs": pairs }))?,
    )?;
    Ok(())
}

#[test]
fn a_record_tolerates_exactly_the_pairs_it_names() -> Result<()> {
    let root = tempfile::tempdir()?;
    let root = root.path();
    write(root, REGISTER, &two_pair_register())?;
    seed(root, &holding("fail", FIRST_RUN))?;

    let missed = scored(root, SECOND_RUN, &report(&[]), STRICT)?;
    let standings: Vec<Standing> = missed
        .card
        .outcomes
        .iter()
        .map(|pair| pair.standing)
        .collect();
    assert_eq!(
        standings,
        [Standing::StillFailing, Standing::Unrecorded],
        "the record tolerates the pair it names and nothing else in the same repository: \
         tolerance is per pair, so it never widens to cover a second defect"
    );
    assert!(missed.markdown.contains(RECORD_FAIL), "{}", missed.markdown);
    assert_eq!(
        recorded(root)?,
        holding("fail", FIRST_RUN),
        "the unrecorded failure stays off the record"
    );
    Ok(())
}

#[test]
fn a_pair_its_register_drops_leaves_the_record_and_other_repositories_keep_theirs() -> Result<()> {
    let root = tempfile::tempdir()?;
    let root = root.path();
    let reversed = json!({
        "sha": pinned_sha(),
        CLEARLY_IN: [{ "why": "judged", "occurrences": [SECOND_RANGE, FIRST_RANGE] }],
    });
    write(root, REGISTER, &reversed)?;
    let other = entry(OTHER_REPO, &OTHER_RANGES, "fail", FIRST_RUN, EARLIER_ENGINE);
    let dropped = entry(
        REPO,
        &[THIRD_RANGE, FOURTH_RANGE],
        "fail",
        FIRST_RUN,
        EARLIER_ENGINE,
    );
    let failing = entry(REPO, &the_ranges(), "fail", FIRST_RUN, EARLIER_ENGINE);
    seed(root, &json!([other, dropped, failing]))?;

    let fixed = scored(root, SECOND_RUN, &report(&the_pair()), STRICT)?;
    assert_eq!(
        standing_of(&fixed)?,
        Standing::Fixed,
        "the register lists the ranges in another order, and it is still the same pair"
    );
    assert_eq!(
        recorded(root)?,
        json!([
            entry(REPO, &the_ranges(), "pass", SECOND_RUN, ENGINE_LABEL),
            other
        ]),
        "the fixed pair goes on record as passing, the pair the register no longer judges \
         leaves the record, and a repository this run never scored keeps its entry — all in \
         one stable order"
    );
    Ok(())
}
