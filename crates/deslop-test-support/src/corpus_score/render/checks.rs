//! [CORPUS-REPORT-CHECKS] The curated corpus checks, per repository and engine.
//!
//! Rendered in every scorecard, including runs that evaluated none: an absent
//! cell is how a register-only run and a curated-check run keep reading as the
//! same document, and how a reader tells "nothing failed" from "nothing was
//! checked".

use super::{
    engine_cells, header, measure_headers, repo_cell, row, Scorecard, TargetScore, ABSENT,
};
use crate::corpus_score::checks::{CheckFailure, CheckOutcome};

/// One engine's outcome: passed of evaluated, flagged when the manifest
/// curates no accuracy claim and the run checked scope and ceilings only.
fn outcome_cell(outcome: &CheckOutcome) -> String {
    let caveat = if outcome.accuracy_curated {
        ""
    } else {
        " (ceilings only)"
    };
    format!(
        "{}/{} pass{caveat}",
        outcome.passed(),
        outcome.evaluated.len()
    )
}

/// A failure's check id, marked new or already tracked.
fn failure_label(failure: &CheckFailure) -> String {
    let status = if failure.known { "known" } else { "NEW" };
    format!("`{}` ({status})", failure.check)
}

/// The failing checks of the last engine that evaluated any.
fn failing_cell(card: &Scorecard, target: &TargetScore) -> String {
    let outcome = card
        .engines
        .iter()
        .rev()
        .find_map(|engine| target.checks.get(&engine.id));
    match outcome {
        None => ABSENT.to_owned(),
        Some(outcome) if outcome.failures.is_empty() => "none".to_owned(),
        Some(outcome) => outcome
            .failures
            .iter()
            .map(failure_label)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// One row: the repository, each engine's outcome, then what is failing.
fn checks_row(card: &Scorecard, target: &TargetScore) -> String {
    let mut cells = vec![repo_cell(target)];
    cells.extend(engine_cells(card, &target.checks, &outcome_cell));
    cells.push(failing_cell(card, target));
    row(&cells)
}

/// Every failure's detail, so a failing check names what it saw.
fn failure_details(card: &Scorecard) -> Vec<String> {
    card.targets
        .iter()
        .flat_map(|target| {
            card.engines
                .iter()
                .filter_map(|engine| Some((engine, target.checks.get(&engine.id)?)))
                .flat_map(|(engine, outcome)| failure_lines(target, &engine.id, outcome))
        })
        .collect()
}

/// One line per failure one engine recorded for one repository.
fn failure_lines(target: &TargetScore, engine_id: &str, outcome: &CheckOutcome) -> Vec<String> {
    outcome
        .failures
        .iter()
        .map(|failure| {
            let label = failure_label(failure);
            format!(
                "- {} `{engine_id}` {label}: {}",
                target.name, failure.detail
            )
        })
        .collect()
}

/// The per-repository curated-checks table and every failure it records.
pub(super) fn checks_section(card: &Scorecard) -> Vec<String> {
    let mut columns = vec!["repository".to_owned()];
    columns.extend(measure_headers(card, "checks"));
    columns.push("failing".to_owned());
    let mut lines = vec![
        "## Per repository — curated checks".to_owned(),
        String::new(),
        CHECKS_INTRO.to_owned(),
        String::new(),
    ];
    lines.extend(header(&columns));
    lines.extend(card.targets.iter().map(|target| checks_row(card, target)));
    lines.push(String::new());
    lines.extend(failure_details(card));
    lines.push(String::new());
    lines
}

/// What the curated-checks table means, stated above it.
const CHECKS_INTRO: &str =
    "The hand-curated assertions in `corpus/<name>.json` that `make test-corpus` holds \
         each repository to: duplicates that must be found, ranking rules, the scan's scope \
         and its resource ceilings. `known` failures are tracked in \
         `corpus/known-failures.json`; a `NEW` one is a regression. `ceilings only` means \
         the manifest curates no accuracy claim, so a pass there says nothing about whether \
         detection is correct. `—` means the run evaluated none.";
