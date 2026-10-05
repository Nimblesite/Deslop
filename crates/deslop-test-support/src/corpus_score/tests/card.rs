//! [CORPUS-SCORE-RENDER] [CORPUS-REPORT-VERDICT] The two-engine scorecard the
//! rendering and verdict suites read.
//!
//! The card is gated and judged the way a real run gates and judges one
//! (`corpus_score::run`): the last engine's breaches come from its own score
//! against the gate the card states, and the verdict from those breaches and
//! its curated checks. Neither is ever set by hand — a fixture that could head
//! a breached gate with a pass would have every suite that reads it asserting
//! against a document no run can write.

use std::collections::BTreeMap;

use super::{
    super::{
        checks::CheckOutcome,
        gate::{add_checks, CorpusTotals},
        verdict::verdict,
    },
    add_costs, breaches, corpus_change, degradation, pinned_sha, totals, Engine, RepoScore,
    RunCost, Scorecard, TargetScore, Thresholds, REPO,
};

pub(super) const OLD_ENGINE: &str = "old";
pub(super) const NEW_ENGINE: &str = "new";
/// Measured costs the rendering tests assert against.
pub(super) const OLD_ELAPSED_MS: u64 = 2000;
pub(super) const NEW_ELAPSED_MS: u64 = 2500;
const OLD_PEAK_MB: u64 = 100;
pub(super) const NEW_PEAK_MB: u64 = 140;
pub(super) const CPU_SECONDS: f64 = 1.5;
/// Each engine's busiest sampling window, in percent of one core.
const OLD_PEAK_CPU: f64 = 250.0;
const NEW_PEAK_CPU: f64 = 380.0;
/// Clusters each engine's report published.
const CLUSTERS_PUBLISHED: usize = 1;
/// Engine labels, and the language the fixture repository is filed under.
pub(super) const OLD_LABEL: &str = "engine-old";
pub(super) const NEW_LABEL: &str = "engine-new";
pub(super) const LANGUAGE: &str = "Rust";
/// The card scans one repository.
pub(super) const ONE_REPO: usize = 1;
/// When the fixture run was scored, and the binary its costs are stamped with.
const GENERATED_AT: &str = "2026-09-04T00:00:00Z";
const BINARY_SHA: &str = "deadbeef";

/// A measured cost for the rendering tests.
fn cost(elapsed_ms: u64, peak_rss_mb: u64, peak_cpu_percent: f64) -> RunCost {
    RunCost {
        elapsed_ms,
        peak_rss_mb: Some(peak_rss_mb),
        cpu_seconds: Some(CPU_SECONDS),
        peak_cpu_percent: Some(peak_cpu_percent),
        binary_sha256: BINARY_SHA.to_owned(),
    }
}

/// The old engine's measured cost.
pub(super) fn old_cost() -> RunCost {
    cost(OLD_ELAPSED_MS, OLD_PEAK_MB, OLD_PEAK_CPU)
}

/// The new engine's measured cost.
pub(super) fn new_cost() -> RunCost {
    cost(NEW_ELAPSED_MS, NEW_PEAK_MB, NEW_PEAK_CPU)
}

/// The engines compared, in run order.
fn engines() -> Vec<Engine> {
    [(OLD_ENGINE, OLD_LABEL), (NEW_ENGINE, NEW_LABEL)]
        .map(|(id, label)| Engine {
            id: id.to_owned(),
            label: label.to_owned(),
        })
        .into()
}

/// One value per engine, keyed by engine id.
fn per_engine<T>(old: T, new: T) -> BTreeMap<String, T> {
    BTreeMap::from([(OLD_ENGINE.to_owned(), old), (NEW_ENGINE.to_owned(), new)])
}

/// The fixture repository as both engines ran it.
fn target(
    before: &RepoScore,
    after: &RepoScore,
    checks: BTreeMap<String, CheckOutcome>,
) -> TargetScore {
    TargetScore {
        name: REPO.to_owned(),
        language: LANGUAGE.to_owned(),
        sha: pinned_sha(),
        registered: true,
        scores: per_engine(before.clone(), after.clone()),
        clusters: per_engine(CLUSTERS_PUBLISHED, CLUSTERS_PUBLISHED),
        costs: per_engine(old_cost(), new_cost()),
        checks,
        degradation: Some(degradation(before, after)),
    }
}

/// One engine's corpus standing over the fixture repository: its score, its
/// measured cost and the curated checks it evaluated.
fn standing(score: &RepoScore, measured: RunCost, checks: Option<&CheckOutcome>) -> CorpusTotals {
    let mut summed = totals(std::slice::from_ref(score));
    let costs = BTreeMap::from([(REPO.to_owned(), measured)]);
    add_costs(&mut summed, &costs, ONE_REPO);
    add_checks(&mut summed, &checks.into_iter().collect::<Vec<_>>());
    summed
}

/// A two-engine scorecard over one repository no corpus test checked.
pub(super) fn card(before: &RepoScore, after: &RepoScore) -> Scorecard {
    checked_card(before, after, BTreeMap::new())
}

/// The same scorecard with the curated checks a corpus test filed, keyed by
/// the engine that evaluated them.
pub(super) fn checked_card(
    before: &RepoScore,
    after: &RepoScore,
    checks: BTreeMap<String, CheckOutcome>,
) -> Scorecard {
    let before_totals = standing(before, old_cost(), checks.get(OLD_ENGINE));
    let after_totals = standing(after, new_cost(), checks.get(NEW_ENGINE));
    let engines = engines();
    let targets = vec![target(before, after, checks)];
    let gate = Thresholds::default();
    let breached = breaches(after, &gate);
    Scorecard {
        generated_at: GENERATED_AT.to_owned(),
        verdict: verdict(&engines, &targets, &breached),
        engines,
        targets,
        change: Some(corpus_change(&before_totals, &after_totals)),
        totals: per_engine(before_totals, after_totals),
        thresholds: BTreeMap::from([(REPO.to_owned(), gate)]),
        breaches: breached,
    }
}
