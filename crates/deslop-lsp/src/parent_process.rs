//! Parent-process lifecycle guard for LSP `initialize.processId`.

use std::{thread, time::Duration};

use deslop_core::process::process_is_alive;

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
        if !process_is_alive(process_id) {
            tracing::warn!(
                parent_process_id = process_id,
                "lsp parent process disappeared; exiting",
            );
            std::process::exit(0);
        }
        thread::sleep(Duration::from_millis(MONITOR_INTERVAL_MS));
    }
}
