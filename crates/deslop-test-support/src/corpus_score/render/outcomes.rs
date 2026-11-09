//! [CORPUS-SCORE-OUTCOMES] The judged pair record, as the scorecard shows it:
//! whether every wrongly answered pair was already on record as failing, and
//! which pairs this run fixed, then every judged pair with its result. The
//! pairs answered wrongly are listed again, with why, under the defects that
//! follow.

use super::cells::{header, row};
use super::Scorecard;
use crate::corpus_score::outcomes::{Outcome, PairOutcome, Standing, OUTCOMES_PATH};

/// What the section says when no repository in the run has a clone register.
const NO_RECORD: &str = "**No judged pairs** — no repository in this run has a clone register, \
                         so nothing was held to the record.";
/// What it says when no pair on record as passing fails now.
const RECORD_PASS: &str = "**PASS** — every failed judged pair is explicitly accepted on record.";
/// What it says when one does.
const RECORD_FAIL: &str =
    "**FAIL** — an unaccepted judged pair fails or a passing pair regressed. \
                           Each one is listed at the top of this scorecard and below.";

/// The record section: what it holds, whether the run kept to it, and every
/// pair this run fixed.
pub(super) fn record_section(card: &Scorecard) -> Vec<String> {
    let mut lines = vec![
        "## Judged pair record".to_owned(),
        String::new(),
        format!(
            "`{OUTCOMES_PATH}` holds every judged pair's last recorded outcome. An unrecorded \
             failure or a regression fails the run. Only explicitly accepted failures are tolerated."
        ),
        String::new(),
        kept_line(&card.outcomes).to_owned(),
        String::new(),
    ];
    lines.extend(fixed_lines(&card.outcomes));
    lines.extend(every_pair_lines(&card.outcomes));
    lines
}

/// [CORPUS-SCORE-OUTCOMES] One row per judged pair this run scored: its
/// result and how that stands against the record, so every result is read
/// on its own and no gain can be read as offsetting a loss.
fn every_pair_lines(outcomes: &[PairOutcome]) -> Vec<String> {
    if outcomes.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        "### Every judged pair".to_owned(),
        String::new(),
        "Each pair stands alone: one regression or unrecorded failure fails the run however \
         many other pairs were fixed."
            .to_owned(),
        String::new(),
    ];
    lines.extend(header(&PAIR_COLUMNS.map(String::from)));
    lines.extend(outcomes.iter().map(|pair| {
        row(&[
            pair.key.repo.clone(),
            pair.key.verdict.clone(),
            format!("`{}`", pair.key.occurrences.join("` + `")),
            result_cell(pair.outcome).to_owned(),
            standing_cell(pair.standing).to_owned(),
        ])
    }));
    lines.push(String::new());
    lines
}

/// The columns of the every-judged-pair table.
const PAIR_COLUMNS: [&str; 5] = ["repository", "verdict", "judged pair", "result", "standing"];

/// A pair's result in this run.
const fn result_cell(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Pass => "✅ pass",
        Outcome::Fail => "🔴 fail",
    }
}

/// How a pair's result stands against the record, and whether it fails the run.
const fn standing_cell(standing: Standing) -> &'static str {
    match standing {
        Standing::Passing => "passing",
        Standing::Fixed => "fixed — commit the record",
        Standing::StillFailing => "on record as failing",
        Standing::Unrecorded => "🔴 not on record — fails the run",
        Standing::Regressed => "🔴 regressed — fails the run",
    }
}

/// Whether the run kept to the record, in words.
fn kept_line(outcomes: &[PairOutcome]) -> &'static str {
    if outcomes.is_empty() {
        NO_RECORD
    } else if outcomes.iter().any(|pair| pair.standing.fails_the_run()) {
        RECORD_FAIL
    } else {
        RECORD_PASS
    }
}

/// One line per pair this run fixed: on record as failing, now answered right.
fn fixed_lines(outcomes: &[PairOutcome]) -> Vec<String> {
    let mut lines: Vec<String> = outcomes
        .iter()
        .filter(|pair| pair.standing == Standing::Fixed)
        .map(|pair| {
            format!(
                "- ✅ **FIXED** {} — `{}` — {}",
                pair.key.repo,
                pair.key.occurrences.join("` + `"),
                pair.note()
            )
        })
        .collect();
    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
