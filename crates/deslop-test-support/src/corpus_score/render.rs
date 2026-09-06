//! [CORPUS-SCORE-RENDER] The corpus scorecard, rendered as markdown.
//!
//! Every figure printed here was computed in [`super`] or [`super::gate`].
//! This module formats; it never derives a number, so the document and the
//! machine-readable `score.json` beside it can never disagree.
//!
//! Every table reads **side by side**. The corpus standing is one measure per
//! row with a column per engine; each per-repository table is one repository
//! per row with a column per engine. Two figures a reader is meant to compare
//! are never a row apart, because a comparison split across rows is one the
//! reader has to reassemble by eye.

mod cells;

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use cells::{
    counted, cpu, header, megabytes, row, score_cell, seconds, signed, signed_amount,
    signed_fraction, ABSENT,
};

use super::{
    gate::{Breach, CorpusChange, CorpusTotals, Degradation, Thresholds},
    RepoScore, RunCost,
};

/// One engine in a scored run.
#[derive(Debug, Clone, Serialize)]
pub struct Engine {
    /// Short identifier used as the key everywhere else in the document, and as
    /// the per-engine column heading.
    pub id: String,
    /// Human label, e.g. `deslop@e8a215e99fb9`.
    pub label: String,
}

/// One target repository, scored by every engine in the run.
#[derive(Debug, Clone, Serialize)]
pub struct TargetScore {
    /// Repository name, matching the register file stem.
    pub name: String,
    /// Language label, for the table only.
    pub language: String,
    /// The commit scanned and judged.
    pub sha: String,
    /// Score per engine id.
    pub scores: BTreeMap<String, RepoScore>,
    /// Measured cost per engine id.
    pub costs: BTreeMap<String, RunCost>,
    /// Whether the last engine lost ground against the first. Absent unless
    /// exactly two engines ran.
    pub degradation: Option<Degradation>,
}

/// A whole scored run: every engine, every target, the totals and the gate.
#[derive(Debug, Clone, Serialize)]
pub struct Scorecard {
    /// When the run was scored.
    pub generated_at: String,
    /// The engines compared, in run order.
    pub engines: Vec<Engine>,
    /// The targets scored, in run order.
    pub targets: Vec<TargetScore>,
    /// Corpus standing per engine id.
    pub totals: BTreeMap<String, CorpusTotals>,
    /// How the last engine's standing moved against the first. Absent unless
    /// two engines ran.
    pub change: Option<CorpusChange>,
    /// The gate each repository was held to.
    pub thresholds: BTreeMap<String, Thresholds>,
    /// Every threshold the last engine breached. Empty means the run passes.
    pub breaches: Vec<Breach>,
}

/// One cell per engine, in run order, out of a map keyed by engine id. Every
/// side-by-side column in the document is built here, so no two tables can
/// order their engines differently or spell an absent run differently.
fn engine_cells<T>(
    card: &Scorecard,
    source: &BTreeMap<String, T>,
    cell: &dyn Fn(&T) -> String,
) -> Vec<String> {
    card.engines
        .iter()
        .map(|engine| {
            source
                .get(&engine.id)
                .map_or_else(|| ABSENT.to_owned(), cell)
        })
        .collect()
}

/// One corpus-standing row: the measure, one cell per engine, then the change
/// when two engines ran.
fn standing_row(
    card: &Scorecard,
    measure: &str,
    cell: &dyn Fn(&CorpusTotals) -> String,
    delta: Option<String>,
) -> String {
    let mut cells = vec![measure.to_owned()];
    cells.extend(engine_cells(card, &card.totals, cell));
    cells.extend(delta);
    row(&cells)
}

/// The accuracy rows of the corpus standing — the measures that are scored.
fn standing_accuracy(card: &Scorecard) -> Vec<String> {
    let moved = card.change.as_ref();
    vec![
        standing_row(
            card,
            "score",
            &|totals| score_cell(totals.score_percent),
            moved.map(|moved| signed_fraction(moved.score_points, "pts")),
        ),
        standing_row(
            card,
            "correct / judged",
            &|totals| format!("{}/{}", totals.correct, totals.judged),
            moved.map(|moved| format!("{} correct", signed(moved.correct))),
        ),
        standing_row(
            card,
            "false negatives",
            &|totals| totals.false_negatives.to_string(),
            moved.map(|moved| signed(moved.false_negatives)),
        ),
        standing_row(
            card,
            "false positives",
            &|totals| totals.false_positives.to_string(),
            moved.map(|moved| signed(moved.false_positives)),
        ),
    ]
}

/// The cost rows of the corpus standing — description, never scored.
fn standing_cost(card: &Scorecard) -> Vec<String> {
    let moved = card.change.as_ref();
    vec![
        standing_row(
            card,
            "clusters",
            &|totals| totals.clusters_total.to_string(),
            moved.map(|moved| signed(moved.clusters_total)),
        ),
        standing_row(
            card,
            "wall",
            &|totals| seconds(totals.elapsed_ms),
            moved.map(|moved| signed_amount(Some(moved.elapsed_ms), "ms")),
        ),
        standing_row(
            card,
            "CPU",
            &|totals| cpu(totals.cpu_seconds),
            moved.map(|moved| signed_fraction(moved.cpu_seconds, "s")),
        ),
        standing_row(
            card,
            "peak RSS",
            &|totals| megabytes(totals.peak_rss_mb),
            moved.map(|moved| signed_amount(moved.peak_rss_mb, "MB")),
        ),
    ]
}

/// The corpus standing: one measure per row, one column per engine.
fn totals_section(card: &Scorecard) -> Vec<String> {
    let mut columns = vec!["measure".to_owned()];
    columns.extend(
        card.engines
            .iter()
            .map(|engine| format!("`{}`", engine.label)),
    );
    if card.change.is_some() {
        columns.push("change".to_owned());
    }
    let mut lines = vec![
        "## Corpus standing".to_owned(),
        String::new(),
        "Score is `correct / judged` over every judged pair in the corpus. Each engine \
         has its own column, so every measure reads across one row. Clusters, wall time \
         and memory are description — they are reported beside the score and never \
         folded into it."
            .to_owned(),
        String::new(),
    ];
    lines.extend(header(&columns));
    lines.extend(standing_accuracy(card));
    lines.extend(standing_cost(card));
    lines.push(String::new());
    lines
}

/// One header cell per engine for a measure: the measure, then the engine id.
fn measure_headers(card: &Scorecard, measure: &str) -> Vec<String> {
    card.engines
        .iter()
        .map(|engine| format!("{measure} `{}`", engine.id))
        .collect()
}

/// What the register judged for this repository. The register is the same
/// document for every engine, so it is stated once rather than per column.
fn judged_cell(target: &TargetScore) -> String {
    target.scores.values().next().map_or_else(
        || ABSENT.to_owned(),
        |score| {
            format!(
                "{} IN + {} OUT",
                score.clearly_in_total, score.clearly_out_total
            )
        },
    )
}

/// Whether the defects this repository carries are new or standing — the only
/// thing that separates a regression from a bug that was already there.
fn degradation_cell(target: &TargetScore) -> String {
    let Some(moved) = target.degradation.as_ref() else {
        return ABSENT.to_owned();
    };
    let mut parts = Vec::new();
    let mut note = |count: usize, label: &str| {
        if count > 0 {
            parts.push(format!("{count} {label}"));
        }
    };
    note(moved.new_false_negatives.len(), "new FN");
    note(moved.new_false_positives.len(), "new FP");
    note(moved.standing_false_negatives, "standing FN");
    note(moved.standing_false_positives, "standing FP");
    if parts.is_empty() {
        "clean".to_owned()
    } else {
        parts.join(", ")
    }
}

/// The repository's name and language, the first cell of every per-repo row.
fn repo_cell(target: &TargetScore) -> String {
    format!("{} ({})", target.name, target.language)
}

/// One accuracy row: the repository, what was judged, then every engine's score
/// and defect counts side by side.
fn accuracy_row(card: &Scorecard, target: &TargetScore) -> String {
    let mut cells = vec![repo_cell(target), judged_cell(target)];
    cells.extend(engine_cells(card, &target.scores, &|score| {
        score_cell(score.score_percent)
    }));
    cells.extend(engine_cells(card, &target.scores, &|score| {
        format!("{}/{}", score.clearly_in_found, score.clearly_in_total)
    }));
    cells.extend(engine_cells(card, &target.scores, &|score| {
        format!("{}/{}", score.clearly_out_absent, score.clearly_out_total)
    }));
    cells.push(degradation_cell(target));
    row(&cells)
}

/// The per-repository accuracy table.
fn accuracy_section(card: &Scorecard) -> Vec<String> {
    let mut columns = vec!["repository".to_owned(), "judged".to_owned()];
    for measure in ["score", "IN found", "OUT absent"] {
        columns.extend(measure_headers(card, measure));
    }
    columns.push("defects".to_owned());
    let mut lines = vec![
        "## Per repository — accuracy".to_owned(),
        String::new(),
        "One row per repository, one column per engine, so the two runs sit beside each \
         other. `IN found` is the CLEARLY IN pairs the engine reported; `OUT absent` the \
         CLEARLY OUT pairs it correctly stayed silent on. The last column says whether a \
         defect is **new** against the first engine or **standing** in both."
            .to_owned(),
        String::new(),
    ];
    lines.extend(header(&columns));
    lines.extend(card.targets.iter().map(|target| accuracy_row(card, target)));
    lines.push(String::new());
    lines
}

/// One cost row: the repository, then every engine's cost side by side.
fn cost_row(card: &Scorecard, target: &TargetScore) -> String {
    let mut cells = vec![repo_cell(target)];
    cells.extend(engine_cells(card, &target.scores, &|score| {
        score.clusters_total.to_string()
    }));
    cells.extend(engine_cells(card, &target.costs, &|cost| {
        seconds(cost.elapsed_ms)
    }));
    cells.extend(engine_cells(card, &target.costs, &|cost| {
        megabytes(cost.peak_rss_mb)
    }));
    cells.extend(engine_cells(card, &target.costs, &|cost| {
        cpu(cost.cpu_seconds)
    }));
    row(&cells)
}

/// The per-repository cost table.
fn cost_section(card: &Scorecard) -> Vec<String> {
    let mut columns = vec!["repository".to_owned()];
    for measure in ["clusters", "wall", "peak", "CPU"] {
        columns.extend(measure_headers(card, measure));
    }
    let mut lines = vec![
        "## Per repository — cost".to_owned(),
        String::new(),
        "Description, never scored. Reported beside the accuracy table so a change in \
         cost can never be mistaken for a change in what the engine found."
            .to_owned(),
        String::new(),
    ];
    lines.extend(header(&columns));
    lines.extend(card.targets.iter().map(|target| cost_row(card, target)));
    lines.push(String::new());
    lines
}

/// The gate: what each repository must clear, and anything it did not.
fn gate_section(card: &Scorecard) -> Vec<String> {
    let mut lines = vec!["## Gate".to_owned(), String::new()];
    lines.extend(header(
        &["repository", "max false neg", "max false pos"].map(ToOwned::to_owned),
    ));
    for (repo, thresholds) in &card.thresholds {
        lines.push(row(&[
            repo.clone(),
            thresholds.maximum_false_negatives.to_string(),
            thresholds.maximum_false_positives.to_string(),
        ]));
    }
    lines.push(String::new());
    lines.extend(breach_lines(card));
    lines
}

/// The verdict, stated in words rather than left to the reader.
fn breach_lines(card: &Scorecard) -> Vec<String> {
    if card.breaches.is_empty() {
        return vec![
            "**PASS** — every scored repository is inside its gate.".to_owned(),
            String::new(),
        ];
    }
    let mut lines = vec![format!(
        "**FAIL** — {} breach(es). A new false positive or false negative is a bug.",
        card.breaches.len()
    )];
    lines.push(String::new());
    for breach in &card.breaches {
        lines.push(format!(
            "- `{}` {}: allows {}, recorded {}",
            breach.repo, breach.measure, breach.allowed, breach.actual
        ));
    }
    lines.push(String::new());
    lines
}

/// Every judged pair the last engine got wrong, so a breach names the code.
fn defects_section(card: &Scorecard) -> Vec<String> {
    let Some(engine) = card.engines.last() else {
        return Vec::new();
    };
    let mut lines = vec![
        format!("## Judged pairs `{}` gets wrong", engine.label),
        String::new(),
    ];
    let mut any = false;
    for target in &card.targets {
        let Some(score) = target.scores.get(&engine.id) else {
            continue;
        };
        for entry in score.entries.iter().filter(|entry| !entry.correct) {
            any = true;
            let kind = if entry.is_false_negative() {
                "FALSE NEGATIVE"
            } else {
                "FALSE POSITIVE"
            };
            lines.push(format!(
                "- **{kind}** {} — `{}`\n  - {}",
                target.name,
                entry.occurrences.join("` + `"),
                entry.why
            ));
        }
    }
    if !any {
        lines.push("None. Every judged pair is answered correctly.".to_owned());
    }
    lines.push(String::new());
    lines
}

/// The scope of the run, stated before any figure: how many repositories were
/// scanned, how many distinct languages they cover, and which. A green run over
/// three repositories must never read like a green run over the whole corpus,
/// for the same reason [CORPUS-CI] makes a scheduled run name what it skipped.
fn scope_line(card: &Scorecard) -> String {
    let languages: BTreeSet<&str> = card
        .targets
        .iter()
        .map(|target| target.language.as_str())
        .collect();
    let named = languages.iter().copied().collect::<Vec<_>>().join(", ");
    format!(
        "Scope: {} across {} — {}.",
        counted(card.targets.len(), "repository", "repositories"),
        counted(languages.len(), "language", "languages"),
        if named.is_empty() { ABSENT } else { &named }
    )
}

/// Renders the whole scorecard.
#[must_use]
pub fn scorecard(card: &Scorecard) -> String {
    let mut lines = vec![
        "# Corpus accuracy scorecard".to_owned(),
        String::new(),
        format!("Generated {}.", card.generated_at),
        String::new(),
        scope_line(card),
        String::new(),
        "Scored against the clone registers in `corpus/register/` — independent ground truth \
         judged in isolation from this codebase (`docs/specs/corpus.md` [CORPUS-REGISTER]). \
         A CLEARLY IN nobody reports is a **false negative**; a CLEARLY OUT that gets \
         reported is a **false positive**. Both are bugs."
            .to_owned(),
        String::new(),
    ];
    lines.extend(totals_section(card));
    lines.extend(accuracy_section(card));
    lines.extend(cost_section(card));
    lines.extend(gate_section(card));
    lines.extend(defects_section(card));
    lines.join("\n")
}
