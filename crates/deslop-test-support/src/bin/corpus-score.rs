//! [CORPUS-SCORE] The corpus scorecard tool.
//!
//! Two jobs, kept in one binary because they share the measurement and the
//! arithmetic: `measure` runs a scan under this platform's measurement and
//! records what it cost, and `score` reads the resulting run manifest, scores
//! every report against its clone register, renders the scorecard, writes the
//! judged pair record back, and holds the last engine to that record.
//!
//! Both jobs are thin: the measurement is [`deslop_test_support::corpus_measure`]
//! and the scorecard is [`deslop_test_support::corpus_score::run`], the same
//! code every `corpus_*` test writes its report through ([CORPUS-REPORT]).
//!
//! This is a development binary (`publish = false`) and is deliberately built
//! from the working tree, never from a compared engine's source: the two
//! engines in a comparison must be scored by one identical scorer.

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
use deslop_test_support::{
    corpus::repo_root,
    corpus_measure::{measured_run, ProcessCost},
    corpus_score::{
        history,
        render::Scorecard,
        run::{report_stem, write_scorecard, Scoring},
        RunCost,
    },
};

/// How many trailing stderr lines a failed measurement quotes.
const STDERR_LINES_QUOTED: usize = 3;

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
    /// Run a command under measurement, recording what it cost.
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
    Score(ScoreArgs),
}

/// What `score` reads, where it writes, and what it holds the run to.
#[derive(Args)]
struct ScoreArgs {
    /// The run manifest describing engines, targets and report paths.
    run: PathBuf,
    /// Directory the scorecard is written to.
    #[arg(long)]
    out: PathBuf,
    /// What ran — `score-gate`, `compare` — which, with the moment it ran,
    /// names the scorecard, so no run overwrites another's.
    #[arg(long)]
    name: String,
    /// Exit non-zero when a judged pair that passed before fails now
    /// ([CORPUS-SCORE-OUTCOMES], [CORPUS-HISTORY]).
    #[arg(long)]
    gate: bool,
    /// Put every failing pair that has no record on record as failing,
    /// accepting it as a known defect. Never moves a pass to a fail.
    #[arg(long, conflicts_with = "gate")]
    accept_failures: bool,
    /// What the run writes beyond its scorecard.
    #[command(flatten)]
    writes: Writes,
}

/// What a scored run writes beyond its scorecard.
#[derive(Args)]
struct Writes {
    /// Print only the JSON scorecard's path, for a caller that reads it.
    #[arg(long)]
    print_json_path: bool,
    /// [CORPUS-HISTORY] Hold the run to every earlier run in the run history —
    /// one judged pair that passed before and fails now fails the run — and
    /// add it to the history. The manifest's last engine must name its commit.
    #[arg(long, requires = "gate")]
    history: bool,
}

/// Runs one measured command and records its cost.
fn measure(timing: &Path, binary_sha: &str, command_line: &[OsString]) -> Result<()> {
    let (program, args) = command_line
        .split_first()
        .ok_or_else(|| anyhow!("measure needs a program to run"))?;
    let run = measured_run(Path::new(program), args)?;
    if !run.output.status.success() {
        return Err(exit_failure(&run.output));
    }
    let cost = RunCost::measured(&run.cost, binary_sha);
    if let Some(parent) = timing.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(timing, serde_json::to_string_pretty(&cost)? + "\n")?;
    print_measured(&run.cost);
    Ok(())
}

/// Describes a measured command that exited non-zero, quoting its last lines.
fn exit_failure(output: &std::process::Output) -> anyhow::Error {
    anyhow!(
        "measured command exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .rev()
            .take(STDERR_LINES_QUOTED)
            .collect::<Vec<_>>()
            .join(" | ")
    )
}

/// The progress line a measured scan leaves; the record is `timing.json`.
fn print_measured(cost: &ProcessCost) {
    eprintln!(
        "==> measured: {:.2} s wall, {:.2} s CPU, {} peak CPU, {} MB peak memory",
        cost.wall.as_secs_f64(),
        cost.cpu_seconds,
        cost.peak_cpu_percent
            .map_or_else(|| "-".to_owned(), |percent| format!("{percent:.0}%")),
        cost.peak_rss_mb,
    );
}

/// Scores a whole run, writes the scorecard and the record, and holds the last
/// engine to the record when asked to. The documents are written before the
/// record is applied, so a failing run still leaves a readable report.
fn score(args: &ScoreArgs) -> Result<()> {
    let scoring = Scoring {
        strict: args.gate,
        accept_failures: args.accept_failures,
        history: args.writes.history,
    };
    let stem = report_stem(&args.name, SystemTime::now());
    let written = write_scorecard(&args.run, &repo_root(), &args.out, &stem, scoring)?;
    if args.writes.print_json_path {
        println!("{}", written.json_path.display());
    } else {
        println!("{}", written.markdown);
    }
    eprintln!("==> scorecard: {}", written.path.display());
    held_to_the_record(&written.card, scoring)
}

/// Fails a strict run when a judged pair on record as passing fails now
/// ([CORPUS-SCORE-OUTCOMES]), and any run held to the history when a pair
/// that passed in an earlier run fails now ([CORPUS-HISTORY]). Nothing else
/// fails a run: a pair that never passed is a known defect.
fn held_to_the_record(card: &Scorecard, scoring: Scoring) -> Result<()> {
    let rejected = card
        .outcomes
        .iter()
        .filter(|pair| pair.standing.fails_the_run())
        .count();
    if scoring.strict && rejected > 0 {
        return Err(anyhow!(
            "{rejected} judged pair(s) on record as passing fail now — see the Judged pair \
             record section above"
        ));
    }
    let backslid = history::backslid(card);
    if backslid > 0 {
        return Err(anyhow!(
            "{backslid} judged pair(s) passed in an earlier run and fail now — see the Run \
             history section above"
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
        Command::Score(args) => score(&args),
    }
}
