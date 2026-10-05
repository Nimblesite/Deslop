//! [CORPUS-REPORT] Turns a run manifest into the corpus scorecard.
//!
//! Every corpus run ends here — `score-gate.sh`, `compare-versions.sh` through
//! the `corpus-score` binary, and every `corpus_*` test through
//! [`crate::corpus_record`] — so every one of them writes the same markdown
//! and JSON scorecard, with accuracy, wall time, CPU and peak memory in the same
//! tables. There is no second renderer for any of them to drift into.
//!
//! A run manifest names the engines and, per target, each engine's report,
//! timing and curated-check outcome. A target with a clone register is scored
//! against it; a target without one is still reported — its cost and its
//! curated checks are evidence too, and dropping it would let a partial run
//! read as a smaller, cleaner one.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{anyhow, Result};
use serde_json::Value;

use super::{
    checks::CheckOutcome,
    clusters,
    gate::{
        add_checks, add_costs, breaches, corpus_change, degradation, load_thresholds, totals,
        Breach, CorpusChange, CorpusTotals, Degradation, Thresholds,
    },
    list,
    render::{scorecard, Engine, Scorecard, TargetScore},
    score_repo, text,
    verdict::verdict,
    RepoScore, RunCost,
};
use crate::read_json;

/// A degradation verdict needs exactly two engines to compare.
const COMPARED_ENGINES: usize = 2;
/// The scorecard a human reads, and the machine-readable one beside it.
pub const MARKDOWN_EXTENSION: &str = "md";
/// See [`MARKDOWN_EXTENSION`].
pub const JSON_EXTENSION: &str = "json";
/// What a timestamp's time-of-day separator becomes in a file name: Windows
/// does not allow `:` in one.
const FILE_NAME_TIME_SEPARATOR: &str = "-";

/// A written scorecard: what it says, and where it went.
#[derive(Debug)]
pub struct WrittenScorecard {
    /// Every figure, as the JSON scorecard records it.
    pub card: Scorecard,
    /// The rendered markdown.
    pub markdown: String,
    /// The path of the markdown scorecard.
    pub path: PathBuf,
    /// The path of the JSON scorecard beside it.
    pub json_path: PathBuf,
}

/// The file stem a run's scorecard is written under: what ran, then when, in
/// UTC — `corpus_flutter_dart-2026-09-27T03-35-02Z`. Every run writes its own
/// pair of files, so no run ever overwrites another's, and the names sort in
/// the order the runs happened.
#[must_use]
pub fn report_stem(run_name: &str, moment: SystemTime) -> String {
    let stamp = humantime::format_rfc3339_seconds(moment)
        .to_string()
        .replace(':', FILE_NAME_TIME_SEPARATOR);
    format!("{run_name}-{stamp}")
}

/// A path field resolved against `root`, absent when missing or blank.
fn path_field(value: &Value, field: &str, root: &Path) -> Option<PathBuf> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .map(|path| root.join(path))
}

/// The register for one target, refused outright when it was judged at a
/// different commit than the one scanned.
fn register_for(target: &Value, root: &Path) -> Result<Option<Value>> {
    let Some(path) = path_field(target, "register", root).filter(|path| path.exists()) else {
        return Ok(None);
    };
    let register = read_json(&path)?;
    let judged = text(&register, "sha");
    let scanned = text(target, "sha");
    if judged != scanned {
        return Err(anyhow!(
            "{} is judged at {judged} but {} was scanned at {scanned} — re-judge the register \
             at the scanned commit rather than scoring it against different source",
            path.display(),
            text(target, "name")
        ));
    }
    Ok(Some(register))
}

/// [CORPUS-SCORE-COST-COMPLETE] A strict gate needs each judged run's timing.
fn run_cost(
    timing: Option<&Path>,
    engine_id: &str,
    name: &str,
    require_timing: bool,
) -> Result<Option<RunCost>> {
    if let Some(path) = timing.filter(|path| path.exists()) {
        return read_json(path).map(Some);
    }
    if require_timing {
        return Err(anyhow!(
            "missing timing for judged run {engine_id} in {name}: {}",
            timing.map_or_else(
                || "<not specified>".to_owned(),
                |path| path.display().to_string()
            )
        ));
    }
    Ok(None)
}

/// The run's report, absent when the scan crashed before writing one. A
/// strict gate refuses a judged run with no report: an unscored register must
/// never read as a register nobody breached.
fn run_report(
    run: &Value,
    register: Option<&Value>,
    context: RunContext<'_>,
) -> Result<Option<Value>> {
    let Some(path) = path_field(run, "report", context.root) else {
        if context.require_timing && register.is_some() {
            return Err(anyhow!(
                "judged run {} in {} has no report to score",
                context.engine_id,
                context.name
            ));
        }
        return Ok(None);
    };
    read_json(&path).map(Some)
}

/// What one engine's run of one target produced.
#[derive(Debug)]
struct EngineRun {
    /// Clusters the report published, absent when there is no report.
    clusters: Option<usize>,
    /// The register score, when the target has a register.
    score: Option<RepoScore>,
    /// The measured cost, when a timing file was written.
    cost: Option<RunCost>,
    /// The curated checks, when a corpus test evaluated them.
    checks: Option<CheckOutcome>,
}

/// Where one engine run is, and what it is held to.
#[derive(Debug, Clone, Copy)]
struct RunContext<'a> {
    /// The repository root every manifest path is relative to.
    root: &'a Path,
    /// The engine that ran.
    engine_id: &'a str,
    /// The target it ran on.
    name: &'a str,
    /// Whether a missing timing file fails the run.
    require_timing: bool,
}

/// Scores and reads one engine run.
fn engine_run(run: &Value, register: Option<&Value>, context: RunContext<'_>) -> Result<EngineRun> {
    let report = run_report(run, register, context)?;
    let score = register
        .zip(report.as_ref())
        .map(|(register, report)| score_repo(context.name, register, report))
        .transpose()?;
    let timing = path_field(run, "timing", context.root);
    let judged = context.require_timing && register.is_some();
    let cost = run_cost(timing.as_deref(), context.engine_id, context.name, judged)?;
    let checks = path_field(run, "checks", context.root)
        .map(|path| read_json(&path))
        .transpose()?;
    Ok(EngineRun {
        clusters: report.as_ref().map(|report| clusters(report).len()),
        score,
        cost,
        checks,
    })
}

/// Every engine's run of one target, each map keyed by engine id.
#[derive(Debug, Default)]
struct EngineRuns {
    /// Clusters published, for every engine that ran.
    clusters: BTreeMap<String, usize>,
    /// Register scores, for every engine scored against a register.
    scores: BTreeMap<String, RepoScore>,
    /// Measured costs, for every engine that left a timing file.
    costs: BTreeMap<String, RunCost>,
    /// Curated check outcomes, for every engine a corpus test ran.
    checks: BTreeMap<String, CheckOutcome>,
}

impl EngineRuns {
    /// Files one engine's run under its id.
    fn insert(&mut self, engine_id: &str, run: EngineRun) {
        let id = engine_id.to_owned();
        let _previous = run
            .clusters
            .map(|count| self.clusters.insert(id.clone(), count));
        let _previous = run.score.map(|score| self.scores.insert(id.clone(), score));
        let _previous = run.cost.map(|cost| self.costs.insert(id.clone(), cost));
        let _previous = run.checks.map(|outcome| self.checks.insert(id, outcome));
    }
}

/// Reads every engine run of one target.
fn engine_runs(
    target: &Value,
    register: Option<&Value>,
    root: &Path,
    require_timing: bool,
) -> Result<EngineRuns> {
    let mut runs_by_engine = EngineRuns::default();
    let name = text(target, "name");
    let runs = target.get("runs").and_then(Value::as_object);
    for (engine_id, run) in runs.into_iter().flatten() {
        let context = RunContext {
            root,
            engine_id,
            name: &name,
            require_timing,
        };
        runs_by_engine.insert(engine_id, engine_run(run, register, context)?);
    }
    Ok(runs_by_engine)
}

/// Compare the two scored engines in manifest order, if both exist.
fn compared_scores(
    scores: &BTreeMap<String, RepoScore>,
    engine_order: &[String],
) -> Option<Degradation> {
    if scores.len() != COMPARED_ENGINES {
        return None;
    }
    let ordered: Vec<&RepoScore> = engine_order
        .iter()
        .filter_map(|engine| scores.get(engine))
        .collect();
    match ordered.as_slice() {
        [before, after] => Some(degradation(before, after)),
        _ => None,
    }
}

/// Scores one target across every engine that ran it.
fn score_target(
    target: &Value,
    root: &Path,
    engine_order: &[String],
    require_timing: bool,
) -> Result<TargetScore> {
    let register = register_for(target, root)?;
    let runs = engine_runs(target, register.as_ref(), root, require_timing)?;
    Ok(TargetScore {
        name: text(target, "name"),
        language: text(target, "language"),
        sha: text(target, "sha"),
        registered: register.is_some(),
        degradation: compared_scores(&runs.scores, engine_order),
        scores: runs.scores,
        clusters: runs.clusters,
        costs: runs.costs,
        checks: runs.checks,
    })
}

/// The engines a run manifest describes, in run order.
fn engines(run: &Value) -> Vec<Engine> {
    list(run, "engines")
        .iter()
        .map(|engine| Engine {
            id: text(engine, "id"),
            label: text(engine, "label"),
        })
        .collect()
}

/// Every target in the manifest, registered or not, in run order.
fn score_targets(
    run: &Value,
    root: &Path,
    engine_order: &[String],
    require_timing: bool,
) -> Result<Vec<TargetScore>> {
    list(run, "targets")
        .iter()
        .map(|target| score_target(target, root, engine_order, require_timing))
        .collect()
}

/// One engine's corpus standing: accuracy over the judged targets, cost and
/// curated checks over every target it ran.
fn engine_totals(engine: &Engine, targets: &[TargetScore]) -> CorpusTotals {
    let id = engine.id.as_str();
    let ran: Vec<&TargetScore> = targets.iter().filter(|target| target.ran(id)).collect();
    let scores: Vec<RepoScore> = ran
        .iter()
        .filter_map(|target| target.scores.get(id).cloned())
        .collect();
    let checks: Vec<&CheckOutcome> = ran
        .iter()
        .filter_map(|target| target.checks.get(id))
        .collect();
    let mut summed = totals(&scores);
    summed.clusters_total = ran
        .iter()
        .map(|target| target.clusters.get(id).copied())
        .sum();
    add_costs(&mut summed, &engine_costs(id, &ran), ran.len());
    add_checks(&mut summed, &checks);
    summed
}

/// One engine's measured cost per repository it ran.
fn engine_costs(engine_id: &str, ran: &[&TargetScore]) -> BTreeMap<String, RunCost> {
    ran.iter()
        .filter_map(|target| {
            let cost = target.costs.get(engine_id)?;
            Some((target.name.clone(), cost.clone()))
        })
        .collect()
}

/// The gate the last engine is held to, and everything it breached. Only a
/// judged repository has a gate: there is nothing to breach without a register.
fn gate_last_engine(
    engines: &[Engine],
    targets: &[TargetScore],
    config: &Value,
) -> (BTreeMap<String, Thresholds>, Vec<Breach>) {
    let mut thresholds = BTreeMap::new();
    let mut found = Vec::new();
    let Some(last) = engines.last() else {
        return (thresholds, found);
    };
    for target in targets.iter().filter(|target| target.registered) {
        let gate = Thresholds::for_repo(config, &target.name);
        if let Some(score) = target.scores.get(&last.id) {
            found.extend(breaches(score, &gate));
        }
        let _previous = thresholds.insert(target.name.clone(), gate);
    }
    (thresholds, found)
}

/// How the last engine's corpus standing moved against the first. Absent unless
/// exactly two engines ran: there is no "change" to state against yourself.
fn standing_change(
    engines: &[Engine],
    totals: &BTreeMap<String, CorpusTotals>,
) -> Option<CorpusChange> {
    let [first, last] = engines else {
        return None;
    };
    Some(corpus_change(totals.get(&first.id)?, totals.get(&last.id)?))
}

/// Scores a whole run manifest into a scorecard.
fn build_scorecard(run_path: &Path, root: &Path, gate: bool) -> Result<Scorecard> {
    let run = read_json(run_path)?;
    let engines = engines(&run);
    let order: Vec<String> = engines.iter().map(|engine| engine.id.clone()).collect();
    let targets = score_targets(&run, root, &order, gate)?;
    let totals = engines
        .iter()
        .map(|engine| (engine.id.clone(), engine_totals(engine, &targets)))
        .collect();
    let (thresholds, breached) = gate_last_engine(&engines, &targets, &load_thresholds(root)?);
    let verdict = verdict(&engines, &targets, &breached);
    Ok(Scorecard {
        generated_at: text(&run, "generated_at"),
        change: standing_change(&engines, &totals),
        engines,
        targets,
        totals,
        thresholds,
        breaches: breached,
        verdict,
    })
}

/// Scores the run manifest at `run_path` and writes `<stem>.md` and
/// `<stem>.json` into `out` — `stem` from [`report_stem`]. With `gate`, a
/// judged run missing its timing is an error; whether the last engine
/// breached its gate is on the returned card, so the caller decides what a
/// breach does — after the documents exist.
///
/// # Errors
///
/// Returns an error when the manifest, a report, a register or a timing file
/// cannot be read, or the documents cannot be written.
pub fn write_scorecard(
    run_path: &Path,
    root: &Path,
    out: &Path,
    stem: &str,
    gate: bool,
) -> Result<WrittenScorecard> {
    let card = build_scorecard(run_path, root, gate)?;
    fs::create_dir_all(out)?;
    let markdown = scorecard(&card);
    let path = out.join(format!("{stem}.{MARKDOWN_EXTENSION}"));
    fs::write(&path, &markdown)?;
    let json_path = out.join(format!("{stem}.{JSON_EXTENSION}"));
    fs::write(&json_path, serde_json::to_string_pretty(&card)? + "\n")?;
    Ok(WrittenScorecard {
        card,
        markdown,
        path,
        json_path,
    })
}

#[cfg(test)]
mod tests;
