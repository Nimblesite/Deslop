//! [CORPUS-HISTORY] The run history: one record per corpus run in
//! `corpus/history/runs/`, carrying every judged pair's outcome, each
//! repository's score and cost, the machine it ran on and the commit — and
//! release version — it was built from, with `corpus/history/HISTORY.md`
//! rendered from every record so accuracy, CPU and memory read run over run.
//!
//! The history also gates. A judged pair that passed in any earlier recorded
//! run of the same commit and fails now is a **backslide**, and one backslide
//! fails the run. There is no allowance, and no way to accept one: unlike the
//! judged pair record ([CORPUS-SCORE-OUTCOMES]), the history is append-only,
//! so a pass, once recorded, holds every later run to it.

pub mod store;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, System};

use super::{
    gate::CorpusTotals,
    outcomes::{Outcome, PairKey, Standing},
    render::{Scorecard, TargetScore},
    source::Source,
    verdict::FailureReason,
    RunCost,
};

/// What a backslide is called in the verdict.
pub const BACKSLIDE: &str = "backslide";
/// Bytes in a mebibyte, for the machine's memory.
const BYTES_PER_MB: u64 = 1024 * 1024;
/// What a machine fact the platform would not report is recorded as.
const UNKNOWN: &str = "unknown";

/// The machine a run measured its cost on. A CPU or memory figure is only
/// comparable with one taken on the same machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    /// Operating system name.
    pub os: String,
    /// Operating system version.
    pub os_version: String,
    /// CPU architecture.
    pub arch: String,
    /// CPU model.
    pub cpu_model: String,
    /// Logical CPUs.
    pub logical_cpus: usize,
    /// Physical cores, absent when the platform would not say.
    pub physical_cores: Option<usize>,
    /// Physical memory in mebibytes.
    pub memory_mb: u64,
}

impl Host {
    /// The machine this process runs on.
    #[must_use]
    pub fn this_machine() -> Self {
        let mut system = System::new();
        system.refresh_cpu_list(CpuRefreshKind::nothing());
        system.refresh_memory();
        Self {
            os: System::name().unwrap_or_else(|| UNKNOWN.to_owned()),
            os_version: System::os_version().unwrap_or_else(|| UNKNOWN.to_owned()),
            arch: System::cpu_arch(),
            cpu_model: system
                .cpus()
                .first()
                .map_or_else(|| UNKNOWN.to_owned(), |cpu| cpu.brand().trim().to_owned()),
            logical_cpus: system.cpus().len(),
            physical_cores: System::physical_core_count(),
            memory_mb: system.total_memory() / BYTES_PER_MB,
        }
    }
}

/// One repository in one run: whether it matched its judgments, every count
/// the score is made of, and what the scan cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRecord {
    /// Repository name.
    pub name: String,
    /// Its language.
    pub language: String,
    /// The commit scanned, which is the commit its register was judged at.
    pub sha: String,
    /// True when every judged pair was answered correctly.
    pub matches_judgments: bool,
    /// Judged pairs, and how many were answered correctly.
    pub judged: usize,
    /// See [`Self::judged`].
    pub correct: usize,
    /// CLEARLY IN pairs, and how many were reported.
    pub clearly_in_total: usize,
    /// See [`Self::clearly_in_total`].
    pub clearly_in_found: usize,
    /// CLEARLY OUT pairs, and how many were correctly left unreported.
    pub clearly_out_total: usize,
    /// See [`Self::clearly_out_total`].
    pub clearly_out_absent: usize,
    /// Wrong answers in each direction.
    pub false_negatives: usize,
    /// See [`Self::false_negatives`].
    pub false_positives: usize,
    /// `100 * correct / judged`, as the scorecard computed it.
    pub score_percent: Option<f64>,
    /// Clusters the report published.
    pub clusters: Option<usize>,
    /// Wall time, CPU time, peak CPU and peak memory of the scan.
    pub cost: Option<RunCost>,
}

/// One judged pair's outcome in one run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Judgment {
    /// Which pair.
    #[serde(flatten)]
    pub key: PairKey,
    /// Whether it was answered correctly.
    pub outcome: Outcome,
    /// How that stood against the judged pair record.
    pub standing: Standing,
}

/// A judged pair that passed in an earlier run of the same commit and fails
/// now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backslide {
    /// Which pair.
    #[serde(flatten)]
    pub key: PairKey,
    /// When the latest earlier run it passed in was generated.
    pub passed_at: String,
}

/// One line of the history: everything one run measured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    /// When the run was generated.
    pub generated_at: String,
    /// The scorecard it wrote, by file stem.
    pub scorecard: String,
    /// The engine scored.
    pub engine: String,
    /// The source it was built from.
    pub source: Source,
    /// The machine it ran on.
    pub host: Host,
    /// True when nothing that passed before fails now.
    pub passed: bool,
    /// The run's standing across every repository it scored: judged and
    /// correct pairs, the score, and summed wall and CPU time with the peaks.
    pub totals: CorpusTotals,
    /// Judged pairs that failed the run: on record as passing, or passing in
    /// an earlier run of the history, and failing now.
    pub regressions: usize,
    /// Judged pairs failing that never passed — known defects, not
    /// regressions.
    pub known_defects: usize,
    /// Every backslide this run found.
    pub backslides: Vec<Backslide>,
    /// Every repository scored.
    pub repos: Vec<RepoRecord>,
    /// Every judged pair scored.
    pub judgments: Vec<Judgment>,
}

/// The run history as the scorecard shows it.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryCheck {
    /// Where each run's record is kept.
    pub runs: String,
    /// The history document rendered from every record.
    pub log: String,
    /// Runs already in the history when this one was checked.
    pub earlier_runs: usize,
    /// The source this run was built from.
    pub source: Source,
    /// The machine it ran on.
    pub host: Host,
    /// Every repository scored, and whether it matched its judgments.
    pub repos: Vec<RepoRecord>,
    /// Every judged pair that went backwards.
    pub backslides: Vec<Backslide>,
}

/// Holds `card` to every earlier run in the history under `root`: records the
/// last engine's source and the machine on it, and fails its verdict on every
/// backslide.
///
/// # Errors
///
/// Returns an error when the history cannot be read.
pub fn hold(card: &mut Scorecard, root: &Path) -> Result<()> {
    let source = card
        .engines
        .last()
        .and_then(|engine| engine.source.clone())
        .ok_or_else(|| anyhow!("a run held to the history must name the commit it was built from"))?;
    let earlier = store::load(root)?;
    let repos = repo_records(card);
    let backslides = backslides(&earlier, &repos, &judgments(card));
    card.verdict
        .failures
        .extend(backslides.iter().map(backslide_failure));
    card.history = Some(HistoryCheck {
        runs: store::RUNS_DIR.to_owned(),
        log: store::LOG_PATH.to_owned(),
        earlier_runs: earlier.len(),
        source,
        host: Host::this_machine(),
        repos,
        backslides,
    });
    Ok(())
}

/// The history line for a scorecard [`hold`] checked, written under `stem`;
/// absent when the run was not held to the history.
#[must_use]
pub fn record(card: &Scorecard, stem: &str) -> Option<RunRecord> {
    let history = card.history.as_ref()?;
    let engine = card.engines.last()?;
    let judgments = judgments(card);
    let regressions = regressions(&judgments, &history.backslides);
    let failing = judgments
        .iter()
        .filter(|judgment| judgment.outcome == Outcome::Fail)
        .count();
    Some(RunRecord {
        generated_at: card.generated_at.clone(),
        scorecard: stem.to_owned(),
        engine: engine.label.clone(),
        source: history.source.clone(),
        host: history.host.clone(),
        passed: kept(card),
        totals: card.totals.get(&engine.id).cloned().unwrap_or_default(),
        regressions,
        known_defects: failing.saturating_sub(regressions),
        backslides: history.backslides.clone(),
        repos: history.repos.clone(),
        judgments,
    })
}

/// Distinct judged pairs that failed the run: regressed against the record,
/// or backslid against the history — a pair doing both is counted once.
fn regressions(judgments: &[Judgment], backslides: &[Backslide]) -> usize {
    let regressed = judgments
        .iter()
        .filter(|judgment| judgment.standing.fails_the_run())
        .map(|judgment| &judgment.key);
    regressed
        .chain(backslides.iter().map(|backslide| &backslide.key))
        .collect::<BTreeSet<_>>()
        .len()
}

/// Whether a run held to the history kept to it: no pair on record as
/// passing fails now ([CORPUS-SCORE-OUTCOMES]), and no pair that passed in an
/// earlier run fails now.
#[must_use]
pub fn kept(card: &Scorecard) -> bool {
    let regressed = card
        .outcomes
        .iter()
        .any(|pair| pair.standing.fails_the_run());
    !regressed && backslid(card) == 0
}

/// How many judged pairs went backwards against the run history; none when
/// the run was not held to it.
#[must_use]
pub fn backslid(card: &Scorecard) -> usize {
    card.history
        .as_ref()
        .map_or(0, |history| history.backslides.len())
}

/// Every judged pair the last engine was scored on.
fn judgments(card: &Scorecard) -> Vec<Judgment> {
    card.outcomes
        .iter()
        .map(|pair| Judgment {
            key: pair.key.clone(),
            outcome: pair.outcome,
            standing: pair.standing,
        })
        .collect()
}

/// Every repository the last engine was scored on against a register.
fn repo_records(card: &Scorecard) -> Vec<RepoRecord> {
    let Some(engine) = card.engines.last() else {
        return Vec::new();
    };
    card.targets
        .iter()
        .filter_map(|target| repo_record(target, &engine.id))
        .collect()
}

/// One target's record for `engine_id`, absent when it was not scored.
fn repo_record(target: &TargetScore, engine_id: &str) -> Option<RepoRecord> {
    let score = target.scores.get(engine_id)?;
    Some(RepoRecord {
        name: target.name.clone(),
        language: target.language.clone(),
        sha: target.sha.clone(),
        matches_judgments: score.false_negatives == 0 && score.false_positives == 0,
        judged: score.judged,
        correct: score.correct,
        clearly_in_total: score.clearly_in_total,
        clearly_in_found: score.clearly_in_found,
        clearly_out_total: score.clearly_out_total,
        clearly_out_absent: score.clearly_out_absent,
        false_negatives: score.false_negatives,
        false_positives: score.false_positives,
        score_percent: score.score_percent,
        clusters: target.clusters.get(engine_id).copied(),
        cost: target.costs.get(engine_id).cloned(),
    })
}

/// A pair at the commit it was judged at: a pass at one commit says nothing
/// about the same ranges at another.
type PinnedPair = (String, PairKey);

/// Every failing pair in `now` that passed in an earlier run of the same
/// commit, with the latest earlier run it passed in.
fn backslides(earlier: &[RunRecord], repos: &[RepoRecord], now: &[Judgment]) -> Vec<Backslide> {
    let passed = passes(earlier);
    now.iter()
        .filter(|judgment| judgment.outcome == Outcome::Fail)
        .filter_map(|judgment| {
            let pinned = (
                scanned_sha(repos, &judgment.key.repo)?,
                judgment.key.clone(),
            );
            Some(Backslide {
                key: judgment.key.clone(),
                passed_at: passed.get(&pinned)?.clone(),
            })
        })
        .collect()
}

/// Every pair that passed in an earlier run, with the latest run it passed
/// in. The history is in run order, so a later pass replaces an earlier one.
fn passes(earlier: &[RunRecord]) -> BTreeMap<PinnedPair, String> {
    earlier
        .iter()
        .flat_map(|run| {
            run.judgments
                .iter()
                .filter(|judgment| judgment.outcome == Outcome::Pass)
                .filter_map(move |judgment| {
                    let sha = scanned_sha(&run.repos, &judgment.key.repo)?;
                    Some(((sha, judgment.key.clone()), run.generated_at.clone()))
                })
        })
        .collect()
}

/// The commit `repo` was scanned at, matched as the record keys it — in
/// lower case.
fn scanned_sha(repos: &[RepoRecord], repo: &str) -> Option<String> {
    repos
        .iter()
        .find(|record| record.name.to_lowercase() == repo)
        .map(|record| record.sha.clone())
}

/// One backslide, as the verdict lists it.
fn backslide_failure(backslide: &Backslide) -> FailureReason {
    FailureReason {
        repo: backslide.key.repo.clone(),
        what: format!("{BACKSLIDE}: {}", backslide.key.defect()),
        detail: format!(
            "`{}`: passed in the run of {}, fails now",
            backslide.key.occurrences.join("` + `"),
            backslide.passed_at
        ),
        tracked_in: None,
        regression: true,
    }
}
