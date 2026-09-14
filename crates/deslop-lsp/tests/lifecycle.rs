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
    io::{copy, sink, BufReader},
    process::{ChildStdout, ExitStatus},
    sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender},
    thread::{sleep, spawn},
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

/// Longest the server may take to end once it has been told to, measured
/// from the moment it closed its own output. A failure bound, never a
/// synchronisation device: each assertion resolves the instant the process
/// ends.
///
/// It stays tight on purpose. Waiting for the analysis pass is
/// [`OUTPUT_CLOSE_CEILING`]'s job; by the time that has resolved the serve
/// loop has already seen the end of its input, and all that remains is the
/// process leaving. That takes milliseconds or it never happens.
///
/// `closing_stdin_ends_the_process_successfully` once blew this bound on a
/// two-core CI runner while the three `exit` cases passed, and slowness was
/// the wrong reading: the client had stopped consuming stdout, so the server
/// blocked writing the diagnostics for the pass it had already started, and
/// its serve loop never reached the end of its input — the very leak
/// [LSP-LIFECYCLE] exists to prevent, wearing a timeout as a disguise.
/// Draining the output fixed it. Widening this instead would have bought
/// the fix nothing and cost the suite the only signal that says `hung`.
const EXIT_WINDOW: Duration = Duration::from_secs(15);

/// How long `shutdown` alone must fail to end the process before the
/// "still serving" promise counts as kept.
const STILL_ALIVE_WINDOW: Duration = Duration::from_millis(750);

/// Base protocol: `exit` after `shutdown` is a successful termination.
const EXIT_CODE_AFTER_SHUTDOWN: i32 = 0;

/// Base protocol: `exit` without a preceding `shutdown` is an error
/// termination — the client tore the session down out of order.
const EXIT_CODE_WITHOUT_SHUTDOWN: i32 = 1;

/// The base-protocol request that stops the server accepting work.
const SHUTDOWN: &str = "shutdown";

/// The base-protocol notification that ends the process.
const EXIT: &str = "exit";

/// Longest the server may hold its output open after the client has gone.
///
/// Not a synchronisation device either: the drain resolves the moment the
/// server closes stdout. This bound only separates "still finishing the
/// analysis pass it had already started, on a loaded and instrumented
/// runner" from "hung", and gh #370 — a refresh that ran for fourteen
/// minutes — is why the second has to be caught at all.
const OUTPUT_CLOSE_CEILING: Duration = Duration::from_secs(120);

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

    still_running_throughout(&mut child, STILL_ALIVE_WINDOW)?;

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

    let status = ended_within(&mut child, stdout, EXIT_WINDOW)?;
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

    let status = ended_within(&mut child, stdout, EXIT_WINDOW)?;
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

    let status = ended_within(&mut child, stdout, EXIT_WINDOW)?;
    assert_eq!(
        status.code(),
        Some(EXIT_CODE_AFTER_SHUTDOWN),
        "a client that vanished is not a server fault — stdin EOF must end the process \
         cleanly, got {status:?}"
    );
    Ok(())
}

/// Asserts `child` is still running at every poll across `window`.
fn still_running_throughout(child: &mut std::process::Child, window: Duration) -> Result<()> {
    let deadline = Instant::now()
        .checked_add(window)
        .unwrap_or_else(Instant::now);
    while Instant::now() < deadline {
        assert!(
            child.try_wait()?.is_none(),
            "shutdown alone must not end the process — the client still has to send `exit`"
        );
        sleep(POLL_INTERVAL);
    }
    Ok(())
}

/// Waits for `child` to end, draining its output first.
///
/// Every failure path kills and reaps: a server that never closed its output
/// would otherwise outlive the test that was there to prove it does not, and
/// the reader thread would stay blocked on it.
fn ended_within(
    child: &mut std::process::Child,
    stdout: BufReader<ChildStdout>,
    window: Duration,
) -> Result<ExitStatus> {
    let (closed, drained) = channel();
    let reader = spawn(move || drain_to_sink(stdout, &closed));
    let ended = ended_once_output_closed(child, &drained, window);
    if ended.is_err() {
        force_end(child);
    }
    reader
        .join()
        .map_err(|_payload| anyhow!("the thread draining deslop-lsp's output panicked"))?;
    ended
}

/// Reads `stdout` to end of file and reports how that went, once.
fn drain_to_sink(mut stdout: BufReader<ChildStdout>, closed: &Sender<std::io::Result<()>>) {
    let _sent = closed.send(copy(&mut stdout, &mut sink()).map(|_bytes| ()));
}

/// Ends a server that would not end on its own.
///
/// Killing closes the pipe, which releases a reader still blocked on a
/// server that never finished, so the join in [`ended_within`] cannot hang.
fn force_end(child: &mut std::process::Child) {
    let _forced = child.kill();
    let _reaped = child.wait();
}

/// Waits for the reader to reach end of file, or says why it never did.
fn output_closed(drained: &Receiver<std::io::Result<()>>) -> Result<()> {
    match drained.recv_timeout(OUTPUT_CLOSE_CEILING) {
        Ok(read) => read.map_err(|error| {
            anyhow!("reading deslop-lsp's output failed before it ended: {error}")
        }),
        Err(RecvTimeoutError::Timeout) => Err(anyhow!(
            "deslop-lsp still held its output open {OUTPUT_CLOSE_CEILING:?} after the \
             client went away, so it never reached the end of its own input"
        )),
        // Blaming the server for a reader that died would be a false
        // accusation, and this suite exists to make accusations that hold.
        Err(RecvTimeoutError::Disconnected) => Err(anyhow!(
            "the thread draining deslop-lsp's output ended without reporting, so this run \
             proves nothing either way about the server"
        )),
    }
}

/// The exit status once the server has closed its output, or why it did not.
///
/// A client that has gone stops reading; the server does not stop writing,
/// because it still publishes diagnostics for the pass it had already
/// started. Once the pipe's buffer fills, that write blocks, the serve loop
/// never reaches the end of its input, and the process outlives the client —
/// the exact leak this suite exists to prevent, surfacing as a timeout that
/// reads like slowness. So the wait is in two parts: the server closing its
/// output, which is the event, and the process ending after it, which is the
/// contract. A read that failed is neither, and must not pass for either.
fn ended_once_output_closed(
    child: &mut std::process::Child,
    drained: &Receiver<std::io::Result<()>>,
    window: Duration,
) -> Result<ExitStatus> {
    output_closed(drained)?;
    wait_for_exit(child, window)?.ok_or_else(|| {
        anyhow!(
            "deslop-lsp was still running {window:?} after it closed its output — an editor \
             that opens this workspace never gets the process back"
        )
    })
}
