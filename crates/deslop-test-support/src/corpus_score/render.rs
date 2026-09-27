//! [CORPUS-SCORE-RENDER] The corpus scorecard, rendered as markdown.
//!
//! Every figure printed here was computed in [`super`] or [`super::gate`].
//! This module formats; it never derives a number, so the document and the
//! machine-readable JSON scorecard beside it can never disagree.
//!
//! Every table reads **side by side**. The corpus standing is one measure per
//! row with a column per engine; each per-repository table is one repository
//! per row with a column per engine. Two figures a reader is meant to compare
//! are never a row apart, because a comparison split across rows is one the
//! reader has to reassemble by eye.

mod cells;
mod checks;
mod cost;
mod coverage;
mod verdict;

use std::collections::{BTreeMap, BTreeSet};

use cells::{counted, header, row, score_cell, signed, signed_fraction, ABSENT};
use checks::checks_section;
use cost::{cost_section, standing_cost};
use coverage::{coverage_section, coverage_value};
use verdict::verdict_banner;
use serde::Serialize;

use super::{
    checks::CheckOutcome,
    gate::{Breach, CorpusChange, CorpusTotals, Degradation, Thresholds},
    verdict::Verdict,
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
    /// Whether a clone register judged this repository at the scanned commit.
    /// An unjudged repository is still reported: its cost and its curated
    /// checks are evidence, and leaving it out would let a partial run read as
    /// a smaller, cleaner one.
    pub registered: bool,
    /// Register score per engine id; empty when there is no register.
    pub scores: BTreeMap<String, RepoScore>,
    /// Clusters each engine's report published, per engine id.
    pub clusters: BTreeMap<String, usize>,
    /// Measured cost per engine id.
    pub costs: BTreeMap<String, RunCost>,
    /// Curated corpus checks per engine id, present when a corpus test ran them.
    pub checks: BTreeMap<String, CheckOutcome>,
    /// Whether the last engine lost ground against the first. Absent unless
    /// exactly two engines ran.
    pub degradation: Option<Degradation>,
}

impl TargetScore {
    /// Whether `engine_id` scanned this repository at all — including a scan
    /// that crashed, which has a cost and a check outcome but no report.
    #[must_use]
    pub fn ran(&self, engine_id: &str) -> bool {
        self.clusters.contains_key(engine_id)
            || self.costs.contains_key(engine_id)
            || self.checks.contains_key(engine_id)
    }
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
    /// Every threshold the last engine breached. Empty means the register
    /// gate passes.
    pub breaches: Vec<Breach>,
    /// Whether the run passed, and every reason it did not.
    pub verdict: Verdict,
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
        standing_row(
            card,
            "curated checks passed",
            &|totals| format!("{}/{}", totals.checks_passed, totals.checks_evaluated),
            moved.map(|_| ABSENT.to_owned()),
        ),
        standing_row(
            card,
            "new curated check failures",
            &|totals| totals.checks_new_failures.to_string(),
            moved.map(|_| ABSENT.to_owned()),
        ),
    ]
}

/// Found positive pairs' line extent is visible but never changes the score.
fn standing_coverage(card: &Scorecard) -> String {
    standing_row(
        card,
        "matched IN coverage",
        &|totals| coverage_value(totals.matched_in_coverage.as_ref()),
        card.change.as_ref().map(|_| ABSENT.to_owned()),
    )
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
        "Score is `correct / judged` over every judged pair in the corpus. Curated checks \
         are the `corpus/<name>.json` assertions a corpus test evaluated. Each engine has \
         its own column, so every measure reads across one row. Matched IN coverage is \
         covered / judged lines among reported CLEARLY IN pairs; missed and CLEARLY OUT \
         pairs are excluded. Coverage, clusters, wall time, CPU and memory are description \
         — reported beside the score and never folded into it or its gate. Cost totals \
         cover every repository scanned: wall and CPU time are summed, and each peak is \
         the largest single scan's."
            .to_owned(),
        String::new(),
    ];
    lines.extend(header(&columns));
    lines.extend(standing_accuracy(card));
    lines.push(standing_coverage(card));
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
    if !target.registered {
        return "no register".to_owned();
    }
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

/// The gate: what each repository must clear, and anything it did not.
fn gate_section(card: &Scorecard) -> Vec<String> {
    let mut lines = vec!["## Register gate".to_owned(), String::new()];
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

/// What the gate and the defect list say when no repository in the run has a
/// clone register: nothing was judged, which is not the same as nothing wrong.
const NO_REGISTER_GATE: &str = "**No register gate** — no repository in this run has a clone \
                                register, so nothing was scored against one.";
/// See [`NO_REGISTER_GATE`].
const NO_JUDGED_PAIRS: &str = "No judged pairs — no repository in this run has a clone register.";

/// The verdict, stated in words rather than left to the reader.
fn breach_lines(card: &Scorecard) -> Vec<String> {
    if card.thresholds.is_empty() {
        return vec![NO_REGISTER_GATE.to_owned(), String::new()];
    }
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
    let judged = card
        .targets
        .iter()
        .any(|target| target.scores.contains_key(&engine.id));
    let defects = defect_lines(card, &engine.id);
    let verdict = match (judged, defects.is_empty()) {
        (false, _) => vec![NO_JUDGED_PAIRS.to_owned()],
        (true, true) => vec!["None. Every judged pair is answered correctly.".to_owned()],
        (true, false) => defects,
    };
    let mut lines = vec![
        format!("## Judged pairs `{}` gets wrong", engine.label),
        String::new(),
    ];
    lines.extend(verdict);
    lines.push(String::new());
    lines
}

/// One line per judged pair `engine_id` answered wrongly.
fn defect_lines(card: &Scorecard, engine_id: &str) -> Vec<String> {
    card.targets
        .iter()
        .filter_map(|target| Some((target, target.scores.get(engine_id)?)))
        .flat_map(|(target, score)| {
            score
                .entries
                .iter()
                .filter(|entry| !entry.correct)
                .map(move |entry| {
                    let kind = if entry.is_false_negative() {
                        "FALSE NEGATIVE"
                    } else {
                        "FALSE POSITIVE"
                    };
                    format!(
                        "- **{kind}** {} — `{}`\n  - {}",
                        target.name,
                        entry.occurrences.join("` + `"),
                        entry.why
                    )
                })
        })
        .collect()
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
    let mut lines = verdict_banner(card);
    lines.extend([
        format!("Generated {}.", card.generated_at),
        String::new(),
        scope_line(card),
        String::new(),
        "Scored against the clone registers in `corpus/register/` — independent ground truth \
         judged in isolation from this codebase (`docs/specs/corpus.md` [CORPUS-REGISTER]). \
         A CLEARLY IN nobody reports is a **false negative**; a CLEARLY OUT that gets \
         reported is a **false positive**. Both are bugs. A repository no register judges \
         is still listed, with its curated checks and its cost."
            .to_owned(),
        String::new(),
    ]);
    lines.extend(totals_section(card));
    lines.extend(accuracy_section(card));
    lines.extend(checks_section(card));
    lines.extend(coverage_section(card));
    lines.extend(cost_section(card));
    lines.extend(gate_section(card));
    lines.extend(defects_section(card));
    lines.join("\n")
}
