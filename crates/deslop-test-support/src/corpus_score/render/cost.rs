//! [CORPUS-SCORE-RENDER] What each scan cost: wall time, CPU time, peak CPU
//! and peak memory, per engine, beside — never inside — the accuracy figures.

use super::{
    cells::{cpu, cpu_rate, megabytes, or_absent, seconds, signed, signed_amount, signed_fraction},
    engine_cells, measure_headers, repo_cell, row, standing_row, table_section, Scorecard,
    TargetScore,
};
use crate::{
    corpus_measure::CPU_SAMPLE_WINDOW,
    corpus_score::gate::{CorpusChange, CorpusTotals},
};

/// One corpus-standing cost row: the measure, how each engine's total reads,
/// and how the change between two engines reads.
type CostRow = (
    &'static str,
    fn(&CorpusTotals) -> String,
    fn(&CorpusChange) -> String,
);

/// The cost measures, in the order every cost table prints them.
const COST_ROWS: [CostRow; 5] = [
    (
        "clusters",
        |totals| or_absent(totals.clusters_total, |count| count.to_string()),
        |moved| or_absent(moved.clusters_total, signed),
    ),
    (
        "wall",
        |totals| or_absent(totals.elapsed_ms, seconds),
        |moved| signed_amount(moved.elapsed_ms, "ms"),
    ),
    (
        "CPU time",
        |totals| cpu(totals.cpu_seconds),
        |moved| signed_fraction(moved.cpu_seconds, "s"),
    ),
    (
        "peak CPU",
        |totals| cpu_rate(totals.peak_cpu_percent),
        |moved| signed_fraction(moved.peak_cpu_percent, "pts"),
    ),
    (
        "peak memory",
        |totals| megabytes(totals.peak_rss_mb),
        |moved| signed_amount(moved.peak_rss_mb, "MB"),
    ),
];

/// The cost rows of the corpus standing — description, never scored.
pub(super) fn standing_cost(card: &Scorecard) -> Vec<String> {
    COST_ROWS
        .iter()
        .map(|(measure, cell, delta)| {
            standing_row(card, measure, cell, card.change.as_ref().map(delta))
        })
        .collect()
}

/// One cost row: the repository, then every engine's cost side by side.
fn cost_row(card: &Scorecard, target: &TargetScore) -> String {
    let mut cells = vec![repo_cell(target)];
    cells.extend(engine_cells(card, &target.clusters, &ToString::to_string));
    cells.extend(engine_cells(card, &target.costs, &|cost| {
        seconds(cost.elapsed_ms)
    }));
    cells.extend(engine_cells(card, &target.costs, &|cost| {
        cpu(cost.cpu_seconds)
    }));
    cells.extend(engine_cells(card, &target.costs, &|cost| {
        cpu_rate(cost.peak_cpu_percent)
    }));
    cells.extend(engine_cells(card, &target.costs, &|cost| {
        megabytes(cost.peak_rss_mb)
    }));
    row(&cells)
}

/// The per-repository cost table.
pub(super) fn cost_section(card: &Scorecard) -> Vec<String> {
    let mut columns = vec!["repository".to_owned()];
    for (measure, _, _) in COST_ROWS {
        columns.extend(measure_headers(card, measure));
    }
    let rows = card.targets.iter().map(|target| cost_row(card, target));
    table_section("## Per repository — cost", &cost_intro(), &columns, rows)
}

/// What each cost column means, stated above the table.
fn cost_intro() -> String {
    format!(
        "Description, never scored. Reported beside the accuracy tables so a change in \
         cost can never be mistaken for a change in what the engine found. `wall` is the \
         scan's elapsed time; `CPU time` is user + system time across every core; `peak \
         CPU` is the busiest {} ms of the scan, in percent of one core (`400%` is four \
         cores busy); `peak memory` is the peak resident set.",
        CPU_SAMPLE_WINDOW.as_millis()
    )
}
