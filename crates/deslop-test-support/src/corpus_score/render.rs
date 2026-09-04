//! [CORPUS-SCORE-RENDER] The corpus scorecard, rendered as markdown.
//!
//! Every figure printed here was computed in [`super`] or [`super::gate`].
//! This module formats; it never derives a number, so the document and the
//! machine-readable `score.json` beside it can never disagree.

use std::collections::BTreeMap;

use serde::Serialize;

use super::{
    gate::{Breach, CorpusTotals, Degradation, Thresholds},
    RepoScore, RunCost,
};

/// Engines whose figures a change column can be drawn between.
const COMPARABLE_ENGINES: usize = 2;
/// Milliseconds in a second, so the wall-time column reads in seconds.
const MS_PER_SECOND: f64 = 1000.0;

/// One engine in a scored run.
#[derive(Debug, Clone, Serialize)]
pub struct Engine {
    /// Short identifier used as the key everywhere else in the document.
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
    /// The gate each repository was held to.
    pub thresholds: BTreeMap<String, Thresholds>,
    /// Every threshold the last engine breached. Empty means the run passes.
    pub breaches: Vec<Breach>,
}

/// One markdown table row.
fn row(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

/// The header divider for a table of `columns` columns.
fn divider(columns: usize) -> String {
    format!("|{}|", vec!["---"; columns].join("|"))
}

/// A percentage, or an explicit absence. Nothing judged must never print as a
/// perfect score.
fn score_cell(value: Option<f64>) -> String {
    value.map_or_else(
        || "not judged".to_owned(),
        |percent| format!("{percent:.1}%"),
    )
}

/// Milliseconds rendered as seconds.
fn seconds(ms: u64) -> String {
    format!("{:.2} s", as_f64(ms) / MS_PER_SECOND)
}

/// Mebibytes, or an explicit absence.
fn megabytes(value: Option<u64>) -> String {
    value.map_or_else(|| "—".to_owned(), |mb| format!("{mb} MB"))
}

/// CPU seconds, or an explicit absence.
fn cpu(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), |secs| format!("{secs:.2} s"))
}

/// Widening that keeps the lossy cast in one reviewed place.
fn as_f64(value: u64) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

/// The change in peak memory, absent when either side went unmeasured — an
/// unmeasured run must not read as "no change".
fn peak_change(before: Option<u64>, after: Option<u64>) -> String {
    match (before, after) {
        (Some(before), Some(after)) => {
            change(
                usize::try_from(before).unwrap_or_default(),
                usize::try_from(after).unwrap_or_default(),
            ) + " MB"
        }
        _ => "—".to_owned(),
    }
}

/// A signed change, so a comparison row reads without the reader subtracting.
fn change(before: usize, after: usize) -> String {
    let signed = |value: usize| i64::try_from(value).unwrap_or(i64::MAX);
    let delta = signed(after).saturating_sub(signed(before));
    if delta == 0 {
        "—".to_owned()
    } else {
        format!("{delta:+}")
    }
}

/// Header of the corpus standing table.
const TOTALS_COLUMNS: [&str; 8] = [
    "engine",
    "score",
    "judged",
    "false negatives",
    "false positives",
    "clusters",
    "wall",
    "peak RSS",
];

/// One totals row for one engine.
fn totals_row(engine: &Engine, totals: &CorpusTotals) -> String {
    row(&[
        format!("`{}`", engine.label),
        score_cell(totals.score_percent),
        format!("{}/{}", totals.correct, totals.judged),
        totals.false_negatives.to_string(),
        totals.false_positives.to_string(),
        totals.clusters_total.to_string(),
        format!(
            "{} ({} CPU)",
            seconds(totals.elapsed_ms),
            cpu(totals.cpu_seconds)
        ),
        megabytes(totals.peak_rss_mb),
    ])
}

/// The change row, drawn only when exactly two engines ran.
fn totals_change(card: &Scorecard) -> Vec<String> {
    let (Some(first), Some(last)) = (card.engines.first(), card.engines.last()) else {
        return Vec::new();
    };
    let (Some(before), Some(after)) = (card.totals.get(&first.id), card.totals.get(&last.id))
    else {
        return Vec::new();
    };
    vec![row(&[
        "**change**".to_owned(),
        format!(
            "{} → {}",
            score_cell(before.score_percent),
            score_cell(after.score_percent)
        ),
        format!("{} correct", change(before.correct, after.correct)),
        change(before.false_negatives, after.false_negatives),
        change(before.false_positives, after.false_positives),
        change(before.clusters_total, after.clusters_total),
        change(
            usize::try_from(before.elapsed_ms).unwrap_or_default(),
            usize::try_from(after.elapsed_ms).unwrap_or_default(),
        ) + " ms",
        peak_change(before.peak_rss_mb, after.peak_rss_mb),
    ])]
}

/// The corpus standing: one row per engine, plus the change between two.
fn totals_section(card: &Scorecard) -> Vec<String> {
    let mut lines = vec![
        "## Corpus standing".to_owned(),
        String::new(),
        "Score is `correct / judged` over every judged pair in the corpus. Clusters, wall \
         time and memory are description — they are reported beside the score and never \
         folded into it."
            .to_owned(),
        String::new(),
        row(&TOTALS_COLUMNS.map(ToOwned::to_owned)),
        divider(TOTALS_COLUMNS.len()),
    ];
    for engine in &card.engines {
        if let Some(totals) = card.totals.get(&engine.id) {
            lines.push(totals_row(engine, totals));
        }
    }
    if card.engines.len() == COMPARABLE_ENGINES {
        lines.extend(totals_change(card));
    }
    lines.push(String::new());
    lines
}

/// Header of the per-repository table.
const REPO_COLUMNS: [&str; 9] = [
    "repository",
    "engine",
    "score",
    "CLEARLY IN found",
    "CLEARLY OUT absent",
    "false neg",
    "false pos",
    "clusters",
    "wall / peak / CPU",
];

/// One row per engine, per repository.
fn repo_rows(card: &Scorecard, target: &TargetScore) -> Vec<String> {
    card.engines
        .iter()
        .filter_map(|engine| {
            let score = target.scores.get(&engine.id)?;
            let cost = target.costs.get(&engine.id);
            Some(row(&[
                format!("{} ({})", target.name, target.language),
                format!("`{}`", engine.label),
                score_cell(score.score_percent),
                format!("{}/{}", score.clearly_in_found, score.clearly_in_total),
                format!("{}/{}", score.clearly_out_absent, score.clearly_out_total),
                score.false_negatives.to_string(),
                score.false_positives.to_string(),
                score.clusters_total.to_string(),
                cost.map_or_else(
                    || "—".to_owned(),
                    |cost| {
                        format!(
                            "{} / {} / {}",
                            seconds(cost.elapsed_ms),
                            megabytes(cost.peak_rss_mb),
                            cpu(cost.cpu_seconds)
                        )
                    },
                ),
            ]))
        })
        .collect()
}

/// The per-repository table.
fn repos_section(card: &Scorecard) -> Vec<String> {
    let mut lines = vec![
        "## Per repository".to_owned(),
        String::new(),
        row(&REPO_COLUMNS.map(ToOwned::to_owned)),
        divider(REPO_COLUMNS.len()),
    ];
    for target in &card.targets {
        lines.extend(repo_rows(card, target));
    }
    lines.push(String::new());
    lines
}

/// The gate: what each repository must clear, and anything it did not.
fn gate_section(card: &Scorecard) -> Vec<String> {
    let mut lines = vec![
        "## Gate".to_owned(),
        String::new(),
        row(&[
            "repository",
            "minimum score",
            "max false neg",
            "max false pos",
        ]
        .map(ToOwned::to_owned)),
        divider(4),
    ];
    for (repo, thresholds) in &card.thresholds {
        lines.push(row(&[
            repo.clone(),
            format!("{:.1}%", thresholds.minimum_score_percent),
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

/// Renders the whole scorecard.
#[must_use]
pub fn scorecard(card: &Scorecard) -> String {
    let mut lines = vec![
        "# Corpus accuracy scorecard".to_owned(),
        String::new(),
        format!("Generated {}.", card.generated_at),
        String::new(),
        "Scored against the clone registers in `corpus/register/` — independent ground truth \
         judged in isolation from this codebase (`docs/specs/corpus.md` [CORPUS-REGISTER]). \
         A CLEARLY IN nobody reports is a **false negative**; a CLEARLY OUT that gets \
         reported is a **false positive**. Both are bugs."
            .to_owned(),
        String::new(),
    ];
    lines.extend(totals_section(card));
    lines.extend(repos_section(card));
    lines.extend(gate_section(card));
    lines.extend(defects_section(card));
    lines.join("\n")
}
