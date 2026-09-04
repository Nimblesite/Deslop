//! [CORPUS-SCORE] Unit coverage for the scoring calculation and the scorecard.
//!
//! These isolate what no black-box run can pin precisely: that *overlap* is
//! what matches a judged pair (not exact line equality), that a hidden
//! occurrence satisfies nothing, that the two verdicts read one predicate in
//! opposite directions, that an unjudged register scores nothing rather than
//! scoring perfectly, and that cost is reported beside the score and never
//! folded into it.
//!
//! Results are taken with `?` rather than `expect`, so a broken fixture fails
//! by name instead of through a panic the workspace lint denies.

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{
    gate::{add_costs, breaches, corpus_change, degradation, totals, Thresholds},
    percent,
    render::{scorecard, Engine, Scorecard, TargetScore},
    score_repo, Range, RepoScore, RunCost, CLEARLY_IN, CLEARLY_OUT,
};

const REPO: &str = "fixture";
const PATH: &str = "src/one.rs";
const OTHER: &str = "src/two.rs";
const CLUSTER_ID: &str = "abc123";
const FIRST_RANGE: &str = "src/one.rs:10-20";
const SECOND_RANGE: &str = "src/two.rs:30-40";
const OLD_ENGINE: &str = "old";
const NEW_ENGINE: &str = "new";
/// A sha is forty characters; the value itself is irrelevant to scoring.
const PINNED_SHA_LENGTH: usize = 40;
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
const ACCURACY_ROW: &str =
    "| fixture (Rust) | 1 IN + 0 OUT | 100.0% | 0.0% | 1/1 | 0/1 | 0/0 | 0/0 | 1 new FN |";
const COST_ROW: &str =
    "| fixture (Rust) | 1 | 1 | 2.00 s | 2.50 s | 100 MB | 140 MB | 1.50 s | 1.50 s |";

fn occurrence(path: &str, start: u64, end: u64, hidden: bool) -> Value {
    json!({ "path": path, "start_line": start, "end_line": end, "hidden": hidden })
}

fn report(occurrences: &[Value]) -> Value {
    json!({ "clusters": [ { "id": CLUSTER_ID, "occurrences": occurrences } ] })
}

fn pinned_sha() -> String {
    "0".repeat(PINNED_SHA_LENGTH)
}

fn register(verdict: &str, ranges: &[&str]) -> Value {
    json!({
        "sha": pinned_sha(),
        verdict: [ { "why": "judged", "occurrences": ranges } ],
    })
}

fn the_pair() -> Vec<Value> {
    vec![
        occurrence(PATH, 10, 20, false),
        occurrence(OTHER, 30, 40, false),
    ]
}

/// The first entry of a score, named so a missing one fails by message.
fn first_entry(score: &RepoScore) -> Result<&super::ScoredEntry> {
    score
        .entries
        .first()
        .ok_or_else(|| anyhow!("the score records no entries"))
}

/// A score's percentage to one decimal place, so the assertions never compare
/// floats directly.
fn rendered_score(score: &RepoScore) -> String {
    score
        .score_percent
        .map_or_else(|| "not judged".to_owned(), |value| format!("{value:.1}"))
}

#[test]
fn a_range_parses_and_refuses_every_malformed_shape() -> Result<()> {
    let parsed = Range::parse("a/b.rs:10-20")?;
    assert_eq!(parsed.path, "a/b.rs");
    assert_eq!((parsed.start, parsed.end), (10, 20));
    assert!(Range::parse("a/b.rs").is_err(), "no span at all");
    assert!(Range::parse("a/b.rs:10").is_err(), "no end line");
    assert!(Range::parse("a/b.rs:20-10").is_err(), "inverted span");
    assert!(
        Range::parse("a/b.rs:0-3").is_err(),
        "line zero does not exist"
    );
    Ok(())
}

#[test]
fn overlap_matches_a_clearly_in_that_exact_line_equality_would_miss() -> Result<()> {
    let judged = register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]);
    // The engine reports wider extents than the judge wrote down. That is
    // extent drift, not a lost pairing, so it must still score as found.
    let drifted = report(&[
        occurrence(PATH, 8, 25, false),
        occurrence(OTHER, 28, 44, false),
    ]);
    let score = score_repo(REPO, &judged, &drifted)?;
    assert_eq!(
        score.clearly_in_found, 1,
        "overlap must match a drifted extent"
    );
    assert_eq!(score.false_negatives, 0);
    assert_eq!(rendered_score(&score), "100.0");
    assert_eq!(
        first_entry(&score)?.cluster.as_deref(),
        Some(CLUSTER_ID),
        "the matching cluster is named so a breach can be traced"
    );

    // One range short of the pair is not the pair.
    let partial = score_repo(REPO, &judged, &report(&[occurrence(PATH, 10, 20, false)]))?;
    assert_eq!(
        partial.false_negatives, 1,
        "half a pair is a false negative"
    );
    assert_eq!(partial.clearly_in_found, 0);
    assert_eq!(rendered_score(&partial), "0.0");
    Ok(())
}

#[test]
fn a_hidden_occurrence_neither_finds_nor_breaches() -> Result<()> {
    let unseen = [
        occurrence(PATH, 10, 20, true),
        occurrence(OTHER, 30, 40, true),
    ];
    let ranges = [FIRST_RANGE, SECOND_RANGE];

    let recall = score_repo(REPO, &register(CLEARLY_IN, &ranges), &report(&unseen))?;
    assert_eq!(
        recall.false_negatives, 1,
        "a pair the reader was never shown was not reported"
    );

    let precision = score_repo(REPO, &register(CLEARLY_OUT, &ranges), &report(&unseen))?;
    assert_eq!(
        precision.false_positives, 0,
        "a false positive nobody is shown is not a false positive"
    );
    assert_eq!(precision.clearly_out_absent, 1);
    Ok(())
}

#[test]
fn the_two_verdicts_read_one_predicate_in_opposite_directions() -> Result<()> {
    let ranges = [FIRST_RANGE, SECOND_RANGE];
    let paired = report(&the_pair());
    let silent = report(&[occurrence(PATH, 10, 20, false)]);

    let matched_in = score_repo(REPO, &register(CLEARLY_IN, &ranges), &paired)?;
    let matched_out = score_repo(REPO, &register(CLEARLY_OUT, &ranges), &paired)?;
    assert_eq!(matched_in.correct, 1, "a matched CLEARLY IN is correct");
    assert_eq!(
        matched_out.false_positives, 1,
        "a matched CLEARLY OUT is a false positive"
    );
    assert_eq!(rendered_score(&matched_out), "0.0");

    let missed_in = score_repo(REPO, &register(CLEARLY_IN, &ranges), &silent)?;
    let missed_out = score_repo(REPO, &register(CLEARLY_OUT, &ranges), &silent)?;
    assert_eq!(
        missed_in.false_negatives, 1,
        "an unmatched CLEARLY IN is a false negative"
    );
    assert_eq!(missed_out.correct, 1, "an unmatched CLEARLY OUT is correct");
    assert_eq!(rendered_score(&missed_out), "100.0");
    Ok(())
}

#[test]
fn an_unjudged_register_scores_nothing_rather_than_scoring_perfectly() -> Result<()> {
    let empty = json!({ "sha": pinned_sha(), "clearly_in": [], "clearly_out": [] });
    let score = score_repo(REPO, &empty, &report(&[]))?;
    assert_eq!(score.judged, 0);
    assert_eq!(
        rendered_score(&score),
        "not judged",
        "being asked nothing must never read as answering everything right"
    );
    assert!(percent(0, 0).is_none());
    assert_eq!(
        percent(4, 6).map(|value| format!("{value:.1}")),
        Some("66.7".to_owned()),
        "a ratio of small integers is reported to one decimal place"
    );
    Ok(())
}

#[test]
fn corpus_totals_weight_by_judged_pairs_not_by_repository() -> Result<()> {
    let ranges = [FIRST_RANGE, SECOND_RANGE];
    let three_missed = json!({
        "sha": pinned_sha(),
        "clearly_in": (0..3)
            .map(|_| json!({ "why": "judged", "occurrences": ranges }))
            .collect::<Vec<_>>(),
    });
    let missed = score_repo("big", &three_missed, &report(&[]))?;
    let found = score_repo(
        "small",
        &register(CLEARLY_IN, &ranges),
        &report(&the_pair()),
    )?;

    let summed = totals(&[missed, found]);
    assert_eq!(summed.repos, 2);
    assert_eq!((summed.judged, summed.correct), (4, 1));
    assert_eq!(summed.false_negatives, 3);
    assert_eq!(
        summed.score_percent.map(|value| format!("{value:.1}")),
        Some("25.0".to_owned()),
        "a two-pair repository must not outvote a six-pair one by averaging percentages"
    );
    Ok(())
}

#[test]
fn the_gate_defaults_to_strict_and_reads_every_override_from_config() {
    let strict = Thresholds::default();
    assert_eq!(strict.maximum_false_negatives, 0);
    assert_eq!(
        strict.maximum_false_positives, 0,
        "zero defects of either kind is what a perfect score is, in units that do not \
         re-scale with the register"
    );

    let config = json!({
        "defaults": { "maximum_false_negatives": 3 },
        "repos": { "Polly": { "maximum_false_positives": 2 } },
    });
    let inherited = Thresholds::for_repo(&config, "click");
    assert_eq!(
        inherited.maximum_false_negatives, 3,
        "defaults apply to unlisted repositories"
    );
    assert_eq!(
        inherited.maximum_false_positives, 0,
        "a field the config never sets stays strict"
    );

    let scoped = Thresholds::for_repo(&config, "Polly");
    assert_eq!(
        scoped.maximum_false_positives, 2,
        "a repository override wins"
    );
    assert_eq!(
        scoped.maximum_false_negatives, 3,
        "a field the override omits falls back to the default, not to the other allowance"
    );
}

#[test]
fn the_gate_breaches_on_a_new_defect_and_its_allowance_never_re_scales() -> Result<()> {
    let missed = score_repo(
        REPO,
        &register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]),
        &report(&[]),
    )?;

    let strict = breaches(&missed, &Thresholds::default());
    assert_eq!(
        strict.len(),
        1,
        "one false negative breaches the strict count gate, and nothing else: the gate \
         judges defect counts only, so one defect is one breach"
    );
    assert!(strict
        .iter()
        .any(|breach| breach.measure == "false negatives"));
    assert!(
        strict.iter().all(|breach| breach.measure != "score"),
        "the gate never judges a ratio: a percentage threshold is a defect allowance \
         divided by the register size, so it loosens itself as the register grows"
    );
    assert!(strict.iter().all(|breach| breach.repo == REPO));

    let tracked = Thresholds {
        maximum_false_negatives: 1,
        maximum_false_positives: 0,
    };
    assert!(
        breaches(&missed, &tracked).is_empty(),
        "a recorded, tracked defect is inside its gate"
    );

    // The allowance must mean the same thing at every register size. A gate that
    // tolerates one defect tolerates exactly one whether the register judges six
    // pairs or six hundred; a ratio would have widened with the denominator.
    let small = RepoScore {
        judged: 6,
        correct: 5,
        score_percent: percent(5, 6),
        ..missed.clone()
    };
    let grown = RepoScore {
        judged: 600,
        correct: 599,
        score_percent: percent(599, 600),
        ..missed.clone()
    };
    for score in [&small, &grown] {
        assert!(
            breaches(score, &tracked).is_empty(),
            "one tracked defect is inside a one-defect gate at every register size"
        );
        let two = RepoScore {
            false_negatives: 2,
            ..score.clone()
        };
        assert_eq!(
            breaches(&two, &tracked).len(),
            1,
            "two defects breach a one-defect gate at every register size"
        );
    }
    Ok(())
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

/// The same judged pair, found by one engine and missed by the other.
fn found_and_missed() -> Result<(RepoScore, RepoScore)> {
    let judged = register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]);
    Ok((
        score_repo(REPO, &judged, &report(&the_pair()))?,
        score_repo(REPO, &judged, &report(&[]))?,
    ))
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
        rendered.contains("0.00 s"),
        "an unmeasured wall is still zero seconds"
    );
    assert!(
        rendered.contains(UNMEASURED_PEAK_ROW),
        "an unmeasured peak reads as absent in every column, change included: {rendered}"
    );
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

#[test]
fn degradation_separates_a_new_defect_from_one_both_engines_share() -> Result<()> {
    let (found, missed) = found_and_missed()?;

    let fresh = degradation(&found, &missed);
    assert!(
        fresh.degraded,
        "a pair the new engine stopped finding is a degradation"
    );
    assert_eq!(fresh.new_false_negatives.len(), 1);
    assert_eq!(fresh.standing_false_negatives, 0);

    let standing = degradation(&missed, &missed);
    assert!(
        !standing.degraded,
        "a defect both engines share is standing, not slippage"
    );
    assert!(standing.new_false_negatives.is_empty());
    assert_eq!(standing.standing_false_negatives, 1);

    let fixed = degradation(&missed, &found);
    assert!(!fixed.degraded, "fixing a defect is never a degradation");
    assert_eq!(fixed.standing_false_negatives, 0);
    Ok(())
}
