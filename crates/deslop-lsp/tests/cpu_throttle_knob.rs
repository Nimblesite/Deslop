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
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use anyhow::Result;

/// Audience: HUMAN. Issue #28. Positive invariant: when the LSP
/// user passes `--worker-threads N`, the startup log line records
/// the chosen value so the user can confirm the knob took effect
/// by tailing the log. The log line goes to stderr; we scrape it
/// briefly after spawn.
#[test]
fn lsp_startup_log_records_the_worker_threads_knob() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let child = Command::new(assert_cmd::cargo::cargo_bin("deslop-lsp"))
        .arg(workspace.path())
        .arg("--worker-threads")
        .arg("2")
        .env("RUST_LOG", "info")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Give the startup log line a moment to land, then kill and
    // drain stderr through `wait_with_output` so the pipe closes
    // cleanly and the tracing buffer reaches us.
    thread::sleep(Duration::from_millis(1500));
    #[allow(unused_mut)]
    let mut handle = child;
    let _ = handle.kill();
    let output = handle.wait_with_output()?;
    let stderr_buf = String::from_utf8_lossy(&output.stderr).into_owned();

    assert!(
        stderr_buf.contains("worker_threads=2"),
        "startup log must record the honored worker_threads value so users can confirm \
         the throttle knob took effect; stderr was:\n{stderr_buf}"
    );

    Ok(())
}
