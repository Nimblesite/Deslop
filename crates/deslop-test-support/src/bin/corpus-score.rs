//! [CORPUS-SCORE] The corpus scorecard tool.
//!
//! Two jobs, kept in one binary because they share the measurement and the
//! arithmetic: `measure` runs a scan under this platform's peak-RSS
//! measurement and records what it cost, and `score` reads the resulting run
//! manifest, scores every report against its clone register, renders the
//! scorecard, and holds the last engine to the gate.
//!
//! This is a development binary (`publish = false`) and is deliberately built
//! from the working tree, never from a compared engine's source: the two
//! engines in a comparison must be scored by one identical scorer.

use std::{collections::BTreeMap, ffi::OsString, fs, path::PathBuf};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use deslop_test_support::{
    corpus::{measured_run, repo_root},
    corpus_score::{
        gate::{
            add_costs, breaches, corpus_change, degradation, load_thresholds, totals, CorpusChange,
            CorpusTotals, Degradation, Thresholds,
        },
        render::{scorecard, Engine, Scorecard, TargetScore},
        score_repo, RepoScore, RunCost,
    },
    read_json,
};
use serde_json::Value;

/// A degradation verdict needs exactly two engines to compare.
const COMPARED_ENGINES: usize = 2;

/// Command line for the corpus scorecard tool.
#[derive(Parser)]
#[command(name = "corpus-score", about = "Measure and score corpus scans")]
struct Cli {
    /// The job to run.
    #[command(subcommand)]
    command: Command,
}

/// The two jobs this binary does.
#[derive(Subcommand)]
enum Command {
    /// Run a command under peak-RSS measurement, recording what it cost.
    Measure {
        /// Where the cost is written.
        #[arg(long)]
        timing: PathBuf,
        /// The sha256 of the binary being measured, so a figure is traceable.
        #[arg(long)]
        binary_sha: String,
        /// The program to run, then its arguments.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command_line: Vec<OsString>,
    },
    /// Score every report in a run manifest against its clone register.
    Score {
        /// The run manifest describing engines, targets and report paths.
        run: PathBuf,
        /// Directory the scorecard is written to.
        #[arg(long)]
        out: PathBuf,
        /// Exit non-zero when the last engine breaches its gate.
        #[arg(long)]
        gate: bool,
    },
}

/// A string field, blank when absent.
fn text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Runs one measured command and records its cost.
fn measure(timing: &std::path::Path, binary_sha: &str, command_line: &[OsString]) -> Result<()> {
    let (program, args) = command_line
        .split_first()
        .ok_or_else(|| anyhow!("measure needs a program to run"))?;
    let run = measured_run(std::path::Path::new(program), args)?;
    if !run.output.status.success() {
        return Err(anyhow!(
            "measured command exited {:?}: {}",
            run.output.status.code(),
            String::from_utf8_lossy(&run.output.stderr)
                .lines()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    let cost = RunCost {
        elapsed_ms: u64::try_from(run.wall.as_millis()).unwrap_or(u64::MAX),
        peak_rss_mb: Some(run.peak_rss_mb),
        cpu_seconds: run.cpu_seconds,
        binary_sha256: binary_sha.to_owned(),
    };
    if let Some(parent) = timing.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(timing, serde_json::to_string_pretty(&cost)? + "\n")?;
    eprintln!(
        "==> measured: {} ms wall, {} MB peak, {} CPU s",
        cost.elapsed_ms,
        run.peak_rss_mb,
        run.cpu_seconds
            .map_or_else(|| "-".to_owned(), |secs| format!("{secs:.2}"))
    );
    Ok(())
}

/// The register for one target, refused outright when it was judged at a
/// different commit than the one scanned.
fn register_for(target: &Value, root: &std::path::Path) -> Result<Option<Value>> {
    let path = root.join(text(target, "register"));
    if !path.exists() {
        return Ok(None);
    }
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
    timing: Option<&std::path::Path>,
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

/// Score one engine run and read its measurement when present.
fn scored_run(
    run: &Value,
    register: &Value,
    root: &std::path::Path,
    engine_id: &str,
    name: &str,
    require_timing: bool,
) -> Result<(RepoScore, Option<RunCost>)> {
    let report = read_json(&root.join(text(run, "report")))?;
    let score = score_repo(name, register, &report)?;
    let timing = run
        .get("timing")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .map(|path| root.join(path));
    Ok((
        score,
        run_cost(timing.as_deref(), engine_id, name, require_timing)?,
    ))
}

/// Score and measure the engines present for one judged repository.
fn scored_runs(
    target: &Value,
    register: &Value,
    root: &std::path::Path,
    require_timing: bool,
) -> Result<(BTreeMap<String, RepoScore>, BTreeMap<String, RunCost>)> {
    let mut scores = BTreeMap::new();
    let mut costs = BTreeMap::new();
    let name = text(target, "name");
    let runs = target.get("runs").and_then(Value::as_object);
    for (engine_id, run) in runs.into_iter().flatten() {
        let (score, cost) = scored_run(run, register, root, engine_id, &name, require_timing)?;
        let _previous = scores.insert(engine_id.clone(), score);
        if let Some(cost) = cost {
            let _previous = costs.insert(engine_id.clone(), cost);
        }
    }
    Ok((scores, costs))
}

/// Compare the two scored engines in manifest order, if both exist.
fn compared_scores(
    scores: &BTreeMap<String, RepoScore>,
    engine_order: &[String],
) -> Option<Degradation> {
    (scores.len() == COMPARED_ENGINES)
        .then(|| {
            let ordered: Vec<&RepoScore> = engine_order
                .iter()
                .filter_map(|engine| scores.get(engine))
                .collect();
            match ordered.as_slice() {
                [before, after] => Some(degradation(before, after)),
                _ => None,
            }
        })
        .flatten()
}

/// Scores one target across every engine that ran it.
fn score_target(
    target: &Value,
    register: &Value,
    root: &std::path::Path,
    engine_order: &[String],
    require_timing: bool,
) -> Result<TargetScore> {
    let (scores, costs) = scored_runs(target, register, root, require_timing)?;
    Ok(TargetScore {
        name: text(target, "name"),
        language: text(target, "language"),
        sha: text(target, "sha"),
        degradation: compared_scores(&scores, engine_order),
        scores,
        costs,
    })
}

/// The engines a run manifest describes, in run order.
fn engines(run: &Value) -> Vec<Engine> {
    run.get("engines")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|engine| Engine {
                    id: text(engine, "id"),
                    label: text(engine, "label"),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Every target's score, skipping targets with no register judged yet.
fn score_targets(
    run: &Value,
    root: &std::path::Path,
    engine_order: &[String],
    require_timing: bool,
) -> Result<Vec<TargetScore>> {
    let mut scored = Vec::new();
    for target in run
        .get("targets")
        .and_then(Value::as_array)
        .unwrap_or(&vec![])
    {
        match register_for(target, root)? {
            Some(register) => scored.push(score_target(
                target,
                &register,
                root,
                engine_order,
                require_timing,
            )?),
            None => eprintln!("==> no register for {}, not scored", text(target, "name")),
        }
    }
    Ok(scored)
}

/// One engine's corpus standing, cost included.
fn engine_totals(
    engine: &Engine,
    targets: &[TargetScore],
) -> deslop_test_support::corpus_score::gate::CorpusTotals {
    let scores: Vec<RepoScore> = targets
        .iter()
        .filter_map(|target| target.scores.get(&engine.id).cloned())
        .collect();
    let costs: BTreeMap<String, RunCost> = targets
        .iter()
        .filter_map(|target| {
            target
                .costs
                .get(&engine.id)
                .map(|cost| (target.name.clone(), cost.clone()))
        })
        .collect();
    let mut summed = totals(&scores);
    add_costs(&mut summed, &costs);
    summed
}

/// The gate the last engine is held to, and everything it breached.
fn gate_last_engine(
    engines: &[Engine],
    targets: &[TargetScore],
    config: &Value,
) -> (
    BTreeMap<String, Thresholds>,
    Vec<deslop_test_support::corpus_score::gate::Breach>,
) {
    let mut thresholds = BTreeMap::new();
    let mut found = Vec::new();
    let Some(last) = engines.last() else {
        return (thresholds, found);
    };
    for target in targets {
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

/// Scores a whole run, writes the scorecard, and holds the last engine to the
/// gate when asked to.
fn score(run_path: &std::path::Path, out: &std::path::Path, gate: bool) -> Result<()> {
    let root = repo_root();
    let run = read_json(run_path)?;
    let engines = engines(&run);
    let order: Vec<String> = engines.iter().map(|engine| engine.id.clone()).collect();
    let targets = score_targets(&run, &root, &order, gate)?;
    let totals = engines
        .iter()
        .map(|engine| (engine.id.clone(), engine_totals(engine, &targets)))
        .collect();
    let (thresholds, breached) = gate_last_engine(&engines, &targets, &load_thresholds(&root)?);
    let card = Scorecard {
        generated_at: text(&run, "generated_at"),
        change: standing_change(&engines, &totals),
        engines,
        targets,
        totals,
        thresholds,
        breaches: breached,
    };

    fs::create_dir_all(out)?;
    let markdown = scorecard(&card);
    fs::write(out.join("SCORE.md"), &markdown)?;
    fs::write(
        out.join("score.json"),
        serde_json::to_string_pretty(&card)? + "\n",
    )?;
    println!("{markdown}");
    eprintln!("==> scorecard: {}", out.join("SCORE.md").display());
    if gate && !card.breaches.is_empty() {
        return Err(anyhow!(
            "{} threshold breach(es) — see the Gate section above",
            card.breaches.len()
        ));
    }
    Ok(())
}

/// Entry point.
fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Measure {
            timing,
            binary_sha,
            command_line,
        } => measure(&timing, &binary_sha, &command_line),
        Command::Score { run, out, gate } => score(&run, &out, gate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TEST_REPORT: &str = "target/.corpus/corpus-score-missing-timing/report.json";
    const TEST_TIMING: &str = "target/.corpus/corpus-score-missing-timing/absent-timing.json";
    const ENGINE_ID: &str = "current";

    /// [CORPUS-SCORE-COST-COMPLETE] Missing required cost fails the strict gate.
    #[test]
    fn registered_run_with_missing_timing_fails_scoring() -> Result<()> {
        let root = repo_root();
        let report = root.join(TEST_REPORT);
        fs::create_dir_all(root.join("target/.corpus/corpus-score-missing-timing"))?;
        fs::write(report, "{\"clusters\":[]}")?;
        let target = json!({"name":"fixture","runs":{ENGINE_ID:{"report":TEST_REPORT,"timing":TEST_TIMING}}});
        let register = json!({"clearly_in":[],"clearly_out":[]});
        let result = score_target(&target, &register, &root, &[ENGINE_ID.to_owned()], true);
        assert!(
            result.is_err(),
            "a missing timing file must never yield a green scorecard"
        );
        Ok(())
    }

    /// [CORPUS-SCORE-COST-COMPLETE] Accuracy-only scoring accepts an unmeasured run.
    #[test]
    fn unmeasured_run_keeps_accuracy_score_without_a_timing_field() -> Result<()> {
        let root = repo_root();
        let report = root.join(TEST_REPORT);
        fs::create_dir_all(root.join("target/.corpus/corpus-score-missing-timing"))?;
        fs::write(report, "{\"clusters\":[]}")?;
        let run = json!({"report":TEST_REPORT});
        let register = json!({"clearly_in":[],"clearly_out":[]});
        let result = scored_run(&run, &register, &root, ENGINE_ID, "fixture", false)?;
        assert_eq!(result.0.correct, 0, "empty register has no judged pairs");
        assert!(result.1.is_none(), "unmeasured run has no timing cost");
        Ok(())
    }
}
