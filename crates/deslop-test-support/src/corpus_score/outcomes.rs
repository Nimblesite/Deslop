//! [CORPUS-SCORE-OUTCOMES] Every judged pair's pass or fail, on record.
//!
//! `corpus/register/outcomes.json` holds the last recorded outcome of every
//! judged pair a run has scored, and since when it has held it. A run reads
//! the record before it judges anything: a pair on record as failing is a
//! known defect and is tolerated, while a pair that fails after passing, or
//! fails with no record at all, fails the run. A count could not tell those
//! apart — one pair fixed and another broken in the same change leave every
//! count where it was.
//!
//! Every run writes the record back, and the record only ever tightens: a
//! fixed pair is recorded as passing, a newly judged passing pair is recorded,
//! and a regression is never written over the pass it broke. A failing pair
//! goes on record only when a run is told to accept it.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{render::TargetScore, ScoredEntry, CLEARLY_IN};
use crate::read_json;

/// Where the record lives, relative to the repository root.
pub const OUTCOMES_PATH: &str = "corpus/register/outcomes.json";
/// The spec section the record points its reader at.
const OUTCOMES_SPEC: &str = "docs/specs/corpus.md [CORPUS-SCORE-OUTCOMES]";
/// What a wrong answer on each verdict is called.
const FALSE_NEGATIVE: &str = "false negative";
/// See [`FALSE_NEGATIVE`].
const FALSE_POSITIVE: &str = "false positive";

/// Whether the engine answered a judged pair correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Answered correctly.
    Pass,
    /// Answered wrongly: a false negative or a false positive.
    Fail,
}

impl Outcome {
    /// The outcome of one scored entry.
    fn of(entry: &ScoredEntry) -> Self {
        if entry.correct {
            Self::Pass
        } else {
            Self::Fail
        }
    }
}

/// What identifies one judged pair: its repository, its verdict and its
/// ranges, sorted so the order a register lists them in never matters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PairKey {
    /// The repository the register judges, named as its register file is —
    /// in lower case — so every entry point keys it the same way.
    pub repo: String,
    /// [`CLEARLY_IN`] or [`super::CLEARLY_OUT`].
    pub verdict: String,
    /// The judged ranges, sorted.
    pub occurrences: Vec<String>,
}

impl PairKey {
    /// The key of one scored entry in `repo`.
    #[must_use]
    pub fn of(repo: &str, entry: &ScoredEntry) -> Self {
        let mut occurrences = entry.occurrences.clone();
        occurrences.sort();
        Self {
            repo: repo.to_lowercase(),
            verdict: entry.verdict.clone(),
            occurrences,
        }
    }

    /// What a wrong answer on this pair is called.
    #[must_use]
    pub fn defect(&self) -> &'static str {
        if self.verdict == CLEARLY_IN {
            FALSE_NEGATIVE
        } else {
            FALSE_POSITIVE
        }
    }
}

/// One judged pair as the record holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedPair {
    /// Which pair.
    #[serde(flatten)]
    pub key: PairKey,
    /// Its last recorded outcome.
    pub outcome: Outcome,
    /// When the run that recorded this outcome was generated.
    pub since: String,
    /// The engine that run scored.
    pub engine: String,
}

/// The whole record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    /// Where the rules for this file are written.
    pub spec: String,
    /// Every recorded pair, sorted by [`PairKey`].
    pub pairs: Vec<RecordedPair>,
}

/// How one judged pair's outcome in this run stands against its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Passes, and is not on record as failing.
    Passing,
    /// Passes, and was on record as failing.
    Fixed,
    /// Fails, and is on record as failing: a known defect, tolerated.
    StillFailing,
    /// Fails, with no record: a defect nobody accepted.
    Unrecorded,
    /// Fails, and was on record as passing.
    Regressed,
}

impl Standing {
    /// How `now` stands against `recorded`.
    fn of(now: Outcome, recorded: Option<Outcome>) -> Self {
        match (now, recorded) {
            (Outcome::Pass, Some(Outcome::Fail)) => Self::Fixed,
            (Outcome::Pass, _) => Self::Passing,
            (Outcome::Fail, Some(Outcome::Fail)) => Self::StillFailing,
            (Outcome::Fail, Some(Outcome::Pass)) => Self::Regressed,
            (Outcome::Fail, None) => Self::Unrecorded,
        }
    }

    /// Whether this standing fails the run.
    #[must_use]
    pub fn fails_the_run(self) -> bool {
        matches!(self, Self::Unrecorded | Self::Regressed)
    }
}

/// One judged pair the last engine was scored on, and how it stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PairOutcome {
    /// Which pair.
    #[serde(flatten)]
    pub key: PairKey,
    /// What this run made of it.
    pub outcome: Outcome,
    /// How that stands against the record.
    pub standing: Standing,
    /// Since when the record has held its outcome; absent with no record.
    pub recorded_since: Option<String>,
}

impl PairOutcome {
    /// Says how this pair stands, for the verdict and the scorecard alike.
    #[must_use]
    pub fn note(&self) -> String {
        let since = self.recorded_since.as_deref().unwrap_or_default();
        match self.standing {
            Standing::Passing => "passing".to_owned(),
            Standing::Fixed => format!("fixed; on record as failing since {since}"),
            Standing::StillFailing => format!("on record as failing since {since}"),
            Standing::Unrecorded => "failing, and not on record as failing".to_owned(),
            Standing::Regressed => format!("regressed; on record as passing since {since}"),
        }
    }
}

/// The run a record entry is stamped with.
#[derive(Debug, Clone, Copy)]
pub struct RunStamp<'a> {
    /// When the run was generated.
    pub at: &'a str,
    /// The engine it scored.
    pub engine: &'a str,
}

/// Reads the record under `root`; an absent file is an empty record.
///
/// # Errors
///
/// Returns an error when the file exists but cannot be read as a record — a
/// malformed record must fail the run rather than tolerate nothing silently.
pub fn load(root: &Path) -> Result<Ledger> {
    let path = root.join(OUTCOMES_PATH);
    if !path.exists() {
        return Ok(Ledger::default());
    }
    read_json(&path)
}

/// Every judged pair `engine_id` was scored on, in target order, with how it
/// stands against `ledger`.
#[must_use]
pub fn standings(ledger: &Ledger, engine_id: &str, targets: &[TargetScore]) -> Vec<PairOutcome> {
    let recorded: BTreeMap<&PairKey, &RecordedPair> =
        ledger.pairs.iter().map(|pair| (&pair.key, pair)).collect();
    targets
        .iter()
        .filter_map(|target| Some((target.name.as_str(), target.scores.get(engine_id)?)))
        .flat_map(|(repo, score)| score.entries.iter().map(move |entry| (repo, entry)))
        .map(|(repo, entry)| standing(&recorded, PairKey::of(repo, entry), Outcome::of(entry)))
        .collect()
}

/// How one pair's `outcome` stands against its record, if it has one.
fn standing(
    recorded: &BTreeMap<&PairKey, &RecordedPair>,
    key: PairKey,
    outcome: Outcome,
) -> PairOutcome {
    let record = recorded.get(&key);
    PairOutcome {
        standing: Standing::of(outcome, record.map(|pair| pair.outcome)),
        recorded_since: record.map(|pair| pair.since.clone()),
        outcome,
        key,
    }
}

/// The record after this run: fixed pairs and newly judged passing pairs are
/// recorded as passing, unrecorded failures only when `accept_failures`, and
/// a pair a scored repository's register no longer judges is dropped. A
/// regression is never recorded over the pass it broke, and a repository this
/// run did not score keeps its record untouched.
#[must_use]
pub fn record(
    ledger: &Ledger,
    outcomes: &[PairOutcome],
    stamp: RunStamp<'_>,
    accept_failures: bool,
) -> Ledger {
    let mut pairs = still_judged(ledger, outcomes);
    let written: Vec<RecordedPair> = outcomes
        .iter()
        .filter(|pair| goes_on_record(pair, pairs.contains_key(&pair.key), accept_failures))
        .map(|pair| recorded(pair, stamp))
        .collect();
    pairs.extend(written.into_iter().map(|pair| (pair.key.clone(), pair)));
    Ledger {
        spec: OUTCOMES_SPEC.to_owned(),
        pairs: pairs.into_values().collect(),
    }
}

/// Whether this run writes `pair` to the record: a fix always, a passing pair
/// only when it is new, an unrecorded failure only when accepted, and a
/// regression or a still-failing pair never.
fn goes_on_record(pair: &PairOutcome, on_record: bool, accept_failures: bool) -> bool {
    match pair.standing {
        Standing::Passing => !on_record,
        Standing::Fixed => true,
        Standing::Unrecorded => accept_failures,
        Standing::StillFailing | Standing::Regressed => false,
    }
}

/// Every recorded pair except those of a scored repository whose register no
/// longer judges them, keyed so the record is written in one stable order.
fn still_judged(ledger: &Ledger, outcomes: &[PairOutcome]) -> BTreeMap<PairKey, RecordedPair> {
    let scored: BTreeSet<&str> = outcomes.iter().map(|pair| pair.key.repo.as_str()).collect();
    let judged: BTreeSet<&PairKey> = outcomes.iter().map(|pair| &pair.key).collect();
    ledger
        .pairs
        .iter()
        .filter(|pair| !scored.contains(pair.key.repo.as_str()) || judged.contains(&pair.key))
        .map(|pair| (pair.key.clone(), pair.clone()))
        .collect()
}

/// `pair`'s outcome in this run, stamped with the run.
fn recorded(pair: &PairOutcome, stamp: RunStamp<'_>) -> RecordedPair {
    RecordedPair {
        key: pair.key.clone(),
        outcome: pair.outcome,
        since: stamp.at.to_owned(),
        engine: stamp.engine.to_owned(),
    }
}

/// Writes the record under `root`.
///
/// # Errors
///
/// Returns an error when the file cannot be written.
pub fn write(root: &Path, ledger: &Ledger) -> Result<()> {
    let path = root.join(OUTCOMES_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(ledger)? + "\n")
        .with_context(|| format!("cannot write {}", path.display()))
}
