//! End-to-end coverage for issue #28: the LSP needs a `--worker-threads N`
//! knob so users can background-ise the analyser when it is saturating
//! CPU on large workspaces.
//!
//! Audience: HUMAN. The editor user who runs `deslop-lsp` on a
//! sprawling monorepo needs a way to constrain the tokio worker
//! count. The flag must be accepted without error; the exact tokio
//! configuration is covered at unit test level inside the LSP crate
//! but the CLI contract is owned here.

use std::{
    io::{BufRead, BufReader},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{
    session::{shutdown, spawn_logging_lsp, take_stdin_stdout},
    *,
};

/// Longest a startup log line may take to appear before the knob is
/// considered unrecorded. A failure bound, never a synchronisation
/// device: the assertion resolves the instant the line arrives.
const STARTUP_LOG_TIMEOUT: Duration = Duration::from_secs(20);

/// Audience: HUMAN. Issue #28. Positive invariant: when the LSP
/// user passes `--worker-threads N`, the startup log line records
/// the chosen value so the user can confirm the knob took effect
/// by tailing the log. The log line goes to stderr; we read it.
#[test]
fn lsp_startup_log_records_the_worker_threads_knob() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let mut child = spawn_logging_lsp(workspace.path(), &["--worker-threads", "2"])?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("child stderr missing"))?;
    let observed = read_stderr_until(stderr, "worker_threads=2");
    // Captured before the kill: a process that already exited on its own
    // never got as far as logging, which is a different failure from one
    // that ran and logged the wrong value. Empty stderr plus an early
    // exit means the binary never started — say so instead of blaming
    // the knob.
    let early_exit = child.try_wait().ok().flatten();
    let _status = deslop_test_support::reap::reap(&mut child);

    assert!(
        observed.contains("worker_threads=2"),
        "startup log must record the honored worker_threads value so users can confirm \
         the throttle knob took effect; early_exit={early_exit:?} stderr was:\n{observed}"
    );

    Ok(())
}

/// Reads the child's stderr until `marker` appears or
/// [`STARTUP_LOG_TIMEOUT`] elapses, returning everything seen.
///
/// The previous revision slept a flat 1500 ms, killed the child, and
/// scraped `wait_with_output`. That is a race, not a wait: the assertion
/// held only when the server happened to have flushed its startup log
/// inside the nap, so under a loaded machine — a parallel `cargo test`
/// is enough — the same binary passed and failed on back-to-back runs
/// and reddened a fail-fast gate at random. CLAUDE.md's testing rules
/// require determinism and forbid `sleep`, so the wait is now driven by
/// the event itself.
///
/// The read runs on its own thread because the server stays alive and
/// quiet after logging: a blocking `read_line` on the test thread would
/// never return once the startup lines stop, which is the same undrained
/// -pipe class of hang GH #370 removed from the shared harness.
fn read_stderr_until(stderr: std::process::ChildStderr, marker: &str) -> String {
    let (sender, receiver) = mpsc::channel();
    let _reader = thread::spawn(move || {
        let mut lines = BufReader::new(stderr).lines();
        while let Some(Ok(line)) = lines.next() {
            if sender.send(line).is_err() {
                return;
            }
        }
    });
    let deadline = Instant::now()
        .checked_add(STARTUP_LOG_TIMEOUT)
        .unwrap_or_else(Instant::now);
    let mut observed = String::new();
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let Ok(line) = receiver.recv_timeout(remaining) else {
            break;
        };
        observed.push_str(&line);
        observed.push('\n');
        if line.contains(marker) {
            break;
        }
    }
    observed
}

/// Audience: HUMAN. Issue #28. Positive invariant: the user-facing
/// throttle settings are honored at startup, and the structured CPU
/// report can prove the LSP is idle with no queued watcher or embedding
/// work when the workspace is quiet.
#[test]
fn lsp_nice_and_worker_knobs_preserve_idle_cpu_report() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let mut child = spawn_logging_lsp(workspace.path(), &["--worker-threads", "1", "--nice", "5"])?;

    thread::sleep(Duration::from_millis(250));
    if child.try_wait()?.is_some() {
        let output = child.wait_with_output()?;
        return Err(anyhow!(
            "deslop-lsp exited before handshake; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let (mut stdin, mut reader) = take_stdin_stdout(&mut child)?;

    let _init = handshake(&mut stdin, &mut reader)?;
    let response = call(&mut stdin, &mut reader, "deslop/cpuReport", &json!({}))?;
    let result = response
        .get("result")
        .ok_or_else(|| anyhow!("cpu report missing result: {response}"))?;
    assert_eq!(
        result.get("current_phase").and_then(Value::as_str),
        Some("idle"),
        "quiet workspace should report idle CPU phase: {result}"
    );
    assert!(
        result
            .get("last_100_phases")
            .and_then(Value::as_array)
            .is_some(),
        "cpu report should expose phase history even when empty: {result}"
    );
    assert_eq!(
        result
            .get("in_flight")
            .and_then(|value| value.get("pending_watcher_events"))
            .and_then(Value::as_u64),
        Some(0),
        "idle report should have no pending watcher events: {result}"
    );
    assert_eq!(
        result
            .get("in_flight")
            .and_then(|value| value.get("pending_embed_requests"))
            .and_then(Value::as_u64),
        Some(0),
        "idle report should have no pending embedding requests: {result}"
    );
    assert!(
        result
            .get("handler_counts")
            .and_then(|value| value.get("deslop/cpuReport"))
            .and_then(Value::as_u64)
            .is_some_and(|count| count >= 1),
        "cpu report should count the diagnostic request itself: {result}"
    );

    let shutdown_response = shutdown(&mut stdin, &mut reader)?;
    assert!(
        shutdown_response.get("error").is_none(),
        "shutdown should complete cleanly after idle cpu report: {shutdown_response}"
    );
    drop(stdin);

    let output = child.wait_with_output()?;
    let stderr_buf = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr_buf.contains("worker_threads=1"),
        "startup log must record worker_threads=1; stderr was:\n{stderr_buf}"
    );
    assert!(
        stderr_buf.contains("nice=5"),
        "startup log must record nice=5; stderr was:\n{stderr_buf}"
    );
    Ok(())
}
