//! [CORPUS-MEASURE-HOST] Pins for the three things a measured scan owes the
//! machine it shares: the lowest priority, a deadline, and one run at a time.
//! They drive POSIX tools (`sh`, `sleep`, `true`); Windows' priority class is
//! set by the same spawn and needs no tool of its own.

use std::{
    fs,
    process::Command,
    time::{Duration, Instant},
};

use anyhow::{ensure, Context, Result};

use super::{is_live, Deadline, MeasureLock, SCAN_NICENESS};
use crate::corpus_measure::measured_run;

/// A deadline short enough that a test never waits on it for long.
const SHORT_DEADLINE: Duration = Duration::from_millis(200);
/// A child that outlives [`SHORT_DEADLINE`] many times over.
const LONG_SLEEP_SECONDS: &str = "30";
/// How long the killed child may take to be reaped, far below its own sleep.
const REAP_BOUND: Duration = Duration::from_secs(10);
/// What the refusal names.
const DEADLINE_WORD: &str = "deadline";
/// A lock file name for one test.
const TEST_LOCK: &str = "measure.lock";

/// The pid of a process that has already exited.
fn exited_pid() -> Result<u32> {
    let mut child = Command::new("true").spawn().context("spawn true")?;
    let pid = child.id();
    let _status = child.wait()?;
    Ok(pid)
}

#[test]
fn a_measured_scan_runs_at_the_lowest_priority_and_records_its_starting_load() -> Result<()> {
    let run = measured_run(
        std::path::Path::new("/bin/sh"),
        &["-c".into(), "ps -o nice= -p $$".into()],
    )?;
    ensure!(
        run.output.status.success(),
        "the probe ran: {:?}",
        run.output
    );
    // `nice` adds to the caller's niceness, and the platforms cap it (19 on
    // Linux, 20 on macOS), so a harness that is itself niced reads higher.
    let niceness: i32 = String::from_utf8(run.output.stdout)?.trim().parse()?;
    let lowest: i32 = SCAN_NICENESS.parse()?;
    ensure!(
        niceness >= lowest,
        "the scan itself runs at niceness {lowest} or lower priority, got {niceness}"
    );
    ensure!(
        run.cost.load_at_start.logical_cores >= 1,
        "the starting load is read against the machine's cores: {:?}",
        run.cost.load_at_start
    );
    ensure!(
        run.cost.load_at_start.one_minute >= 0.0,
        "a load average is never negative: {:?}",
        run.cost.load_at_start
    );
    Ok(())
}

#[test]
fn a_scan_past_its_deadline_is_killed_and_the_refusal_names_the_deadline() -> Result<()> {
    let started = Instant::now();
    let mut child = Command::new("sleep").arg(LONG_SLEEP_SECONDS).spawn()?;
    let deadline = Deadline::arm(child.id(), SHORT_DEADLINE);
    let status = child.wait()?;
    let refusal = deadline.disarm(SHORT_DEADLINE).err();
    ensure!(
        !status.success(),
        "the hung child was killed, not left to finish: {status}"
    );
    ensure!(
        started.elapsed() < REAP_BOUND,
        "killed at its deadline, not after its own sleep"
    );
    let message = refusal.map(|error| error.to_string()).unwrap_or_default();
    ensure!(
        message.contains(DEADLINE_WORD),
        "the refusal says why: {message:?}"
    );
    Ok(())
}

#[test]
fn a_scan_inside_its_deadline_is_left_alone() -> Result<()> {
    let mut child = Command::new("true").spawn()?;
    let deadline = Deadline::arm(child.id(), REAP_BOUND);
    let status = child.wait()?;
    deadline.disarm(REAP_BOUND)?;
    ensure!(
        status.success(),
        "a scan that finishes in time keeps its own exit status"
    );
    Ok(())
}

#[test]
fn a_lock_left_by_an_exited_process_is_taken_over_and_released_on_drop() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join(TEST_LOCK);
    fs::write(&path, exited_pid()?.to_string())?;
    let lock = MeasureLock::acquire_at(path.clone())?;
    ensure!(
        fs::read_to_string(&path)? == std::process::id().to_string(),
        "the lock now names this process"
    );
    drop(lock);
    ensure!(!path.exists(), "dropping the lock releases it");
    Ok(())
}

#[test]
fn a_lock_holder_is_live_only_while_its_process_runs() -> Result<()> {
    ensure!(
        is_live(&std::process::id().to_string()),
        "this process is live"
    );
    ensure!(
        !is_live(&exited_pid()?.to_string()),
        "an exited process is not"
    );
    ensure!(!is_live("not a pid"), "an unreadable holder is not");
    ensure!(!is_live(""), "an empty lock file names no holder");
    Ok(())
}
