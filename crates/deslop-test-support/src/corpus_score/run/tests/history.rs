//! [CORPUS-HISTORY] The run history, driven end to end: the fixture
//! repository's judged pair scored run after run in one checkout, with the
//! history file on disk read back after each run.

use std::{fs, path::Path};

use anyhow::{anyhow, Result};

use super::{
    record::{holding, scored, seed, FIRST_RUN, FOURTH_RUN, SECOND_RUN, THIRD_RUN},
    ELAPSED_MS, PEAK_RSS_MB, REPO, STRICT,
};
use crate::corpus_score::{
    history::{self, Host, RunRecord, Source, BACKSLIDE, HISTORY_PATH},
    outcomes::{Outcome, Standing},
    run::{Scoring, WrittenScorecard},
    tests::{
        pinned_sha, report, the_pair,
        verdict::{FAILED_HEADING, PASSED_HEADING},
        FIRST_RANGE, SECOND_RANGE,
    },
    CLEARLY_IN,
};

/// The commit the fixture engine is recorded as built from.
const SOURCE_COMMIT: &str = "1111111111111111111111111111111111111111";
/// A commit the fixture repository was never scanned at.
const OTHER_SHA: &str = "2222222222222222222222222222222222222222";
/// The scorecard lines the history section must carry.
const SECTION: &str = "## Run history";
const SOURCE_LINE: &str =
    "Source: commit `1111111111111111111111111111111111111111`, with uncommitted changes.";
const NO_EARLIER_RUNS: &str =
    "**PASS** — no judged pair went backwards against **0 earlier runs**.";
const NONE_AGAINST_TWO: &str =
    "**PASS** — no judged pair went backwards against **2 earlier runs**.";
const ONE_AGAINST_ONE: &str = "**FAIL** — **1 judged pair** went backwards against **1 earlier \
                               run**. Each passed in an earlier run of the same commit and fails \
                               now:";
const ONE_AGAINST_TWO: &str = "**FAIL** — **1 judged pair** went backwards against **2 earlier \
                               runs**. Each passed in an earlier run of the same commit and \
                               fails now:";
const BACKSLIDE_LINE: &str = "- 🔴 **flutter** — `src/one.rs:10-20` + `src/two.rs:30-40` — false \
                              negative; passed in the run of 2026-10-01T00:00:00Z";
const CAUTION_LINE: &str = "> - 🔴 **flutter** — `backslide: false negative`: `src/one.rs:10-20` \
                            + `src/two.rs:30-40`: passed in the run of 2026-10-01T00:00:00Z, \
                            fails now";
const MATCHED_ROW: &str =
    "| flutter (dart) | ✅ yes | 100.0% | 1/1 | 0/0 | 295.00 s | 1210.50 s | 781% | 7947 MB |";
const MISSED_ROW: &str =
    "| flutter (dart) | 🔴 no | 0.0% | 0/1 | 0/0 | 295.00 s | 1210.50 s | 781% | 7947 MB |";

/// The source every history run in this suite is built from.
fn source() -> Source {
    Source {
        commit: SOURCE_COMMIT.to_owned(),
        dirty: true,
    }
}

/// One run of the fixture held to the history, whose scan reported the pair
/// when `reported`.
fn run(root: &Path, at: &str, reported: bool) -> Result<WrittenScorecard> {
    let pairs = if reported { the_pair() } else { Vec::new() };
    let source = source();
    let scoring = Scoring {
        history: Some(&source),
        ..STRICT
    };
    scored(root, at, &report(&pairs), scoring)
}

/// Every run the history under `root` holds.
fn runs(root: &Path) -> Result<Vec<RunRecord>> {
    history::load(root)
}

/// The last run the history holds.
fn last(root: &Path) -> Result<RunRecord> {
    runs(root)?
        .pop()
        .ok_or_else(|| anyhow!("the history holds no run"))
}

/// Asserts the scorecard carries every line in `lines`.
fn carries(written: &WrittenScorecard, lines: &[&str]) {
    for line in lines {
        let markdown = &written.markdown;
        assert!(markdown.contains(line), "missing `{line}`\n{markdown}");
    }
}

/// Asserts one history run: when it ran, whether it passed, how its one pair
/// came out, and which earlier run it backslid against.
fn assert_run(
    record: &RunRecord,
    at: &str,
    outcome: Outcome,
    backslid_since: Option<&str>,
) -> Result<()> {
    assert_eq!(
        record.generated_at, at,
        "the run is stamped with when it ran"
    );
    assert_eq!(
        record.passed,
        backslid_since.is_none() && outcome == Outcome::Pass,
        "{at}"
    );
    assert_eq!(
        record.source,
        source(),
        "{at}: the source commit and tree are recorded"
    );
    assert_eq!(
        record.host,
        Host::this_machine(),
        "{at}: the machine is recorded"
    );
    let [judgment] = record.judgments.as_slice() else {
        return Err(anyhow!(
            "{at}: expected one judged pair, recorded {:?}",
            record.judgments
        ));
    };
    assert_eq!(judgment.key.repo, REPO, "{at}");
    assert_eq!(judgment.key.verdict, CLEARLY_IN, "{at}");
    assert_eq!(
        judgment.key.occurrences,
        [FIRST_RANGE, SECOND_RANGE],
        "{at}"
    );
    assert_eq!(
        judgment.outcome, outcome,
        "{at}: every judged pair is recorded"
    );
    let since: Vec<&str> = record
        .backslides
        .iter()
        .map(|slide| slide.passed_at.as_str())
        .collect();
    assert_eq!(
        since,
        backslid_since.into_iter().collect::<Vec<_>>(),
        "{at}"
    );
    Ok(())
}

/// Asserts the one repository a history run recorded, and what it cost.
fn assert_repo(record: &RunRecord, matched: bool) -> Result<()> {
    let [repo] = record.repos.as_slice() else {
        return Err(anyhow!(
            "expected one repository, recorded {}",
            record.repos.len()
        ));
    };
    assert_eq!(repo.name, REPO);
    assert_eq!(
        repo.matches_judgments, matched,
        "whether it matched its judgments"
    );
    assert_eq!((repo.judged, repo.correct), (1, usize::from(matched)));
    assert_eq!(
        (repo.clearly_in_total, repo.clearly_in_found),
        (1, usize::from(matched))
    );
    assert_eq!(repo.false_negatives, usize::from(!matched));
    let cost = repo
        .cost
        .as_ref()
        .ok_or_else(|| anyhow!("no cost recorded"))?;
    assert_eq!(
        (cost.elapsed_ms, cost.peak_rss_mb),
        (ELAPSED_MS, Some(PEAK_RSS_MB))
    );
    Ok(())
}

#[test]
fn a_pass_on_record_in_the_history_holds_every_later_run_of_that_commit() -> Result<()> {
    let root = tempfile::tempdir()?;
    let root = root.path();

    let first = run(root, FIRST_RUN, true)?;
    assert_eq!(first.markdown.lines().next(), Some(PASSED_HEADING));
    carries(
        &first,
        &[SECTION, SOURCE_LINE, NO_EARLIER_RUNS, MATCHED_ROW],
    );
    assert_eq!(history::backslid(&first.card), 0);
    assert_run(&last(root)?, FIRST_RUN, Outcome::Pass, None)?;
    assert_repo(&last(root)?, true)?;

    let second = run(root, SECOND_RUN, false)?;
    assert_eq!(second.markdown.lines().next(), Some(FAILED_HEADING));
    carries(
        &second,
        &[ONE_AGAINST_ONE, BACKSLIDE_LINE, CAUTION_LINE, MISSED_ROW],
    );
    assert_eq!(
        history::backslid(&second.card),
        1,
        "one pair going backwards fails the run"
    );
    assert!(
        second
            .card
            .verdict
            .failures
            .iter()
            .any(|failure| failure.what.starts_with(BACKSLIDE)),
        "the verdict lists the backslide"
    );
    assert_run(&last(root)?, SECOND_RUN, Outcome::Fail, Some(FIRST_RUN))?;
    assert_repo(&last(root)?, false)?;
    assert_eq!(runs(root)?.len(), 2, "a failed run is recorded too");
    Ok(())
}

#[test]
fn a_backslide_fails_the_run_even_when_the_record_tolerates_the_pair() -> Result<()> {
    let root = tempfile::tempdir()?;
    let root = root.path();
    let _first = run(root, FIRST_RUN, true)?;
    let _second = run(root, SECOND_RUN, false)?;
    seed(root, &holding("fail", SECOND_RUN))?;

    let third = run(root, THIRD_RUN, false)?;
    let [pair] = third.card.outcomes.as_slice() else {
        return Err(anyhow!("expected one judged pair"));
    };
    assert_eq!(
        pair.standing,
        Standing::StillFailing,
        "the record tolerates the pair"
    );
    assert_eq!(history::backslid(&third.card), 1, "the history does not");
    assert!(!history::kept(&third.card), "so the run fails");
    carries(&third, &[ONE_AGAINST_TWO, BACKSLIDE_LINE, CAUTION_LINE]);
    assert_run(&last(root)?, THIRD_RUN, Outcome::Fail, Some(FIRST_RUN))?;
    Ok(())
}

#[test]
fn a_pass_at_another_commit_holds_nothing_and_a_run_off_the_history_writes_none() -> Result<()> {
    let root = tempfile::tempdir()?;
    let root = root.path();
    let unrecorded = scored(root, FIRST_RUN, &report(&the_pair()), STRICT)?;
    assert!(
        !root.join(HISTORY_PATH).exists(),
        "a run off the history appends nothing"
    );
    assert!(
        !unrecorded.markdown.contains(SECTION),
        "and shows no history section"
    );

    let _first = run(root, FIRST_RUN, true)?;
    let _second = run(root, SECOND_RUN, false)?;
    let path = root.join(HISTORY_PATH);
    let repinned = fs::read_to_string(&path)?.replace(&pinned_sha(), OTHER_SHA);
    fs::write(&path, repinned)?;
    seed(root, &holding("fail", SECOND_RUN))?;

    let fourth = run(root, FOURTH_RUN, false)?;
    assert_eq!(
        history::backslid(&fourth.card),
        0,
        "a pass at another commit is not this pair"
    );
    assert!(
        history::kept(&fourth.card),
        "with the pair on record as failing, the run keeps"
    );
    carries(&fourth, &[NONE_AGAINST_TWO]);
    assert_eq!(
        runs(root)?.len(),
        3,
        "every run held to the history is appended as one line"
    );
    Ok(())
}
