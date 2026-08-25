//! [LSP-LIFECYCLE] How the server ends, driven against the real binary.
//!
//! The LSP base protocol makes three promises about termination and an
//! editor depends on all three: `shutdown` stops the server accepting new
//! work but leaves the process running; `exit` ends the process —
//! successfully when `shutdown` came first, with a failure code when it
//! did not; and a closed stdin ends it too, because a crashed client
//! leaves nothing else behind.
//!
//! A server that ignores `exit` outlives every editor session that opened
//! it, and each abandoned process keeps its workspace watcher and its
//! analysis threads, so the machine accumulates one live analyser per
//! window the user has ever opened.
//!
//! This is also the contract the test harness reaps on
//! (`deslop_test_support::reap`): a signalled child writes no coverage
//! profile, so every line it executed is discarded. Termination that
//! works is what makes this crate's coverage measurable at all.

use std::{
    process::ExitStatus,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::{
    call, handshake, notification, spawn_lsp_on_fixture, wait_for_exit, write_frame, POLL_INTERVAL,
};

/// Fixture every case boots against — small enough that the first analysis
/// pass finishes well inside the windows below.
const FIXTURE: &str = "csharp-small";

/// Longest the server may take to end once it has been told to. A failure
/// bound, never a synchronisation device: each assertion resolves the
/// instant the process ends.
const EXIT_WINDOW: Duration = Duration::from_secs(15);

/// How long `shutdown` alone must fail to end the process before the
/// "still serving" promise counts as kept.
const STILL_ALIVE_WINDOW: Duration = Duration::from_millis(750);

/// Base protocol: `exit` after `shutdown` is a successful termination.
const EXIT_CODE_AFTER_SHUTDOWN: i32 = 0;

/// Base protocol: `exit` without a preceding `shutdown` is an error
/// termination — the client tore the session down out of order.
const EXIT_CODE_WITHOUT_SHUTDOWN: i32 = 1;

/// The two base-protocol methods under test, named once.
const SHUTDOWN: &str = "shutdown";
const EXIT: &str = "exit";

/// [LSP-LIFECYCLE] `shutdown` answers and leaves the process running.
/// Exiting here would strand the client mid-handshake, with the `exit`
/// notification it is required to send next going nowhere.
#[test]
fn shutdown_answers_without_ending_the_process() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) = spawn_lsp_on_fixture(FIXTURE)?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    let response = call(&mut stdin, &mut stdout, SHUTDOWN, &Value::Null)?;
    assert!(
        response.get("result").is_some(),
        "shutdown must answer in band before the process ends: {response}"
    );

    let deadline = Instant::now() + STILL_ALIVE_WINDOW;
    while Instant::now() < deadline {
        assert!(
            child.try_wait()?.is_none(),
            "shutdown alone must not end the process — the client still has to send `exit`"
        );
        sleep(POLL_INTERVAL);
    }

    let _status = deslop_test_support::reap::reap_with_stdin(&mut child, stdin);
    Ok(())
}

/// [LSP-LIFECYCLE] `exit` after `shutdown` ends the process, successfully.
/// Every editor sends this pair when it closes a workspace; a server that
/// ignores it is still running, still watching, and still analysing long
/// after the window is gone.
#[test]
fn exit_after_shutdown_ends_the_process_successfully() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) = spawn_lsp_on_fixture(FIXTURE)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let _shutdown = call(&mut stdin, &mut stdout, SHUTDOWN, &Value::Null)?;

    write_frame(&mut stdin, &notification(EXIT, &Value::Null)?)?;

    let status = ended_within(&mut child, EXIT_WINDOW)?;
    assert_eq!(
        status.code(),
        Some(EXIT_CODE_AFTER_SHUTDOWN),
        "`exit` after `shutdown` is a clean end: the base protocol fixes the code at \
         {EXIT_CODE_AFTER_SHUTDOWN}, got {status:?}"
    );
    Ok(())
}

/// [LSP-LIFECYCLE] `exit` without `shutdown` still ends the process, but
/// reports the out-of-order teardown through the exit code so the client's
/// crash handling can tell the two apart.
#[test]
fn exit_without_shutdown_ends_the_process_with_a_failure_code() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) = spawn_lsp_on_fixture(FIXTURE)?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    write_frame(&mut stdin, &notification(EXIT, &Value::Null)?)?;

    let status = ended_within(&mut child, EXIT_WINDOW)?;
    assert_eq!(
        status.code(),
        Some(EXIT_CODE_WITHOUT_SHUTDOWN),
        "`exit` with no preceding `shutdown` must end with {EXIT_CODE_WITHOUT_SHUTDOWN}, \
         got {status:?}"
    );
    Ok(())
}

/// [LSP-LIFECYCLE] A closed stdin ends the process. This is what a crashed
/// or force-quit client leaves behind, and it is the only teardown the
/// coverage-preserving test reaper can rely on.
#[test]
fn closing_stdin_ends_the_process_successfully() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) = spawn_lsp_on_fixture(FIXTURE)?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    drop(stdin);

    let status = ended_within(&mut child, EXIT_WINDOW)?;
    assert_eq!(
        status.code(),
        Some(EXIT_CODE_AFTER_SHUTDOWN),
        "a client that vanished is not a server fault — stdin EOF must end the process \
         cleanly, got {status:?}"
    );
    Ok(())
}

/// Waits for `child` to end, failing with the reason rather than hanging.
fn ended_within(child: &mut std::process::Child, window: Duration) -> Result<ExitStatus> {
    wait_for_exit(child, window)?.ok_or_else(|| {
        let _forced = child.kill();
        let _reaped = child.wait();
        anyhow!(
            "deslop-lsp was still running {window:?} after it was told to end — an editor \
             that opens this workspace never gets the process back"
        )
    })
}
