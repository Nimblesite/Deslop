//! [CORPUS-HISTORY] The run history, as the scorecard shows it: the source and
//! machine the run is recorded against, whether each repository matched its
//! judgments, and every judged pair that went backwards against an earlier run.

use super::{
    cells::{counted, cpu, cpu_rate, header, megabytes, or_absent, row, score_cell, seconds},
    Scorecard,
};
use crate::corpus_score::{
    history::{Backslide, Host, RepoRecord},
    source::Source,
};

/// A repository that matched every judgment, and one that did not.
const MATCHES: &str = "✅ yes";
/// See [`MATCHES`].
const MISMATCHES: &str = "🔴 no";
/// The per-repository table's columns.
const COLUMNS: [&str; 9] = [
    "repository",
    "matches judgments",
    "score",
    "IN found",
    "OUT absent",
    "wall",
    "CPU time",
    "peak CPU",
    "peak memory",
];

/// The history section, absent when the run was not held to the history.
pub(in crate::corpus_score) fn history_section(card: &Scorecard) -> Vec<String> {
    let Some(history) = card.history.as_ref() else {
        return Vec::new();
    };
    let mut lines = vec![
        "## Run history".to_owned(),
        String::new(),
        format!(
            "This run is recorded in `{}`, with every judged pair's outcome, and `{}` is rendered \
             again from every record. A judged pair that passed in any earlier run of the same \
             commit and fails now is a **backslide**, and one backslide fails the run.",
            history.runs, history.log
        ),
        String::new(),
        format!("Source: {}.", source_line(&history.source)),
        String::new(),
        format!("Machine: {}.", host_line(&history.host)),
        String::new(),
    ];
    lines.extend(header(&COLUMNS.map(str::to_owned)));
    lines.extend(history.repos.iter().map(repo_row));
    lines.push(String::new());
    lines.extend(backslide_lines(&history.backslides, history.earlier_runs));
    lines
}

/// The release version when the commit is one, then the full commit.
fn source_line(source: &Source) -> String {
    source.version.as_ref().map_or_else(
        || format!("commit `{}`", source.commit),
        |version| format!("**{version}**, commit `{}`", source.commit),
    )
}

/// The machine's facts on one line.
fn host_line(host: &Host) -> String {
    let physical = or_absent(host.physical_cores, |cores| cores.to_string());
    format!(
        "{} {} {} — {}, {} logical CPUs ({physical} physical), {}",
        host.os,
        host.os_version,
        host.arch,
        host.cpu_model,
        host.logical_cpus,
        megabytes(Some(host.memory_mb))
    )
}

/// One repository's row.
fn repo_row(repo: &RepoRecord) -> String {
    let cost = repo.cost.as_ref();
    row(&[
        format!("{} ({})", repo.name, repo.language),
        (if repo.matches_judgments {
            MATCHES
        } else {
            MISMATCHES
        })
        .to_owned(),
        score_cell(repo.score_percent),
        format!("{}/{}", repo.clearly_in_found, repo.clearly_in_total),
        format!("{}/{}", repo.clearly_out_absent, repo.clearly_out_total),
        or_absent(cost.map(|cost| cost.elapsed_ms), seconds),
        cpu(cost.and_then(|cost| cost.cpu_seconds)),
        cpu_rate(cost.and_then(|cost| cost.peak_cpu_percent)),
        megabytes(cost.and_then(|cost| cost.peak_rss_mb)),
    ])
}

/// Whether anything went backwards, and every pair that did.
fn backslide_lines(backslides: &[Backslide], earlier_runs: usize) -> Vec<String> {
    let runs = counted(earlier_runs, "earlier run", "earlier runs");
    if backslides.is_empty() {
        return vec![
            format!("**PASS** — no judged pair went backwards against {runs}."),
            String::new(),
        ];
    }
    let mut lines = vec![
        format!(
            "**FAIL** — {} went backwards against {runs}. Each passed in an earlier run of the \
             same commit and fails now:",
            counted(backslides.len(), "judged pair", "judged pairs")
        ),
        String::new(),
    ];
    lines.extend(backslides.iter().map(|backslide| {
        format!(
            "- 🔴 **{}** — `{}` — {}; passed in the run of {}",
            backslide.key.repo,
            backslide.key.occurrences.join("` + `"),
            backslide.key.defect(),
            backslide.passed_at
        )
    }));
    lines.push(String::new());
    lines
}
