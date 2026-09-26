//! [CORPUS-SCORE-RENDER] What the scorecard document must say.
//!
//! The scoring tests next door pin the numbers; these pin the page a reader
//! actually gets — that every comparison reads across one row, that an
//! unmeasured run says so rather than reading as free, that a breach names the
//! measure it breached, and that the document states the scope it covers
//! before it states a score.

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use serde_json::Value;

use super::super::{gate::CorpusTotals, CLEARLY_IN};
use super::{
    add_costs, breaches, corpus_change, degradation, found_and_missed, occurrence, pinned_sha,
    register, report, score_repo, scorecard, totals, Engine, RepoScore, RunCost, Scorecard,
    TargetScore, Thresholds, FIRST_RANGE, OTHER, PATH, REPO, SECOND_RANGE,
};

const OLD_ENGINE: &str = "old";
const NEW_ENGINE: &str = "new";
/// Measured costs the rendering tests assert against.
const OLD_ELAPSED_MS: u64 = 2000;
const NEW_ELAPSED_MS: u64 = 2500;
const OLD_PEAK_MB: u64 = 100;
const NEW_PEAK_MB: u64 = 140;
const CPU_SECONDS: f64 = 1.5;
/// Engine labels, and the language the fixture repository is filed under.
const OLD_LABEL: &str = "engine-old";
const NEW_LABEL: &str = "engine-new";
const LANGUAGE: &str = "Rust";
/// The opening cell of every per-repository row. Each per-repository table
/// carries exactly one such row — one row per repository, never one per engine.
const REPO_ROW: &str = "| fixture (Rust) |";
/// The tables that carry one row per repository: accuracy, then cost.
const REPO_TABLES: usize = 2;
/// The rendered rows the layout tests pin, each one a whole comparison read
/// across a single line rather than reassembled from two.
const SCORE_ROW: &str = "| score | 100.0% | 0.0% | -100.0 pts |";
const CORRECT_ROW: &str = "| correct / judged | 1/1 | 0/1 | -1 correct |";
const WALL_ROW: &str = "| wall | 2.00 s | 2.50 s | +500 ms |";
const PEAK_ROW: &str = "| peak RSS | 100 MB | 140 MB | +40 MB |";
const UNMEASURED_PEAK_ROW: &str = "| peak RSS | — | — | — |";
const UNMEASURED_WALL_ROW: &str = "| wall | — | — | — |";
const BOTH_ELAPSED_MS: u64 = OLD_ELAPSED_MS + NEW_ELAPSED_MS;
const BOTH_CPU_SECONDS: f64 = CPU_SECONDS + CPU_SECONDS;
const TWO_REPOS: usize = 2;
const TOTALS_FIELD: &str = "totals";
const CHANGE_FIELD: &str = "change";
const REPOS_FIELD: &str = "repos";
const ELAPSED_FIELD: &str = "elapsed_ms";
const CPU_FIELD: &str = "cpu_seconds";
const PEAK_FIELD: &str = "peak_rss_mb";
const COVERAGE_FIELD: &str = "matched_in_coverage";
const COVERED_FIELD: &str = "covered_lines";
const JUDGED_FIELD: &str = "judged_lines";
const ACCURACY_ROW: &str =
    "| fixture (Rust) | 1 IN + 0 OUT | 100.0% | 0.0% | 1/1 | 0/1 | 0/0 | 0/0 | 1 new FN |";
/// Extra repositories the scope test widens the card with, one of them sharing
/// `LANGUAGE` so a language count can never be a repository count in disguise.
const SECOND_REPO: &str = "second";
const THIRD_REPO: &str = "third";
const SECOND_LANGUAGE: &str = "Python";
/// The scope the renderer must state, counted over those repositories.
const WIDE_SCOPE_LINE: &str = "Scope: **3 repositories** across **2 languages** — Python, Rust.";
const SINGLE_SCOPE_LINE: &str = "Scope: **1 repository** across **1 language** — Rust.";
const COST_ROW: &str =
    "| fixture (Rust) | 1 | 1 | 2.00 s | 2.50 s | 100 MB | 140 MB | 1.50 s | 1.50 s |";
const COVERAGE_ROW: &str =
    "| fixture | IN | `src/one.rs:10-20` + `src/two.rs:30-40` | 22/22 (100.0%) | 7/22 (31.8%) |";
const AGGREGATE_COVERAGE_ROW: &str = "| matched IN coverage | 22/22 (100.0%) | 7/22 (31.8%) | — |";
const PARTIAL_FIRST_START: u64 = 19;
const PARTIAL_FIRST_END: u64 = 21;
const PARTIAL_SECOND_START: u64 = 30;
const PARTIAL_SECOND_END: u64 = 34;
const PARTIAL_COVERED_LINES: u64 = 7;
const JUDGED_LINES: u64 = 22;

/// Reads a serialized scorecard field without hiding a missing parent.
fn field<'a>(document: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(document, |current, key| current.get(*key))
}

/// A measured cost for the rendering tests.
fn cost(elapsed_ms: u64, peak_rss_mb: u64) -> RunCost {
    RunCost {
        elapsed_ms,
        peak_rss_mb: Some(peak_rss_mb),
        cpu_seconds: Some(CPU_SECONDS),
        binary_sha256: "deadbeef".to_owned(),
    }
}

/// Two judged repositories isolate incomplete aggregate measurements.
fn paired_totals() -> Result<CorpusTotals> {
    let (found, missed) = found_and_missed()?;
    Ok(totals(&[found, missed]))
}

/// A fully measured pair that each partial-cost test can change one field in.
fn paired_costs() -> BTreeMap<String, RunCost> {
    BTreeMap::from([
        (REPO.to_owned(), cost(OLD_ELAPSED_MS, OLD_PEAK_MB)),
        (SECOND_REPO.to_owned(), cost(NEW_ELAPSED_MS, NEW_PEAK_MB)),
    ])
}

/// A two-engine scorecard over one repository.
fn card(before: &RepoScore, after: &RepoScore) -> Scorecard {
    let engine_totals = |score: &RepoScore, elapsed, peak| {
        let mut summed = totals(std::slice::from_ref(score));
        add_costs(
            &mut summed,
            &BTreeMap::from([(REPO.to_owned(), cost(elapsed, peak))]),
        );
        summed
    };
    let before_totals = engine_totals(before, OLD_ELAPSED_MS, OLD_PEAK_MB);
    let after_totals = engine_totals(after, NEW_ELAPSED_MS, NEW_PEAK_MB);
    Scorecard {
        generated_at: "2026-09-04T00:00:00Z".to_owned(),
        engines: vec![
            Engine {
                id: OLD_ENGINE.to_owned(),
                label: OLD_LABEL.to_owned(),
            },
            Engine {
                id: NEW_ENGINE.to_owned(),
                label: NEW_LABEL.to_owned(),
            },
        ],
        targets: vec![TargetScore {
            name: REPO.to_owned(),
            language: LANGUAGE.to_owned(),
            sha: pinned_sha(),
            scores: BTreeMap::from([
                (OLD_ENGINE.to_owned(), before.clone()),
                (NEW_ENGINE.to_owned(), after.clone()),
            ]),
            costs: BTreeMap::from([
                (OLD_ENGINE.to_owned(), cost(OLD_ELAPSED_MS, OLD_PEAK_MB)),
                (NEW_ENGINE.to_owned(), cost(NEW_ELAPSED_MS, NEW_PEAK_MB)),
            ]),
            degradation: Some(degradation(before, after)),
        }],
        change: Some(corpus_change(&before_totals, &after_totals)),
        totals: BTreeMap::from([
            (OLD_ENGINE.to_owned(), before_totals),
            (NEW_ENGINE.to_owned(), after_totals),
        ]),
        thresholds: BTreeMap::from([(REPO.to_owned(), Thresholds::default())]),
        breaches: Vec::new(),
    }
}

#[test]
fn the_scorecard_reports_cost_beside_the_score_and_never_folds_it_in() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let rendered = scorecard(&card(&found, &missed));

    assert!(
        rendered.contains(SCORE_ROW),
        "the score change is stated beside both engines' scores: {rendered}"
    );
    assert!(
        rendered.contains(CORRECT_ROW),
        "correct-of-judged is stated for both engines: {rendered}"
    );
    assert!(
        rendered.contains(WALL_ROW),
        "wall time and its change are reported: {rendered}"
    );
    assert!(
        rendered.contains("| CPU | 1.50 s | 1.50 s |"),
        "CPU seconds are reported per engine: {rendered}"
    );
    assert!(
        rendered.contains(PEAK_ROW),
        "peak memory and its change are reported: {rendered}"
    );
    assert!(
        rendered.contains("never folded into it"),
        "the document says cost is description, not score"
    );
    assert!(
        rendered.contains("**FALSE NEGATIVE** fixture"),
        "the pair the engine got wrong is named: {rendered}"
    );
    assert!(
        rendered.contains("src/one.rs:10-20` + `src/two.rs:30-40"),
        "the defect names the code, not just a count"
    );
    Ok(())
}

#[test]
fn the_scorecard_describes_how_much_of_each_pair_was_reported() -> Result<()> {
    let (found, _) = found_and_missed()?;
    let partial = score_repo(
        REPO,
        &register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]),
        &report(&[
            occurrence(PATH, PARTIAL_FIRST_START, PARTIAL_FIRST_END, false),
            occurrence(OTHER, PARTIAL_SECOND_START, PARTIAL_SECOND_END, false),
        ]),
    )?;
    let compared = card(&found, &partial);
    let recorded = serde_json::to_value(&compared)?;
    assert_eq!(
        field(
            &recorded,
            &[TOTALS_FIELD, NEW_ENGINE, COVERAGE_FIELD, COVERED_FIELD]
        ),
        Some(&Value::from(PARTIAL_COVERED_LINES)),
        "score.json must carry the same aggregate covered-line count as SCORE.md"
    );
    assert_eq!(
        field(
            &recorded,
            &[TOTALS_FIELD, NEW_ENGINE, COVERAGE_FIELD, JUDGED_FIELD]
        ),
        Some(&Value::from(JUDGED_LINES)),
        "the aggregate denominator is the judged extent of reported positive pairs"
    );
    let rendered = scorecard(&compared);
    assert!(
        rendered.contains(COVERAGE_ROW),
        "coverage is description beside both engines for the same judged pair: {rendered}"
    );
    assert!(
        rendered.contains(AGGREGATE_COVERAGE_ROW),
        "the corpus standing must expose a mechanical aggregate of judged-line coverage: {rendered}"
    );
    Ok(())
}

#[test]
fn an_unmeasured_run_renders_as_absent_rather_than_as_zero() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let mut unmeasured = card(&found, &missed);
    for target in &mut unmeasured.targets {
        target.costs = BTreeMap::new();
    }
    for engine in unmeasured.totals.values_mut() {
        add_costs(engine, &BTreeMap::new());
    }
    let standing = |id: &str| {
        unmeasured
            .totals
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("no standing for engine {id}"))
    };
    unmeasured.change = Some(corpus_change(
        &standing(OLD_ENGINE)?,
        &standing(NEW_ENGINE)?,
    ));
    let rendered = scorecard(&unmeasured);
    assert!(
        !rendered.contains("0 MB"),
        "an unmeasured peak must never print as zero memory: {rendered}"
    );
    assert!(
        rendered.contains(UNMEASURED_WALL_ROW),
        "missing wall timing must read as absent: {rendered}"
    );
    assert!(
        rendered.contains(UNMEASURED_PEAK_ROW),
        "an unmeasured peak reads as absent in every column, change included: {rendered}"
    );
    let serialized = serde_json::to_value(&unmeasured)?;
    assert_eq!(
        field(&serialized, &[TOTALS_FIELD, OLD_ENGINE, ELAPSED_FIELD]),
        Some(&Value::Null)
    );
    assert_eq!(
        field(&serialized, &[CHANGE_FIELD, ELAPSED_FIELD]),
        Some(&Value::Null)
    );
    Ok(())
}

// [CORPUS-SCORE-COST-COMPLETE] Missing measurements cannot create a fast total.
#[test]
fn missing_repository_timing_does_not_publish_partial_aggregate_costs() -> Result<()> {
    let mut standing = paired_totals()?;
    add_costs(
        &mut standing,
        &BTreeMap::from([(REPO.to_owned(), cost(OLD_ELAPSED_MS, OLD_PEAK_MB))]),
    );
    let serialized = serde_json::to_value(&standing)?;
    assert_eq!(
        field(&serialized, &[REPOS_FIELD]),
        Some(&Value::from(TWO_REPOS))
    );
    assert_eq!(field(&serialized, &[ELAPSED_FIELD]), Some(&Value::Null));
    assert_eq!(field(&serialized, &[CPU_FIELD]), Some(&Value::Null));
    assert_eq!(field(&serialized, &[PEAK_FIELD]), Some(&Value::Null));
    Ok(())
}

#[test]
fn partial_cpu_timing_keeps_complete_wall_and_peak_without_inventing_cpu() -> Result<()> {
    let mut standing = paired_totals()?;
    let mut costs = paired_costs();
    costs
        .get_mut(SECOND_REPO)
        .ok_or_else(|| anyhow!("second run absent"))?
        .cpu_seconds = None;
    add_costs(&mut standing, &costs);
    let serialized = serde_json::to_value(&standing)?;
    assert_eq!(
        field(&serialized, &[ELAPSED_FIELD]),
        Some(&Value::from(BOTH_ELAPSED_MS))
    );
    assert_eq!(
        field(&serialized, &[PEAK_FIELD]),
        Some(&Value::from(NEW_PEAK_MB))
    );
    assert_eq!(field(&serialized, &[CPU_FIELD]), Some(&Value::Null));
    Ok(())
}

#[test]
fn partial_peak_timing_keeps_complete_wall_and_cpu_without_inventing_peak() -> Result<()> {
    let mut standing = paired_totals()?;
    let mut costs = paired_costs();
    costs
        .get_mut(SECOND_REPO)
        .ok_or_else(|| anyhow!("second run absent"))?
        .peak_rss_mb = None;
    add_costs(&mut standing, &costs);
    let serialized = serde_json::to_value(&standing)?;
    assert_eq!(
        field(&serialized, &[ELAPSED_FIELD]),
        Some(&Value::from(BOTH_ELAPSED_MS))
    );
    assert_eq!(
        field(&serialized, &[CPU_FIELD]),
        Some(&Value::from(BOTH_CPU_SECONDS))
    );
    assert_eq!(field(&serialized, &[PEAK_FIELD]), Some(&Value::Null));
    Ok(())
}

#[test]
fn the_engines_sit_side_by_side_so_one_row_carries_the_whole_comparison() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let rendered = scorecard(&card(&found, &missed));

    assert_eq!(
        rendered.matches(REPO_ROW).count(),
        REPO_TABLES,
        "one row per repository in each table, never one row per engine: {rendered}"
    );
    assert!(
        rendered.contains(&format!("score `{OLD_ENGINE}` | score `{NEW_ENGINE}`")),
        "each engine heads its own column, in run order: {rendered}"
    );
    assert!(
        rendered.contains(ACCURACY_ROW),
        "the repository's whole accuracy comparison reads across one row: {rendered}"
    );
    assert!(
        rendered.contains(COST_ROW),
        "the repository's whole cost comparison reads across one row: {rendered}"
    );
    assert!(
        rendered.contains(&format!(
            "| measure | `{OLD_LABEL}` | `{NEW_LABEL}` | change |"
        )),
        "the corpus standing gives each engine a column and states the change: {rendered}"
    );
    Ok(())
}

#[test]
fn a_breached_gate_renders_as_a_failure_that_names_the_measure() -> Result<()> {
    let (found, missed) = found_and_missed()?;
    let mut breached = card(&found, &missed);
    breached.breaches = breaches(&missed, &Thresholds::default());
    let rendered = scorecard(&breached);
    assert!(
        rendered.contains("**FAIL**"),
        "a breach renders as a failure"
    );
    assert!(
        rendered.contains("false negatives: allows at most 0, recorded 1"),
        "the breach names the measure, the allowance and the actual: {rendered}"
    );
    assert!(
        !rendered.contains("**PASS**"),
        "a failing scorecard must not also claim to pass"
    );
    Ok(())
}

#[test]
fn a_clean_scorecard_says_pass_and_names_no_defect() -> Result<()> {
    let (found, _) = found_and_missed()?;
    let rendered = scorecard(&card(&found, &found));
    assert!(rendered.contains("**PASS**"));
    assert!(rendered.contains("None. Every judged pair is answered correctly."));
    assert!(!rendered.contains("FALSE NEGATIVE"));
    assert!(
        rendered.contains("— correct"),
        "no change in correct answers is stated as no change"
    );
    Ok(())
}

/// The same scorecard widened to several repositories, so a count the renderer
/// prints can never be right by accident: two languages over three
/// repositories, one language carrying two of them.
fn wide_card(before: &RepoScore, after: &RepoScore) -> Result<Scorecard> {
    let mut wide = card(before, after);
    let first = wide
        .targets
        .first()
        .ok_or_else(|| anyhow!("the fixture card names no target"))?
        .clone();
    let target = |name: &str, language: &str| {
        let mut copy = first.clone();
        copy.name = name.to_owned();
        copy.language = language.to_owned();
        copy
    };
    wide.targets = vec![
        first.clone(),
        target(SECOND_REPO, SECOND_LANGUAGE),
        target(THIRD_REPO, LANGUAGE),
    ];
    Ok(wide)
}

#[test]
fn the_scorecard_states_how_many_repositories_and_languages_it_covers() -> Result<()> {
    let (found, missed) = found_and_missed()?;

    let wide = scorecard(&wide_card(&found, &missed)?);
    assert!(
        wide.contains(WIDE_SCOPE_LINE),
        "the scope counts repositories and DISTINCT languages, and names them: {wide}"
    );
    assert_eq!(
        wide.matches(REPO_ROW).count(),
        REPO_TABLES,
        "the fixture repository still occupies exactly one row per table: {wide}"
    );
    for repo in [SECOND_REPO, THIRD_REPO] {
        assert!(
            wide.contains(repo),
            "every counted repository is also a row in the tables: {wide}"
        );
    }

    let single = scorecard(&card(&found, &missed));
    assert!(
        single.contains(SINGLE_SCOPE_LINE),
        "one repository reads as a singular, never as `1 repositories`: {single}"
    );
    assert!(
        !single.contains(SECOND_LANGUAGE),
        "a language nothing was scanned in is never named: {single}"
    );
    Ok(())
}
