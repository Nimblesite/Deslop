//! [CORPUS-HISTORY] `corpus/history/HISTORY.md`: every recorded run as a
//! person reads it — one row per run, then one table per repository with a
//! row per run, newest first, then the pairs the latest run gets wrong and
//! every regression any run found. Rendered from the run records alone; every
//! figure was computed when its run was scored, and nothing here derives one.

use super::cells::{cpu, cpu_rate, header, megabytes, or_absent, row, score_cell, seconds};
use crate::corpus_score::{
    history::{Backslide, Host, Judgment, RepoRecord, RunRecord},
    outcomes::Outcome,
    source::Source,
};

/// What the document says before any run is recorded.
const NO_RUNS: &str = "No run is recorded yet. `make score-corpus` records one.";
/// A run, or a repository, that kept to its judgments, and one that did not.
const PASSED: &str = "✅ passed";
/// See [`PASSED`].
const FAILED: &str = "🔴 failed";
/// The runs table's columns.
const RUN_COLUMNS: [&str; 11] = [
    "run (UTC)",
    "source",
    "result",
    "correct / judged",
    "score",
    "regressions",
    "known defects",
    "wall",
    "CPU time",
    "peak memory",
    "machine",
];
/// Each repository table's columns.
const REPO_COLUMNS: [&str; 12] = [
    "run (UTC)",
    "source",
    "matches judgments",
    "correct / judged",
    "IN found",
    "OUT absent",
    "false negatives",
    "false positives",
    "wall",
    "CPU time",
    "peak CPU",
    "peak memory",
];

/// The whole history document for `runs`, given oldest first.
#[must_use]
pub fn history_log(runs: &[RunRecord]) -> String {
    let newest: Vec<&RunRecord> = runs.iter().rev().collect();
    let mut lines = vec![
        "# Corpus run history".to_owned(),
        String::new(),
        "Every `make score-corpus` run over the whole judged corpus, newest first ([CORPUS-HISTORY]). \
         Each run is tied to the commit it was built from, and to its release version when the \
         commit is a tagged release. This page is rendered from the records in \
         `corpus/history/runs/` each time a run is added — never edit it by hand. A run fails \
         only when a judged pair that passed before fails now."
            .to_owned(),
        String::new(),
    ];
    if newest.is_empty() {
        lines.extend([NO_RUNS.to_owned(), String::new()]);
        return lines.join("\n");
    }
    lines.extend(runs_section(&newest));
    lines.extend(repos_section(&newest));
    lines.extend(failing_section(newest.first().copied()));
    lines.extend(regressions_section(&newest));
    lines.join("\n")
}

/// The commit, and the version in front of it when the commit is a release.
fn source_cell(source: &Source) -> String {
    source.version.as_ref().map_or_else(
        || format!("`{}`", source.short_commit()),
        |version| format!("**{version}** (`{}`)", source.short_commit()),
    )
}

/// The machine, on one line.
fn host_cell(host: &Host) -> String {
    format!(
        "{} · {} CPUs · {} · {} {}",
        host.cpu_model,
        host.logical_cpus,
        megabytes(Some(host.memory_mb)),
        host.os,
        host.os_version
    )
}

/// A titled table: a heading, the header and one row per entry.
fn table(title: &str, columns: &[&str], rows: impl Iterator<Item = String>) -> Vec<String> {
    let mut lines = vec![title.to_owned(), String::new()];
    lines.extend(header(
        &columns
            .iter()
            .map(|column| (*column).to_owned())
            .collect::<Vec<_>>(),
    ));
    lines.extend(rows);
    lines.push(String::new());
    lines
}

/// One row per run.
fn runs_section(newest: &[&RunRecord]) -> Vec<String> {
    table(
        "## Runs",
        &RUN_COLUMNS,
        newest.iter().map(|run| run_row(run)),
    )
}

/// One run's row.
fn run_row(run: &RunRecord) -> String {
    let totals = &run.totals;
    row(&[
        run.generated_at.clone(),
        source_cell(&run.source),
        (if run.passed { PASSED } else { FAILED }).to_owned(),
        format!("{}/{}", totals.correct, totals.judged),
        score_cell(totals.score_percent),
        run.regressions.to_string(),
        run.known_defects.to_string(),
        or_absent(totals.elapsed_ms, seconds),
        cpu(totals.cpu_seconds),
        megabytes(totals.peak_rss_mb),
        host_cell(&run.host),
    ])
}

/// One table per repository the latest run scored, a row per run.
fn repos_section(newest: &[&RunRecord]) -> Vec<String> {
    let mut lines = vec!["## By repository".to_owned(), String::new()];
    let latest = newest
        .first()
        .map_or_else(Vec::new, |run| run.repos.clone());
    for repo in &latest {
        let title = format!("### {} ({})", repo.name, repo.language);
        let rows = newest.iter().filter_map(|run| {
            let record = run.repos.iter().find(|scored| scored.name == repo.name)?;
            Some(repo_row(run, record))
        });
        lines.extend(table(&title, &REPO_COLUMNS, rows));
    }
    lines
}

/// One repository's row in one run.
fn repo_row(run: &RunRecord, repo: &RepoRecord) -> String {
    let cost = repo.cost.as_ref();
    row(&[
        run.generated_at.clone(),
        source_cell(&run.source),
        (if repo.matches_judgments {
            "✅ yes"
        } else {
            "🔴 no"
        })
        .to_owned(),
        format!(
            "{}/{} ({})",
            repo.correct,
            repo.judged,
            score_cell(repo.score_percent)
        ),
        format!("{}/{}", repo.clearly_in_found, repo.clearly_in_total),
        format!("{}/{}", repo.clearly_out_absent, repo.clearly_out_total),
        repo.false_negatives.to_string(),
        repo.false_positives.to_string(),
        or_absent(cost.map(|cost| cost.elapsed_ms), seconds),
        cpu(cost.and_then(|cost| cost.cpu_seconds)),
        cpu_rate(cost.and_then(|cost| cost.peak_cpu_percent)),
        megabytes(cost.and_then(|cost| cost.peak_rss_mb)),
    ])
}

/// Every judged pair the latest run gets wrong, and how it stands.
fn failing_section(latest: Option<&RunRecord>) -> Vec<String> {
    let Some(run) = latest else {
        return Vec::new();
    };
    let failing: Vec<&Judgment> = run
        .judgments
        .iter()
        .filter(|judgment| judgment.outcome == Outcome::Fail)
        .collect();
    let mut lines = vec![
        format!(
            "## Judged pairs the latest run gets wrong ({})",
            failing.len()
        ),
        String::new(),
    ];
    if failing.is_empty() {
        lines.push("None. Every judged pair is answered correctly.".to_owned());
    }
    lines.extend(failing.iter().map(|judgment| failing_line(judgment)));
    lines.push(String::new());
    lines
}

/// One failing pair and what failed: a regression in red, a known defect
/// otherwise.
fn failing_line(judgment: &Judgment) -> String {
    let mark = if judgment.regression {
        "🔴 regression"
    } else {
        "⚠️ known defect"
    };
    format!(
        "- {mark} — **{}** — {}, {} — `{}`",
        judgment.key.repo,
        judgment.key.defect(),
        judgment.failed.as_deref().unwrap_or_default(),
        judgment.key.occurrences.join("` + `")
    )
}

/// Every run that found a backslide, and each pair that went backwards.
fn regressions_section(newest: &[&RunRecord]) -> Vec<String> {
    let mut lines = vec!["## Regressions".to_owned(), String::new()];
    let backslid: Vec<&&RunRecord> = newest
        .iter()
        .filter(|run| !run.backslides.is_empty())
        .collect();
    if backslid.is_empty() {
        lines.push(
            "None. No judged pair that passed in one run has failed in a later one.".to_owned(),
        );
    }
    for run in backslid {
        lines.push(format!(
            "- {} at {}:",
            run.generated_at,
            source_cell(&run.source)
        ));
        lines.extend(run.backslides.iter().map(backslide_line));
    }
    lines.push(String::new());
    lines
}

/// One pair that went backwards.
fn backslide_line(backslide: &Backslide) -> String {
    format!(
        "  - 🔴 **{}** — {} — `{}` — passed in the run of {}",
        backslide.key.repo,
        backslide.key.defect(),
        backslide.key.occurrences.join("` + `"),
        backslide.passed_at
    )
}
