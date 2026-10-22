//! [CORPUS-HISTORY] The run history, driven end to end: the fixture
//! repository's judged pair scored run after run in one checkout, with every
//! run record and the rendered `HISTORY.md` on disk read back after each run.

use std::{fs, path::Path};

use anyhow::{anyhow, Result};

use super::{
    record::{
        holding, scored, scored_as, seed, the_engine, FIRST_RUN, FOURTH_RUN, SECOND_RUN, THIRD_RUN,
    },
    write, ELAPSED_MS, PEAK_RSS_MB, REPO, RUN, SOURCE_COMMIT, STEM, STRICT,
};
use crate::corpus_score::{
    history::{
        self,
        store::{self, LOG_PATH, RUNS_DIR},
        Host, RunRecord, BACKSLIDE,
    },
    outcomes::{Outcome, Standing},
    run::{write_scorecard, Scoring, WrittenScorecard},
    source::Source,
    tests::{
        pinned_sha, report, the_pair,
        verdict::{FAILED_HEADING, PASSED_HEADING},
        FIRST_RANGE, SECOND_RANGE,
    },
    CLEARLY_IN,
};

/// A release the fixture engine is built from in one run.
const RELEASE: &str = "v9.9.9";
/// The run manifest field an engine's commit is read from, and what a run
/// held to the history says when its engine names no commit.
const COMMIT: &str = "commit";
const NO_COMMIT: &str = "must name the commit it was built from";
/// A commit the fixture repository was never scanned at.
const OTHER_SHA: &str = "2222222222222222222222222222222222222222";
/// The scorecard lines the history section must carry.
const SECTION: &str = "## Run history";
const SOURCE_LINE: &str = "Source: commit `1111111111111111111111111111111111111111`.";
const RELEASE_LINE: &str = "Source: **v9.9.9**, commit `1111111111111111111111111111111111111111`.";
/// How `HISTORY.md` lists each run: its time, its source, its result, its
/// score and its cost. The machine follows, and is this machine's.
const LOG_FIRST_ROW: &str =
    "| 2026-10-01T00:00:00Z | `111111111111` | ✅ passed | 1/1 | 100.0% | 0 \
                             | 0 | 295.00 s | 1210.50 s | 7947 MB |";
const LOG_SECOND_ROW: &str =
    "| 2026-10-02T00:00:00Z | `111111111111` | 🔴 failed | 0/1 | 0.0% | 1 \
                              | 0 | 295.00 s | 1210.50 s | 7947 MB |";
const LOG_RELEASE_ROW: &str =
    "| 2026-10-04T00:00:00Z | **v9.9.9** (`111111111111`) | ✅ passed | 0/1 \
                               | 0.0% | 0 | 1 | 295.00 s | 1210.50 s | 7947 MB |";
const LOG_REPO_ROW: &str =
    "| 2026-10-02T00:00:00Z | `111111111111` | 🔴 no | 0/1 (0.0%) | 0/1 | 0/0 \
                            | 1 | 0 | 295.00 s | 1210.50 s | 781% | 7947 MB |";
const LOG_REGRESSION: &str = "  - 🔴 **flutter** — false negative — `src/one.rs:10-20` + \
                              `src/two.rs:30-40` — passed in the run of 2026-10-01T00:00:00Z";
const LOG_KNOWN_DEFECT: &str = "- ⚠️ known defect — **flutter** — false negative — \
                                `src/one.rs:10-20` + `src/two.rs:30-40`";
/// The headings `HISTORY.md` carries, newest run first.
const LOG_HEADINGS: [&str; 5] = [
    "# Corpus run history",
    "## Runs",
    "### flutter (dart)",
    "## Judged pairs the latest run gets wrong (1)",
    "## Regressions",
];
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
        version: None,
    }
}

/// Scoring that holds a run to the history and records it there.
const HELD: Scoring = Scoring {
    history: true,
    ..STRICT
};

/// One run of the fixture held to the history, whose scan reported the pair
/// when `reported`.
fn run(root: &Path, at: &str, reported: bool) -> Result<WrittenScorecard> {
    let pairs = if reported { the_pair() } else { Vec::new() };
    scored(root, at, &report(&pairs), HELD)
}

/// Every run the history under `root` holds.
fn runs(root: &Path) -> Result<Vec<RunRecord>> {
    store::load(root)
}

/// The history document under `root`.
fn log(root: &Path) -> Result<String> {
    Ok(fs::read_to_string(root.join(LOG_PATH))?)
}

/// Asserts `text` carries every line in `lines`.
fn holds(text: &str, lines: &[&str]) {
    for line in lines {
        assert!(text.contains(line), "missing `{line}`\n{text}");
    }
}

/// The last run the history holds.
fn last(root: &Path) -> Result<RunRecord> {
    runs(root)?
        .pop()
        .ok_or_else(|| anyhow!("the history holds no run"))
}

/// Asserts the scorecard carries every line in `lines`.
fn carries(written: &WrittenScorecard, lines: &[&str]) {
    holds(&written.markdown, lines);
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
        "{at}: the source commit is recorded"
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
    let (regressions, known) = (last(root)?.regressions, last(root)?.known_defects);
    assert_eq!(
        (regressions, known),
        (1, 0),
        "a pair both regressed and backslid counts once"
    );

    let history_log = log(root)?;
    holds(
        &history_log,
        &[LOG_FIRST_ROW, LOG_SECOND_ROW, LOG_REPO_ROW, LOG_REGRESSION],
    );
    holds(&history_log, &LOG_HEADINGS);
    let newest_first = history_log.find(LOG_SECOND_ROW) < history_log.find(LOG_FIRST_ROW);
    assert!(
        newest_first,
        "the newest run is listed first:\n{history_log}"
    );
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
        !root.join(RUNS_DIR).exists() && !root.join(LOG_PATH).exists(),
        "a run off the history records nothing"
    );
    assert!(
        !unrecorded.markdown.contains(SECTION),
        "and shows no history section"
    );

    let _first = run(root, FIRST_RUN, true)?;
    let _second = run(root, SECOND_RUN, false)?;
    for entry in fs::read_dir(root.join(RUNS_DIR))? {
        let path = entry?.path();
        let repinned = fs::read_to_string(&path)?.replace(&pinned_sha(), OTHER_SHA);
        fs::write(&path, repinned)?;
    }
    seed(root, &holding("fail", SECOND_RUN))?;

    let fourth = scored_as(root, FOURTH_RUN, &report(&[]), HELD, Some(RELEASE))?;
    assert_eq!(
        history::backslid(&fourth.card),
        0,
        "a pass at another commit is not this pair"
    );
    assert!(
        history::kept(&fourth.card),
        "with the pair on record as failing, the run keeps"
    );
    assert_eq!(
        fourth.markdown.lines().next(),
        Some(PASSED_HEADING),
        "a known defect that went nowhere is no regression, so the scorecard passes"
    );
    carries(&fourth, &[NONE_AGAINST_TWO, RELEASE_LINE]);
    assert_eq!(
        runs(root)?.len(),
        3,
        "every run held to the history is recorded in its own file"
    );
    let released = last(root)?;
    assert_eq!(
        released.source.version.as_deref(),
        Some(RELEASE),
        "a release run records its version beside its commit"
    );
    assert_eq!((released.regressions, released.known_defects), (0, 1));
    holds(&log(root)?, &[LOG_RELEASE_ROW, LOG_KNOWN_DEFECT]);
    Ok(())
}

#[test]
fn a_run_held_to_the_history_must_name_its_commit() -> Result<()> {
    let root = tempfile::tempdir()?;
    let root = root.path();
    let _first = run(root, FIRST_RUN, true)?;
    let mut manifest: serde_json::Value = crate::read_json(&root.join(RUN))?;
    let _commit = the_engine(&mut manifest)?.remove(COMMIT);
    write(root, RUN, &manifest)?;
    let refused = write_scorecard(&root.join(RUN), root, root, STEM, HELD);
    assert!(
        refused.is_err_and(|error| error.to_string().contains(NO_COMMIT)),
        "a run tied to no commit is never added to the history"
    );
    assert_eq!(
        runs(root)?.len(),
        1,
        "the history keeps only the run that named its commit"
    );
    Ok(())
}
