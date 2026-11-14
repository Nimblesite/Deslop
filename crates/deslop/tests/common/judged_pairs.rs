//! Judged string-heavy pairs as test input ([FUSED-CONTENT-GATE-LITERAL-BOUND],
//! [CORPUS-REGISTER]).
//!
//! `fixtures/judged-string-pairs/` holds verbatim excerpts of corpus repos,
//! each a pair the corpus register judges CLEARLY IN whose text is mostly
//! string literals: embedded programs, docstrings, error messages, HTTP
//! headers, YAML documents. `register.json` beside them is the register's
//! own entries re-based to the excerpt lines, with `judged_at` naming the
//! original pair. Scans score through the corpus gate's own scorer, so a
//! test here fails exactly where the score gate would.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
pub(crate) use deslop_test_support::copy_tree;
use deslop_test_support::corpus_score::{
    score_repo, RepoScore, ScoredEntry, CLEARLY_IN, CLEARLY_OUT,
};
use serde_json::{json, Value};

use super::{clusters, field, fixture, load_json, occurrence_line_span, occurrences, run_report};

/// The fixture tree of judged excerpts.
pub(crate) const JUDGED_FIXTURE: &str = "judged-string-pairs";
/// The register file beside the excerpts.
const REGISTER_FILE: &str = "register.json";
/// The node floor every corpus-style scan here runs at.
pub(crate) const MIN_NODES: u32 = 30;
/// The least share of a judged pair's lines the matching cluster must show:
/// a cluster that touches one line of each side has not reported the copy.
pub(crate) const MIN_JUDGED_COVERAGE_PERCENT: f64 = 50.0;

/// Each language's repositories in the fixture tree, by top-level directory.
pub(crate) const LANGUAGES: [(&str, &[&str]); 8] = [
    ("C#", &["BenchmarkDotNet"]),
    ("Rust", &["serde"]),
    ("Python", &["fastapi", "flask"]),
    ("TypeScript", &["vite"]),
    ("JavaScript", &["express", "date-fns"]),
    ("PHP", &["console", "monolog"]),
    ("Dart", &["http"]),
    ("Go", &["viper"]),
];

/// Every repository directory in the fixture tree.
pub(crate) fn all_repos() -> Vec<&'static str> {
    LANGUAGES
        .iter()
        .flat_map(|(_, repos)| repos.iter().copied())
        .collect()
}

/// The judged register for the fixture tree.
pub(crate) fn register() -> Result<Value> {
    load_json(&fixture(JUDGED_FIXTURE).join(REGISTER_FILE))
}

/// Whether a written range `repo/path:a-b` lies under one of `repos`.
fn under(range: &str, repos: &[&str]) -> bool {
    range
        .split_once('/')
        .is_some_and(|(repo, _)| repos.contains(&repo))
}

/// The register narrowed to the entries whose every range lies under `repos`.
pub(crate) fn register_for(register: &Value, repos: &[&str]) -> Value {
    let keep = |verdict: &str| -> Vec<Value> {
        field(register, verdict)
            .as_array()
            .into_iter()
            .flatten()
            .filter(|entry| {
                field(entry, "occurrences")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .all(|range| range.as_str().is_some_and(|text| under(text, repos)))
            })
            .cloned()
            .collect()
    };
    json!({
        "name": field(register, "name"),
        "sha": field(register, "sha"),
        CLEARLY_IN: keep(CLEARLY_IN),
        CLEARLY_OUT: keep(CLEARLY_OUT),
        "not_clear": [],
    })
}

/// Copies the named repositories of the fixture into `scan_root`.
pub(crate) fn seed_repos(scan_root: &Path, repos: &[&str]) -> Result<()> {
    let source = fixture(JUDGED_FIXTURE);
    repos
        .iter()
        .try_for_each(|repo| copy_tree(&source.join(repo), &scan_root.join(repo)))
}

/// Every file under `root`, sorted.
pub(crate) fn files_under(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(files_under(&path)?);
        } else {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Scans `scan_root` and scores it against `register` as the gate does.
pub(crate) fn scan_and_score(scan_root: &Path, register: &Value) -> Result<(Value, RepoScore)> {
    let report = run_report(scan_root, MIN_NODES)?;
    let score = score_repo(JUDGED_FIXTURE, register, &report)?;
    Ok((report, score))
}

/// One wrong entry, written so the failure names the judged pair.
fn describe(entry: &ScoredEntry) -> String {
    format!(
        "{} {:?} reported as {:?}: {}",
        entry.verdict, entry.occurrences, entry.reported_as, entry.why
    )
}

/// Asserts every judged entry is answered correctly, and that each
/// CLEARLY IN cluster shows at least [`MIN_JUDGED_COVERAGE_PERCENT`] of
/// the judged lines.
pub(crate) fn assert_every_judgment_holds(score: &RepoScore, scenario: &str) {
    let wrong: Vec<String> = score
        .entries
        .iter()
        .filter(|entry| !entry.correct)
        .map(describe)
        .collect();
    assert!(
        wrong.is_empty(),
        "{scenario}: judged pairs answered wrongly:\n{}",
        wrong.join("\n")
    );
    assert_eq!(score.false_negatives, 0, "{scenario}");
    assert_eq!(score.false_positives, 0, "{scenario}");
    assert_eq!(score.correct, score.judged, "{scenario}");
    assert!(
        score.judged > 0,
        "{scenario}: the register narrowed to nothing"
    );
    for entry in score
        .entries
        .iter()
        .filter(|entry| entry.verdict == CLEARLY_IN)
    {
        let shown = entry
            .coverage
            .as_ref()
            .and_then(|coverage| coverage.percent);
        assert!(
            shown.is_some_and(|percent| percent >= MIN_JUDGED_COVERAGE_PERCENT),
            "{scenario}: {:?} is reported but shows only {shown:?}% of the judged lines",
            entry.occurrences
        );
    }
}

/// The reported cluster that answered one judged entry.
pub(crate) fn answering_cluster<'a>(report: &'a Value, entry: &ScoredEntry) -> Result<&'a Value> {
    let id = entry
        .cluster
        .as_deref()
        .with_context(|| format!("judged pair is not reported: {:?}", entry.occurrences))?;
    clusters(report)
        .iter()
        .find(|cluster| field(cluster, "id").as_str() == Some(id))
        .with_context(|| format!("cluster {id} is not in the report"))
}

/// For each judged entry, the line spans of the cluster that answered it,
/// in register order; the shape two equivalent scans must share.
pub(crate) fn answering_spans(report: &Value, score: &RepoScore) -> Result<Vec<Vec<(u64, u64)>>> {
    score
        .entries
        .iter()
        .map(|entry| {
            let cluster = answering_cluster(report, entry)?;
            let mut spans: Vec<(u64, u64)> = occurrences(cluster)
                .iter()
                .map(occurrence_line_span)
                .collect();
            spans.sort_unstable();
            Ok(spans)
        })
        .collect()
}

/// `object` with `key` set to `value`; any other value comes back unchanged.
pub(crate) fn with_list(object: &Value, key: &str, value: Value) -> Value {
    let mut changed = object.clone();
    if let Some(map) = changed.as_object_mut() {
        let _previous = map.insert(key.to_owned(), value);
    }
    changed
}
