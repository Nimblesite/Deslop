//! Parent-process lifecycle guard for LSP `initialize.processId`.

use std::{thread, time::Duration};

#[cfg(windows)]
use std::process::Command;

/// Poll interval for detecting when the editor process disappears.
const MONITOR_INTERVAL_MS: u64 = 250;

/// Starts a detached monitor that exits this LSP when the parent dies.
pub(crate) fn start_monitor(process_id: Option<u32>) {
    let Some(process_id) = process_id else {
        return;
    };
    match thread::Builder::new()
        .name("deslop-lsp-parent-process-monitor".to_owned())
        .spawn(move || monitor_parent(process_id))
    {
        Ok(handle) => drop(handle),
        Err(error) => tracing::warn!(
            %error,
            parent_process_id = process_id,
            "failed to start lsp parent process monitor",
        ),
    }
}

/// Polls the parent process until it disappears, then exits this process.
fn monitor_parent(process_id: u32) -> ! {
    loop {
        if !process_exists(process_id) {
            tracing::warn!(
                parent_process_id = process_id,
                "lsp parent process disappeared; exiting",
            );
            std::process::exit(0);
        }
        thread::sleep(Duration::from_millis(MONITOR_INTERVAL_MS));
    }
}

/// Returns whether `process_id` currently resolves to a live process.
/// Uses `kill(pid, None)` (signal 0): the kernel performs the existence
/// / permission check it would for any signal but never delivers one.
/// Direct syscall through `nix` replaces the previous `kill -0`
/// subprocess spawn so the poll stays under a microsecond — important
/// under heavy concurrent test load where the `wait_for_exit` deadline
/// is tight.
#[cfg(unix)]
#[must_use]
fn process_exists(process_id: u32) -> bool {
    let Ok(pid_raw) = i32::try_from(process_id) else {
        return false;
    };
    match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid_raw), None) {
        Err(nix::errno::Errno::ESRCH) => false,
        Ok(()) | Err(_) => true,
    }
}

/// Returns whether `process_id` currently resolves to a live process.
#[cfg(windows)]
#[must_use]
fn process_exists(process_id: u32) -> bool {
    let filter = format!("PID eq {process_id}");
    Command::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| line.contains(&process_id.to_string()))
        })
}

/// Keeps the LSP alive on platforms without a process-probing backend.
#[cfg(not(any(unix, windows)))]
#[must_use]
fn process_exists(_process_id: u32) -> bool {
    true
}
