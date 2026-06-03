//! Cross-platform "is this process still alive?" probe shared by the LSP
//! and MCP parent-process monitors.
//!
//! Both binaries spawn a detached monitor that exits the server when the
//! editor / agent that launched it disappears, so neither leaks an orphan
//! analysis process. The monitor *policy* (which pid to watch, how it was
//! discovered) differs per binary and stays in each binary; the
//! *primitive* — "does this pid resolve to a live process?" — was byte-for-byte
//! identical in both and now lives here so the two servers cannot drift.
//!
//! This is server shell scaffolding, not analysis. It sits alongside
//! [`crate::version_contract`] (the other cross-binary server glue) and is
//! a candidate to migrate into the shared `lspkit` toolkit once it matures
//! (see the repo migration note in `CLAUDE.md`).

/// Returns whether `process_id` currently resolves to a live process.
///
/// Issues `kill(pid, None)` (signal 0): the kernel performs the existence
/// / permission check it would for any signal but never delivers one, so
/// the probe stays well under a microsecond — important under the tight
/// monitor poll interval where a subprocess spawn would be far too slow.
#[cfg(unix)]
#[must_use]
pub fn process_is_alive(process_id: u32) -> bool {
    let Ok(pid_raw) = i32::try_from(process_id) else {
        return false;
    };
    match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid_raw), None) {
        Err(nix::errno::Errno::ESRCH) => false,
        Ok(()) | Err(_) => true,
    }
}

/// Returns whether `process_id` currently resolves to a live process.
///
/// Queries `tasklist` filtered to the pid; a successful, non-empty match
/// means the process is still running.
#[cfg(windows)]
#[must_use]
pub fn process_is_alive(process_id: u32) -> bool {
    let filter = format!("PID eq {process_id}");
    std::process::Command::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| line.contains(&process_id.to_string()))
        })
}

/// Conservatively reports the process as alive on platforms without a
/// process-probing backend, so a server is never killed by a monitor it
/// cannot implement.
#[cfg(not(any(unix, windows)))]
#[must_use]
pub fn process_is_alive(_process_id: u32) -> bool {
    true
}
