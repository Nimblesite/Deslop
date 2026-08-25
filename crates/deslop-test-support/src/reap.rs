//! Ending a spawned Deslop server without discarding what it executed.
//!
//! A signalled process never runs libc's `atexit` handlers, so LLVM's
//! profile runtime never writes the `.profraw` it accumulated, and under
//! `cargo llvm-cov` every line the child executed is silently dropped.
//! Measured against this repository's own instrumented `deslop-lsp`:
//! `SIGKILL` and `SIGTERM` each produce **zero** profile files, closing
//! stdin produces one. The crate read 92.5% line coverage while
//! `threshold_warning.rs` showed 0 hits on the exact lines whose output a
//! green E2E asserts word for word — the coverage was never missing, it
//! was thrown away at teardown.
//!
//! Both Deslop servers exit on stdin EOF, so closing the pipe and reaping
//! the child preserves the profile. `kill` survives only as the fallback
//! for a child that has not honoured EOF by [`REAP_TIMEOUT`].

use std::{
    process::{Child, ChildStdin, ExitStatus},
    thread::sleep,
    time::{Duration, Instant},
};

/// How long a child gets to exit on stdin EOF before it is signalled.
/// Generous on purpose: the fallback loses the child's coverage, so
/// waiting is always cheaper than killing.
pub const REAP_TIMEOUT: Duration = Duration::from_secs(20);

/// Gap between `try_wait` polls while waiting out [`REAP_TIMEOUT`].
const REAP_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Closes `stdin` and reaps `child`, returning how it ended — `None` when
/// it had to be signalled, which is the branch that loses the profile.
///
/// Pass the [`ChildStdin`] whenever the caller took it off the child —
/// the server only sees EOF once every handle to its stdin is dropped, so
/// a retained one would hold the child open until [`REAP_TIMEOUT`] and
/// then lose exactly the profile this exists to keep.
pub fn reap_with_stdin(child: &mut Child, stdin: ChildStdin) -> Option<ExitStatus> {
    drop(stdin);
    reap(child)
}

/// Reaps `child` after closing whatever stdin handle it still owns.
///
/// Callers holding a separately-taken [`ChildStdin`] must drop it before
/// calling this (or use [`reap_with_stdin`]).
pub fn reap(child: &mut Child) -> Option<ExitStatus> {
    drop(child.stdin.take());
    wait_until(child, Instant::now() + REAP_TIMEOUT).or_else(|| force(child))
}

/// Polls `child` until it exits or `deadline` passes.
fn wait_until(child: &mut Child, deadline: Instant) -> Option<ExitStatus> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Err(_) => return None,
            Ok(None) if Instant::now() >= deadline => return None,
            Ok(None) => sleep(REAP_POLL_INTERVAL),
        }
    }
}

/// Signals a child that did not honour stdin EOF and reaps it. Its
/// coverage profile is lost — this is the branch worth never taking.
fn force(child: &mut Child) -> Option<ExitStatus> {
    let _killed = child.kill();
    child.wait().ok()
}
