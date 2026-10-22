//! [CORPUS-SCORE-OUTCOMES] The judged pair record, as the scorecard shows it:
//! whether every wrongly answered pair was already on record as failing, and
//! which pairs this run fixed. The pairs answered wrongly are listed, with how
//! each stands, under the defects that follow.

use super::Scorecard;
use crate::corpus_score::outcomes::{PairOutcome, Standing, OUTCOMES_PATH};

/// What the section says when no repository in the run has a clone register.
const NO_RECORD: &str = "**No judged pairs** — no repository in this run has a clone register, \
                         so nothing was held to the record.";
/// What it says when no pair on record as passing fails now.
const RECORD_PASS: &str = "**PASS** — no judged pair that passed before fails now.";
/// What it says when one does.
const RECORD_FAIL: &str = "**FAIL** — a judged pair that passed before fails now. Each one is \
                           listed at the top of this scorecard and below. A regression is a bug.";

/// The record section: what it holds, whether the run kept to it, and every
/// pair this run fixed.
pub(super) fn record_section(card: &Scorecard) -> Vec<String> {
    let mut lines = vec![
        "## Judged pair record".to_owned(),
        String::new(),
        format!(
            "`{OUTCOMES_PATH}` holds every judged pair's last recorded outcome. Only a pair on \
             record as passing that fails now — a regression — fails the run. A pair that has \
             never passed is a known defect, listed below."
        ),
        String::new(),
        kept_line(&card.outcomes).to_owned(),
        String::new(),
    ];
    lines.extend(fixed_lines(&card.outcomes));
    lines
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
