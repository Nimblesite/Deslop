//! [CORPUS-SCORE-RENDER] The judged-line coverage beside each pair's verdict.

use super::{row, score_cell, table_section, Scorecard, TargetScore, ABSENT};
use crate::corpus_score::{RangeCoverage, ScoredEntry, CLEARLY_IN};

/// Render coverage already calculated by the scorer.
pub(super) fn coverage_value(coverage: Option<&RangeCoverage>) -> String {
    coverage.map_or_else(
        || ABSENT.to_owned(),
        |coverage| {
            format!(
                "{}/{} ({})",
                coverage.covered_lines,
                coverage.judged_lines,
                score_cell(coverage.percent)
            )
        },
    )
}

/// Formatting only: all line counts and the percentage were scored upstream.
fn coverage_cell(target: &TargetScore, engine_id: &str, index: usize) -> String {
    let coverage = target
        .scores
        .get(engine_id)
        .and_then(|score| score.entries.get(index))
        .and_then(|entry| entry.coverage.as_ref());
    coverage_value(coverage)
}

/// One judged pair, with each engine's coverage in a separate column.
fn coverage_row(
    card: &Scorecard,
    target: &TargetScore,
    entry: &ScoredEntry,
    index: usize,
) -> String {
    let verdict = if entry.verdict == CLEARLY_IN {
        "IN"
    } else {
        "OUT"
    };
    let mut cells = vec![
        target.name.clone(),
        verdict.to_owned(),
        format!("`{}`", entry.occurrences.join("` + `")),
    ];
    let coverage = card
        .engines
        .iter()
        .map(|engine| coverage_cell(target, &engine.id, index));
    cells.extend(coverage);
    row(&cells)
}

/// Headings for one coverage column per engine.
fn coverage_columns(card: &Scorecard) -> Vec<String> {
    let mut columns = vec![
        "repository".to_owned(),
        "verdict".to_owned(),
        "judged pair".to_owned(),
    ];
    columns.extend(
        card.engines
            .iter()
            .map(|engine| format!("coverage `{}`", engine.id)),
    );
    columns
}

/// Each judged pair, in register order, with both engines side by side.
fn coverage_rows(card: &Scorecard) -> Vec<String> {
    card.targets
        .iter()
        .filter_map(|target| {
            card.engines
                .first()
                .and_then(|engine| target.scores.get(&engine.id))
                .map(|score| (target, score))
        })
        .flat_map(|(target, score)| {
            score
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| coverage_row(card, target, entry, index))
        })
        .collect()
}

/// Description of report extent; no coverage figure changes the score or gate.
pub(super) fn coverage_section(card: &Scorecard) -> Vec<String> {
    let title = "## Judged-range coverage";
    table_section(
        title,
        COVERAGE_INTRO,
        &coverage_columns(card),
        coverage_rows(card),
    )
}

/// What the coverage table means, stated above it.
const COVERAGE_INTRO: &str = "Each cell shows covered / judged lines across the pair's ranges. Overlapping occurrences count each line once within a range. A dash means no pair was reported. Coverage describes extent and never changes the verdict.";
