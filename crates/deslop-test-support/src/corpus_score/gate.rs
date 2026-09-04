//! [CORPUS-SCORE-GATE] The thresholds a scored run is held to, and the totals
//! across every scored repository.
//!
//! Thresholds are configuration, never constants buried in code: the defaults
//! below are the strict answer (a judged repository must be perfect), and
//! `corpus/register/score-thresholds.json` records every place the engine is
//! not yet there, with the reason. An entry in that file is an admission that a
//! defect shipped — the same ratchet as `corpus/known-failures.json`. Loosening
//! one to make a run pass is prohibited; the only correct exit is fixing the
//! engine and tightening the number.

use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use super::{percent, RepoScore, DEFAULT_MAXIMUM_DEFECTS, DEFAULT_MINIMUM_SCORE_PERCENT};

/// Where the gate reads its thresholds.
pub const THRESHOLDS_PATH: &str = "corpus/register/score-thresholds.json";
/// Slack on the score comparison, in percentage points. A score is a ratio of
/// small integers, so a threshold written to one decimal place must not fail a
/// run that is exactly on it: 4/6 is 66.666…, and `66.7` is the honest way to
/// write that down.
const SCORE_TOLERANCE_PERCENT: f64 = 0.05;

/// The gate for one repository.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Thresholds {
    /// Lowest score this repository may record.
    pub minimum_score_percent: f64,
    /// Most false positives tolerated. Every one is a tracked defect.
    pub maximum_false_positives: usize,
    /// Most false negatives tolerated. Every one is a tracked defect.
    pub maximum_false_negatives: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            minimum_score_percent: DEFAULT_MINIMUM_SCORE_PERCENT,
            maximum_false_positives: DEFAULT_MAXIMUM_DEFECTS,
            maximum_false_negatives: DEFAULT_MAXIMUM_DEFECTS,
        }
    }
}

/// A threshold field read from config, falling back to the strict default.
fn field<T: Copy>(config: &Value, name: &str, read: fn(&Value) -> Option<T>, fallback: T) -> T {
    config.get(name).and_then(read).unwrap_or(fallback)
}

/// A count threshold, falling back when the field is absent or out of range.
fn count_field(entry: &Value, name: &str, fallback: usize) -> usize {
    entry
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(fallback)
}

impl Thresholds {
    /// Reads one repository's thresholds out of a parsed config document,
    /// falling back to the defaults field by field.
    #[must_use]
    pub fn for_repo(config: &Value, repo: &str) -> Self {
        let defaults = Self::read(
            config.get("defaults").unwrap_or(&Value::Null),
            Self::default(),
        );
        let scoped = config.get("repos").and_then(|repos| repos.get(repo));
        scoped.map_or(defaults, |entry| Self::read(entry, defaults))
    }

    /// Reads one threshold set, taking every absent field from `fallback`.
    fn read(entry: &Value, fallback: Self) -> Self {
        let count = |name, fallback| count_field(entry, name, fallback);
        Self {
            minimum_score_percent: field(
                entry,
                "minimum_score_percent",
                Value::as_f64,
                fallback.minimum_score_percent,
            ),
            maximum_false_positives: count(
                "maximum_false_positives",
                fallback.maximum_false_positives,
            ),
            maximum_false_negatives: count(
                "maximum_false_negatives",
                fallback.maximum_false_negatives,
            ),
        }
    }
}

/// Loads the thresholds config, or an empty document when none exists.
///
/// # Errors
///
/// Returns an error when the file exists but is not valid JSON — a malformed
/// gate must fail the run rather than silently fall back to the defaults.
pub fn load_thresholds(repo_root: &std::path::Path) -> Result<Value> {
    let path = repo_root.join(THRESHOLDS_PATH);
    if !path.exists() {
        return Ok(Value::Null);
    }
    crate::read_json(&path)
}

/// One breached threshold, stated so the message names the fix.
#[derive(Debug, Clone, Serialize)]
pub struct Breach {
    /// Repository whose gate was breached.
    pub repo: String,
    /// Which threshold it was.
    pub measure: String,
    /// What the gate allows, and what the run actually recorded.
    pub allowed: String,
    /// See [`Self::allowed`].
    pub actual: String,
}

/// Every threshold this repository's score breaches.
#[must_use]
pub fn breaches(score: &RepoScore, thresholds: &Thresholds) -> Vec<Breach> {
    let mut found = Vec::new();
    let mut breach = |measure: &str, allowed: String, actual: String| {
        found.push(Breach {
            repo: score.name.clone(),
            measure: measure.to_owned(),
            allowed,
            actual,
        });
    };
    if score.false_negatives > thresholds.maximum_false_negatives {
        breach(
            "false negatives",
            format!("at most {}", thresholds.maximum_false_negatives),
            score.false_negatives.to_string(),
        );
    }
    if score.false_positives > thresholds.maximum_false_positives {
        breach(
            "false positives",
            format!("at most {}", thresholds.maximum_false_positives),
            score.false_positives.to_string(),
        );
    }
    if let Some(actual) = score.score_percent {
        if actual + SCORE_TOLERANCE_PERCENT < thresholds.minimum_score_percent {
            breach(
                "score",
                format!("at least {:.1}%", thresholds.minimum_score_percent),
                format!("{actual:.1}%"),
            );
        }
    }
    found
}

/// Whether one engine lost ground against another on the same register.
///
/// This is the only evidence of an accuracy change. Cluster totals, duplication
/// percentages, rank movement and band movement are description.
#[derive(Debug, Clone, Serialize)]
pub struct Degradation {
    /// True when `after` introduces a defect `before` did not have.
    pub degraded: bool,
    /// Pairs `after` stopped finding, named by their ranges.
    pub new_false_negatives: Vec<String>,
    /// Pairs `after` started reporting, named by their ranges.
    pub new_false_positives: Vec<String>,
    /// Defects both engines share: real, but not slippage.
    pub standing_false_negatives: usize,
    /// See [`Self::standing_false_negatives`].
    pub standing_false_positives: usize,
}

/// The ranges of every entry one engine got wrong in the given direction.
fn wrong(score: &RepoScore, false_negative: bool) -> Vec<String> {
    score
        .entries
        .iter()
        .filter(|entry| {
            if false_negative {
                entry.is_false_negative()
            } else {
                entry.is_false_positive()
            }
        })
        .map(|entry| entry.occurrences.join(" + "))
        .collect()
}

/// Compares two engines' standing on one register.
#[must_use]
pub fn degradation(before: &RepoScore, after: &RepoScore) -> Degradation {
    let split = |direction: bool| {
        let prior = wrong(before, direction);
        let now = wrong(after, direction);
        let fresh: Vec<String> = now
            .iter()
            .filter(|entry| !prior.contains(entry))
            .cloned()
            .collect();
        let standing = now.len().saturating_sub(fresh.len());
        (fresh, standing)
    };
    let (new_false_negatives, standing_false_negatives) = split(true);
    let (new_false_positives, standing_false_positives) = split(false);
    Degradation {
        degraded: !new_false_negatives.is_empty() || !new_false_positives.is_empty(),
        new_false_negatives,
        new_false_positives,
        standing_false_negatives,
        standing_false_positives,
    }
}

/// The standing across every scored repository, for one engine.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CorpusTotals {
    /// Repositories scored.
    pub repos: usize,
    /// Judged entries, and how many were answered correctly.
    pub judged: usize,
    /// See [`Self::judged`].
    pub correct: usize,
    /// Judged entries the engine got wrong, in each direction.
    pub false_negatives: usize,
    /// See [`Self::false_negatives`].
    pub false_positives: usize,
    /// Clusters published across the corpus. Description, never scored.
    pub clusters_total: usize,
    /// Wall milliseconds summed across the scored repositories.
    pub elapsed_ms: u64,
    /// Highest peak resident set any one scan reached, in mebibytes. The runs
    /// are sequential, so the corpus peak is the largest of them, not the sum.
    /// Absent when the platform measured none — never printed as zero.
    pub peak_rss_mb: Option<u64>,
    /// CPU seconds summed across the scored repositories, when measured.
    pub cpu_seconds: Option<f64>,
    /// `100 * correct / judged` across every judged entry in the corpus.
    pub score_percent: Option<f64>,
}

/// Sums one engine's scores into the corpus standing.
///
/// The corpus score is `correct / judged` over **every judged entry**, not the
/// mean of the per-repository scores: averaging percentages would let a
/// repository with two judged pairs outvote one with two hundred.
#[must_use]
pub fn totals(scores: &[RepoScore]) -> CorpusTotals {
    let sum = |pick: fn(&RepoScore) -> usize| scores.iter().map(pick).sum();
    let judged = sum(|score| score.judged);
    let correct = sum(|score| score.correct);
    CorpusTotals {
        repos: scores.len(),
        judged,
        correct,
        false_negatives: sum(|score| score.false_negatives),
        false_positives: sum(|score| score.false_positives),
        clusters_total: sum(|score| score.clusters_total),
        score_percent: percent(correct, judged),
        ..CorpusTotals::default()
    }
}

/// Adds the measured cost of every run to an engine's totals.
pub fn add_costs(totals: &mut CorpusTotals, costs: &BTreeMap<String, super::RunCost>) {
    totals.elapsed_ms = costs.values().map(|cost| cost.elapsed_ms).sum();
    totals.peak_rss_mb = costs.values().filter_map(|cost| cost.peak_rss_mb).max();
    let cpu: Vec<f64> = costs.values().filter_map(|cost| cost.cpu_seconds).collect();
    totals.cpu_seconds = (!cpu.is_empty()).then(|| cpu.iter().sum());
}
