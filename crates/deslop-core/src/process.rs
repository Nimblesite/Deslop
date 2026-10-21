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

#[cfg(all(test, windows))]
#[path = "process/tests.rs"]
mod tests;

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
/// [LIVE-PARENT-LIVENESS] Checks a kernel process handle without launching
/// children. An exited process signals its handle; access-denied and unknown
/// errors conservatively preserve the server, matching the Unix probe.
#[cfg(windows)]
#[must_use]
pub fn process_is_alive(process_id: u32) -> bool {
    use winsafe::{co, HPROCESS};

    const NO_WAIT_MILLISECONDS: u32 = 0;
    let process = match HPROCESS::OpenProcess(co::PROCESS::SYNCHRONIZE, false, process_id) {
        Ok(process) => process,
        Err(co::ERROR::INVALID_PARAMETER) => return false,
        Err(_) => return true,
    };
    !matches!(
        process.WaitForSingleObject(Some(NO_WAIT_MILLISECONDS)),
        Ok(co::WAIT::OBJECT_0)
    )
}

/// Conservatively reports the process as alive on platforms without a
/// process-probing backend, so a server is never killed by a monitor it
/// cannot implement.
#[cfg(not(any(unix, windows)))]
#[must_use]
pub fn process_is_alive(_process_id: u32) -> bool {
    true
}
