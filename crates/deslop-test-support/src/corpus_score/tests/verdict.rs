//! [CORPUS-REPORT-VERDICT] Whether a run passed is the first thing its
//! scorecard says, and every reason it did not is listed under the heading.
//!
//! The rendering tests next door pin the tables; these pin the verdict above
//! them — which failures it collects and in what order, that a failure the
//! baseline already tracks still fails the run, that only the last engine is
//! judged, and that the heading and the caution block say exactly what the
//! verdict holds, in the markdown and the JSON alike.

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{
    super::{
        checks::{CheckFailure, CheckOutcome},
        gate::Breach,
        verdict::{verdict, FailureReason},
    },
    card::{card, checked_card, NEW_ENGINE, OLD_ENGINE},
    found_and_missed, scorecard, Scorecard, TargetScore, REPO,
};

/// The first line of every scorecard, by verdict, and where the JSON scorecard
/// keeps that verdict's failures. The rendering suite and the end-to-end suite
/// in `run/tests.rs` read these too, so the page a renderer builds and the
/// file a run writes are held to one spelling.
pub(in crate::corpus_score) const PASSED_HEADING: &str = "# ✅ PASSED — Corpus accuracy scorecard";
pub(in crate::corpus_score) const FAILED_HEADING: &str = "# 🔴 FAILED — Corpus accuracy scorecard";
pub(in crate::corpus_score) const VERDICT_FAILURES: &str = "/verdict/failures";
/// The threshold one missed pair breaches under the strict gate, and what the
/// verdict reports of it.
pub(in crate::corpus_score) const BREACHED_MEASURE: &str = "false negatives";
pub(in crate::corpus_score) const BREACH_DETAIL: &str = "allows at most 0, recorded 1";
/// What opens the block that lists a failed run's failures, and the quoted
/// blank line between its summary and the list.
const CAUTION: &str = "> [!CAUTION]";
const QUOTED_BLANK: &str = ">";
/// The block's summary line, by how many failures the run recorded.
const ONE_FAILURE: &str = "> **🔴 FAILED** — **1 failure**. This run did not pass. Every failure \
                           is listed here; the tables below give the detail.";
const THREE_FAILURES: &str = "> **🔴 FAILED** — **3 failures**. This run did not pass. Every \
                              failure is listed here; the tables below give the detail.";
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
const TRACKED_LINE: &str = "> - 🔴 **fixture** — `memory` (tracked in \
                            `corpus/known-failures.json`): peak RSS 9100MB exceeds the 9000MB \
                            ceiling";
const NEW_LINE: &str = "> - 🔴 **fixture** — `wall`: scan took 400s the ceiling is 300s";
const BREACH_LINE: &str = "> - 🔴 **fixture** — `false negatives`: allows at most 0, recorded 1";
/// A second repository, so the order failures are listed in is not an accident
/// of there being one.
const OTHER_REPO: &str = "other";

/// The JSON scorecard's whole list of failures when a run recorded exactly
/// one, and the baseline does not track it.
pub(in crate::corpus_score) fn one_untracked_failure(
    repo: &str,
    what: &str,
    detail: &str,
) -> Value {
    json!([{ "repo": repo, "what": what, "detail": detail, "tracked": false }])
}

/// One reason the fixture repository failed.
fn reason(what: &str, detail: &str, tracked: bool) -> FailureReason {
    FailureReason {
        repo: REPO.to_owned(),
        what: what.to_owned(),
        detail: detail.to_owned(),
        tracked,
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
/// counts the failures in `summary` and lists one of `failures` per line.
fn failed_opening<'a>(summary: &'a str, failures: &[&'a str]) -> Vec<&'a str> {
    let block = [FAILED_HEADING, "", CAUTION, summary, QUOTED_BLANK];
    [block.as_slice(), failures, [""].as_slice()].concat()
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
fn a_breached_gate_fails_the_run_and_is_named_under_the_heading() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let breached = card(&found, &missed);
    assert!(
        !breached.verdict.passed(),
        "a breached gate is a failed run"
    );
    assert_eq!(
        breached.verdict.failures,
        [reason(BREACHED_MEASURE, BREACH_DETAIL, false)],
        "the breach is the one reason: the threshold, what it allows and what was recorded"
    );
    assert_opens_with(&breached, &failed_opening(ONE_FAILURE, &[BREACH_LINE]));
    let rendered = scorecard(&breached);
    for claim in [PASSED_HEADING, PASS_MARK] {
        assert!(
            !rendered.contains(claim),
            "a failed scorecard must not say `{claim}`:\n{rendered}"
        );
    }
    assert_eq!(
        serde_json::to_value(&breached)?.pointer(VERDICT_FAILURES),
        Some(&one_untracked_failure(
            REPO,
            BREACHED_MEASURE,
            BREACH_DETAIL
        )),
        "the JSON scorecard lists the same failure the heading does"
    );
    Ok(())
}

#[test]
fn every_failed_check_is_listed_before_the_gate_breaches() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let checks = checked_by(NEW_ENGINE, vec![tracked_failure(), new_failure()]);
    let failing = checked_card(&found, &missed, checks);
    assert_eq!(
        failing.verdict.failures,
        [
            reason(TRACKED_CHECK, TRACKED_DETAIL, true),
            reason(NEW_CHECK, NEW_DETAIL, false),
            reason(BREACHED_MEASURE, BREACH_DETAIL, false),
        ],
        "every failed check in the order the test recorded it, then every gate breach"
    );
    let listed = [TRACKED_LINE, NEW_LINE, BREACH_LINE];
    assert_opens_with(&failing, &failed_opening(THREE_FAILURES, &listed));
    Ok(())
}

#[test]
fn a_failure_the_baseline_tracks_still_fails_the_run_and_says_it_is_tracked() -> Result<()> {
    let (found, _) = found_and_missed()?;
    let tracked_only = checked_by(NEW_ENGINE, vec![tracked_failure()]);
    let failing = checked_card(&found, &found, tracked_only);
    assert!(
        failing.breaches.is_empty(),
        "the register gate is clean, so the tracked check is the only thing wrong"
    );
    assert_eq!(
        failing.verdict.failures,
        [reason(TRACKED_CHECK, TRACKED_DETAIL, true)],
        "a tracked failure is still a failure; the flag only says it is not new"
    );
    assert_opens_with(&failing, &failed_opening(ONE_FAILURE, &[TRACKED_LINE]));
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
        [reason(NEW_CHECK, NEW_DETAIL, false)],
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

/// The fixture repository's breach, recorded against another repository too.
fn other_breach(card: &Scorecard) -> Result<Breach> {
    let first = card
        .breaches
        .first()
        .ok_or_else(|| anyhow!("the fixture card breaches no gate"))?;
    Ok(Breach {
        repo: OTHER_REPO.to_owned(),
        ..first.clone()
    })
}

#[test]
fn failures_are_listed_repository_by_repository_checks_before_breaches() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let first = checked_card(
        &found,
        &missed,
        checked_by(NEW_ENGINE, vec![tracked_failure()]),
    );
    let targets = [first.targets.clone(), vec![other_target(&first)?]].concat();
    let breaches = [first.breaches.clone(), vec![other_breach(&first)?]].concat();
    let judged = verdict(&first.engines, &targets, &breaches);
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
            (REPO, BREACHED_MEASURE),
            (OTHER_REPO, BREACHED_MEASURE),
        ],
        "check failures in repository order, then gate breaches in repository order"
    );
    Ok(())
}
