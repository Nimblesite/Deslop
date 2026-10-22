//! [CORPUS-REPORT-VERDICT] Whether a run passed is the first thing its
//! scorecard says, and every reason it did not is listed under the heading.
//!
//! The rendering tests next door pin the tables; these pin the verdict above
//! them — which failures it collects and in what order, that only a
//! regression fails the run while a failure that never passed is listed as a
//! known defect, that only the last engine is
//! judged, and that the heading and the caution block say exactly what the
//! verdict holds, in the markdown and the JSON alike.

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{
    super::{
        checks::{CheckFailure, CheckOutcome},
        outcomes::{Ledger, Outcome, PairKey, PairOutcome, RecordedPair},
        verdict::{verdict, FailureReason},
        RepoScore,
    },
    card::{card, checked_card, recorded_card, NEW_ENGINE, NEW_LABEL, OLD_ENGINE},
    found_and_missed, scorecard, Scorecard, TargetScore, REPO,
};

/// The first line of every scorecard, by verdict, and where the JSON scorecard
/// keeps that verdict's failures. The rendering suite and the end-to-end suite
/// in `run/tests.rs` read these too, so the page a renderer builds and the
/// file a run writes are held to one spelling.
pub(in crate::corpus_score) const PASSED_HEADING: &str = "# ✅ PASSED — Corpus accuracy scorecard";
pub(in crate::corpus_score) const FAILED_HEADING: &str = "# 🔴 FAILED — Corpus accuracy scorecard";
pub(in crate::corpus_score) const VERDICT_FAILURES: &str = "/verdict/failures";
/// What one missed pair is called, and what the verdict says of it when the
/// record does not hold it as failing — the strict default.
pub(in crate::corpus_score) const MISSED_DEFECT: &str = "false negative";
pub(in crate::corpus_score) const UNRECORDED_DETAIL: &str =
    "`src/one.rs:10-20` + `src/two.rs:30-40`: failing, and not on record as failing";
/// The records a tracked failure names.
pub(in crate::corpus_score) const KNOWN_FAILURES: &str = "corpus/known-failures.json";
/// What opens the block that lists a failed run's failures, and the quoted
/// blank line between its summary and the list.
pub(in crate::corpus_score) const CAUTION: &str = "> [!CAUTION]";
const QUOTED_BLANK: &str = ">";
/// The block's summary line, by how many regressions the run recorded.
const ONE_REGRESSION: &str = "> **🔴 FAILED** — **1 regression**: something that passed before \
                              fails now. Every regression is listed here; the tables below give \
                              the detail.";
/// What opens the note that lists known defects, and its summary line.
const NOTE: &str = "> [!NOTE]";
const ONE_KNOWN_DEFECT: &str = "> ⚠️ **1 known defect** — none of them passed before, so none is \
                                a regression and none fails the run. Each is still a bug.";
const TWO_KNOWN_DEFECTS: &str = "> ⚠️ **2 known defects** — none of them passed before, so none \
                                 is a regression and none fails the run. Each is still a bug.";
/// When the fixture pair went on record as passing, and how the verdict says
/// it regressed from that pass.
const PASSED_SINCE: &str = "2026-10-01T00:00:00Z";
const PASSED_COMMIT: &str = "1111111111111111111111111111111111111111";
const REGRESSED_DETAIL: &str = "`src/one.rs:10-20` + `src/two.rs:30-40`: regressed; on record as \
                                passing since 2026-10-01T00:00:00Z";
const REGRESSED_LINE: &str = "> - 🔴 **fixture** — `false negative`: `src/one.rs:10-20` + \
                              `src/two.rs:30-40`: regressed; on record as passing since \
                              2026-10-01T00:00:00Z";
/// The marks a verdict is found by at a glance; neither may appear under the
/// other's heading.
const FAILURE_MARK: &str = "🔴";
const PASS_MARK: &str = "✅";
/// What follows the verdict: the document proper.
const AFTER_VERDICT: &str = "Generated ";
/// The curated checks the fixture test evaluated: one passed, one failed and
/// is tracked in `corpus/known-failures.json`, one failed and is new.
const PASSING_CHECK: &str = "recall";
const TRACKED_CHECK: &str = "memory";
const NEW_CHECK: &str = "wall";
const TRACKED_DETAIL: &str = "peak RSS 9100MB exceeds the 9000MB ceiling";
/// A detail that spans two lines; the caution block carries it on one.
const NEW_DETAIL: &str = "scan took 400s\nthe ceiling is 300s";
/// One line per failure, as the caution block lists them.
const TRACKED_LINE: &str = "> - ⚠️ **fixture** — `memory` (tracked in \
                            `corpus/known-failures.json`): peak RSS 9100MB exceeds the 9000MB \
                            ceiling";
const NEW_LINE: &str = "> - 🔴 **fixture** — `wall`: scan took 400s the ceiling is 300s";
pub(in crate::corpus_score) const UNRECORDED_LINE: &str =
    "> - ⚠️ **fixture** — `false negative`: `src/one.rs:10-20` + \
                               `src/two.rs:30-40`: failing, and not on record as failing";
/// A second repository, so the order failures are listed in is not an accident
/// of there being one.
const OTHER_REPO: &str = "other";

/// The JSON scorecard's whole list of failures when a run recorded exactly
/// one, the baseline does not track it, and it is a `regression` or not.
pub(in crate::corpus_score) fn one_untracked_failure(
    repo: &str,
    what: &str,
    detail: &str,
    regression: bool,
) -> Value {
    json!([{
        "repo": repo, "what": what, "detail": detail, "tracked_in": null,
        "regression": regression,
    }])
}

/// Whether a listed failure fails the run.
pub(in crate::corpus_score) const REGRESSION: bool = true;
/// See [`REGRESSION`].
pub(in crate::corpus_score) const KNOWN_DEFECT: bool = false;

/// One reason the fixture repository failed.
fn reason(what: &str, detail: &str, tracked_in: Option<&str>, regression: bool) -> FailureReason {
    FailureReason {
        repo: REPO.to_owned(),
        what: what.to_owned(),
        detail: detail.to_owned(),
        tracked_in: tracked_in.map(ToOwned::to_owned),
        regression,
    }
}

/// A record holding every judged pair of `score` as passing since
/// [`PASSED_SINCE`].
fn passing_record(score: &RepoScore) -> Ledger {
    Ledger {
        spec: String::new(),
        pairs: score
            .entries
            .iter()
            .map(|entry| RecordedPair {
                key: PairKey::of(REPO, entry),
                outcome: Outcome::Pass,
                since: PASSED_SINCE.to_owned(),
                engine: NEW_LABEL.to_owned(),
                commit: PASSED_COMMIT.to_owned(),
                version: None,
            })
            .collect(),
    }
}

/// The failure the baseline already tracks.
fn tracked_failure() -> CheckFailure {
    CheckFailure {
        check: TRACKED_CHECK.to_owned(),
        detail: TRACKED_DETAIL.to_owned(),
        known: true,
    }
}

/// The failure nothing tracks.
fn new_failure() -> CheckFailure {
    CheckFailure {
        check: NEW_CHECK.to_owned(),
        detail: NEW_DETAIL.to_owned(),
        known: false,
    }
}

/// The curated checks `engine_id` evaluated, with the `failures` it recorded.
fn checked_by(engine_id: &str, failures: Vec<CheckFailure>) -> BTreeMap<String, CheckOutcome> {
    let outcome = CheckOutcome {
        evaluated: [PASSING_CHECK, TRACKED_CHECK, NEW_CHECK]
            .map(str::to_owned)
            .into(),
        accuracy_curated: true,
        failures,
    };
    BTreeMap::from([(engine_id.to_owned(), outcome)])
}

/// How a failed scorecard opens: the heading, then the caution block that
/// counts the new failures in `summary` and lists one of `failures` per line.
fn failed_opening<'a>(summary: &'a str, failures: &[&'a str]) -> Vec<&'a str> {
    let block = [FAILED_HEADING, "", CAUTION, summary, QUOTED_BLANK];
    [block.as_slice(), failures, [""].as_slice()].concat()
}

/// The note that follows the heading: `summary` counts the known defects and
/// one of `defects` is listed per line.
fn known_note<'a>(summary: &'a str, defects: &[&'a str]) -> Vec<&'a str> {
    let block = [NOTE, summary, QUOTED_BLANK];
    [block.as_slice(), defects, [""].as_slice()].concat()
}

/// Asserts the scorecard opens with exactly `expected` and the document proper
/// follows at once: the verdict comes before anything else, so every assertion
/// about it reads the page from its first line.
fn assert_opens_with(card: &Scorecard, expected: &[&str]) {
    let rendered = scorecard(card);
    let opening: Vec<&str> = rendered.lines().take(expected.len()).collect();
    assert_eq!(
        opening, expected,
        "the verdict is the first thing the scorecard says:\n{rendered}"
    );
    assert!(
        rendered
            .lines()
            .nth(expected.len())
            .is_some_and(|line| line.starts_with(AFTER_VERDICT)),
        "the document proper follows the verdict, with nothing in between:\n{rendered}"
    );
}

#[test]
fn a_run_that_failed_nothing_is_headed_passed_and_lists_no_failure() -> Result<()> {
    let (found, _) = found_and_missed()?;
    let clean = card(&found, &found);
    assert!(clean.verdict.passed(), "nothing failed, so the run passed");
    assert_eq!(
        clean.verdict.failures,
        Vec::new(),
        "a pass carries no reason"
    );
    assert_opens_with(&clean, &[PASSED_HEADING, ""]);
    let rendered = scorecard(&clean);
    for claim in [CAUTION, FAILED_HEADING, FAILURE_MARK] {
        assert!(
            !rendered.contains(claim),
            "a passing scorecard must not say `{claim}`:\n{rendered}"
        );
    }
    assert_eq!(
        serde_json::to_value(&clean)?.pointer(VERDICT_FAILURES),
        Some(&json!([])),
        "the JSON scorecard carries the same empty verdict"
    );
    Ok(())
}

#[test]
fn a_pair_that_passed_before_and_fails_now_fails_the_run_and_is_named_under_the_heading(
) -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let regressed = recorded_card(&found, &missed, BTreeMap::new(), &passing_record(&found));
    assert!(
        !regressed.verdict.passed(),
        "a pair on record as passing that fails now is a regression, and fails the run"
    );
    assert_eq!(
        regressed.verdict.failures,
        [reason(MISSED_DEFECT, REGRESSED_DETAIL, None, REGRESSION)],
        "the pair is the one reason: what it is, its ranges and the pass it broke"
    );
    assert_opens_with(
        &regressed,
        &failed_opening(ONE_REGRESSION, &[REGRESSED_LINE]),
    );
    let rendered = scorecard(&regressed);
    for claim in [PASSED_HEADING, PASS_MARK] {
        assert!(
            !rendered.contains(claim),
            "a failed scorecard must not say `{claim}`:\n{rendered}"
        );
    }
    assert_eq!(
        serde_json::to_value(&regressed)?.pointer(VERDICT_FAILURES),
        Some(&one_untracked_failure(
            REPO,
            MISSED_DEFECT,
            REGRESSED_DETAIL,
            REGRESSION
        )),
        "the JSON scorecard lists the same regression the heading does"
    );
    Ok(())
}

#[test]
fn a_pair_that_never_passed_is_a_known_defect_and_fails_nothing() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let never_passed = card(&found, &missed);
    assert!(
        never_passed.verdict.passed(),
        "a pair with no pass on record has nothing to regress from, so the run passes"
    );
    assert_eq!(
        never_passed.verdict.failures,
        [reason(MISSED_DEFECT, UNRECORDED_DETAIL, None, KNOWN_DEFECT)],
        "it is still listed: what it is, its ranges and how it stands against the record"
    );
    let opening = [
        vec![PASSED_HEADING, ""],
        known_note(ONE_KNOWN_DEFECT, &[UNRECORDED_LINE]),
    ]
    .concat();
    assert_opens_with(&never_passed, &opening);
    assert!(
        !scorecard(&never_passed).contains(CAUTION),
        "a known defect is never shown as a regression"
    );
    assert_eq!(
        serde_json::to_value(&never_passed)?.pointer(VERDICT_FAILURES),
        Some(&one_untracked_failure(
            REPO,
            MISSED_DEFECT,
            UNRECORDED_DETAIL,
            KNOWN_DEFECT
        )),
        "the JSON scorecard lists it as a known defect, not a regression"
    );
    Ok(())
}

#[test]
fn every_failed_check_is_listed_before_the_judged_pairs() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let checks = checked_by(NEW_ENGINE, vec![tracked_failure(), new_failure()]);
    let failing = checked_card(&found, &missed, checks);
    assert_eq!(
        failing.verdict.failures,
        [
            reason(
                TRACKED_CHECK,
                TRACKED_DETAIL,
                Some(KNOWN_FAILURES),
                KNOWN_DEFECT
            ),
            reason(NEW_CHECK, NEW_DETAIL, None, REGRESSION),
            reason(MISSED_DEFECT, UNRECORDED_DETAIL, None, KNOWN_DEFECT),
        ],
        "every failed check in the order the test recorded it, then every judged pair"
    );
    assert!(
        !failing.verdict.passed(),
        "the untracked check is a regression; the tracked check and the never-passed pair are not"
    );
    let opening = [
        failed_opening(ONE_REGRESSION, &[NEW_LINE]),
        known_note(TWO_KNOWN_DEFECTS, &[TRACKED_LINE, UNRECORDED_LINE]),
    ]
    .concat();
    assert_opens_with(&failing, &opening);
    Ok(())
}

#[test]
fn a_failure_the_baseline_tracks_is_a_known_defect_that_fails_nothing() -> Result<()> {
    let (found, _) = found_and_missed()?;
    let tracked_only = checked_by(NEW_ENGINE, vec![tracked_failure()]);
    let known = checked_card(&found, &found, tracked_only);
    assert!(
        known
            .outcomes
            .iter()
            .all(|pair| !pair.standing.fails_the_run()),
        "every judged pair is answered right, so the tracked check is the only thing wrong"
    );
    assert_eq!(
        known.verdict.failures,
        [reason(
            TRACKED_CHECK,
            TRACKED_DETAIL,
            Some(KNOWN_FAILURES),
            KNOWN_DEFECT
        )],
        "a tracked failure is still listed, with the record that tracks it"
    );
    assert!(
        known.verdict.passed(),
        "it is not a regression, so the run passes"
    );
    let opening = [
        vec![PASSED_HEADING, ""],
        known_note(ONE_KNOWN_DEFECT, &[TRACKED_LINE]),
    ]
    .concat();
    assert_opens_with(&known, &opening);
    Ok(())
}

#[test]
fn only_the_last_engine_is_judged() -> Result<()> {
    let (found, _) = found_and_missed()?;
    let reference = checked_card(&found, &found, checked_by(OLD_ENGINE, vec![new_failure()]));
    assert_eq!(
        reference.verdict.failures,
        Vec::new(),
        "the first engine is the reference a comparison reads against, not the build on trial"
    );
    let on_trial = checked_card(&found, &found, checked_by(NEW_ENGINE, vec![new_failure()]));
    assert_eq!(
        on_trial.verdict.failures,
        [reason(NEW_CHECK, NEW_DETAIL, None, REGRESSION)],
        "the same failure on the last engine fails the run"
    );
    Ok(())
}

/// The fixture repository's row under another name, failing a check of its own.
fn other_target(card: &Scorecard) -> Result<TargetScore> {
    let first = card
        .targets
        .first()
        .ok_or_else(|| anyhow!("the fixture card names no target"))?;
    Ok(TargetScore {
        name: OTHER_REPO.to_owned(),
        checks: checked_by(NEW_ENGINE, vec![new_failure()]),
        ..first.clone()
    })
}

/// The fixture repository's missed pair, missed in another repository too.
fn other_pair(card: &Scorecard) -> Result<PairOutcome> {
    let mut other = card
        .outcomes
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("the fixture card judges no pair"))?;
    other.key.repo = OTHER_REPO.to_owned();
    Ok(other)
}

#[test]
fn failures_are_listed_repository_by_repository_checks_before_judged_pairs() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let first = checked_card(
        &found,
        &missed,
        checked_by(NEW_ENGINE, vec![tracked_failure()]),
    );
    let targets = [first.targets.clone(), vec![other_target(&first)?]].concat();
    let pairs = [first.outcomes.clone(), vec![other_pair(&first)?]].concat();
    let judged = verdict(&first.engines, &targets, &pairs);
    let listed: Vec<(&str, &str)> = judged
        .failures
        .iter()
        .map(|failure| (failure.repo.as_str(), failure.what.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            (REPO, TRACKED_CHECK),
            (OTHER_REPO, NEW_CHECK),
            (REPO, MISSED_DEFECT),
            (OTHER_REPO, MISSED_DEFECT),
        ],
        "check failures in repository order, then judged pairs in repository order"
    );
    Ok(())
}
