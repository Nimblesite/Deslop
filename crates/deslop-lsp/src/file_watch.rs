//! Filesystem watcher startup and scheduler-to-LSP notification
//! forwarding ([LIVE-WATCHER], [LSP-PUSH]).
//!
//! ## Why this module exists
//!
//! The LSP *also* receives `textDocument/didChange` from the editor,
//! but that only covers files the user has open. This module starts a
//! `notify`-backed [`LiveWatcher`] on the **whole workspace root** so
//! that changes made by AI agents, CI runners, `git` operations, or
//! any other tool outside the editor also trigger incremental
//! re-analysis — with no polling.
//!
//! Once the [`Scheduler`] completes a pass it broadcasts a
//! [`ReportChangedNotification`] and an [`AnalysisState`]. A
//! background tokio task (started here) forwards those broadcasts to
//! the connected editor as `deslop/reportChanged` and
//! `deslop/analysisState` notifications so every VSIX surface (tree,
//! decorations, bubble, status bar) refreshes immediately without any
//! editor action ([LSP-PUSH-NOTIFICATIONS], [VSIX-REACTIVITY-INVARIANT]).

use std::{path::Path, sync::Arc};

use deslop_core::{
    live::{
        AnalysisSession, AnalysisState, LiveError, LiveWatcher, ReportChangedNotification,
        Scheduler,
    },
    ExclusionConfig,
};
use tokio::sync::{broadcast::Receiver, Mutex};
use tower_lsp::Client;

use crate::backend::{AnalysisStateLspNotification, ReportChangedLspNotification, ANALYSIS_STATE};

/// File extensions the watcher monitors — one entry per supported language.
const WATCHED_EXTENSIONS: &[&str] = &["cs", "rs", "py"];

/// Starts the filesystem watcher + scheduler for `root` and spawns
/// the background task that forwards change broadcasts to `client`.
///
/// The returned `(LiveWatcher, Scheduler)` must be kept alive for the
/// duration of the server — dropping either stops the analysis loop.
///
/// # Errors
///
/// Returns [`LiveError::WatcherInit`] when the OS refuses to watch
/// `root` (e.g. permission denied on the workspace directory).
pub fn start(
    root: &Path,
    session: Arc<Mutex<AnalysisSession>>,
    client: Client,
) -> Result<(LiveWatcher, Scheduler), LiveError> {
    let extensions: Vec<String> = WATCHED_EXTENSIONS.iter().map(|e| (*e).to_owned()).collect();
    let exclusion = Arc::new(ExclusionConfig::empty());
    let (watcher, watcher_rx) =
        LiveWatcher::start(root, extensions, exclusion).map_err(|err| LiveError::WatcherInit {
            message: err.to_string(),
        })?;
    let scheduler = Scheduler::with_system_clock(session, watcher_rx);
    let report_rx = scheduler.subscribe_report_changed();
    let state_rx = scheduler.subscribe_state();
    let _join = tokio::spawn(forward_broadcasts(client, report_rx, state_rx));
    tracing::info!(root = %root.display(), "file_watch started");
    Ok((watcher, scheduler))
}

/// Loops over both broadcast channels and pushes each event as an LSP
/// notification. Exits when the schedulers are dropped (server shutdown).
async fn forward_broadcasts(
    client: Client,
    mut report_rx: Receiver<ReportChangedNotification>,
    mut state_rx: Receiver<AnalysisState>,
) {
    loop {
        tokio::select! {
            result = report_rx.recv() => {
                match result {
                    Ok(n) => push_report_changed(&client, n).await,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "report_changed broadcast lagged");
                    }
                }
            }
            result = state_rx.recv() => {
                match result {
                    Ok(s) => push_analysis_state(&client, s).await,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "analysis_state broadcast lagged");
                    }
                }
            }
        }
    }
    tracing::debug!("file_watch broadcast forwarder exited");
}

/// Pushes `deslop/reportChanged` for a watcher-triggered analysis pass.
async fn push_report_changed(client: &Client, notification: ReportChangedNotification) {
    tracing::debug!(
        generation = notification.generation,
        "file_watch pushing deslop/reportChanged",
    );
    client
        .send_notification::<ReportChangedLspNotification>(notification)
        .await;
}

/// Pushes `deslop/analysisState` as a plain string so the VSIX can
/// check `state === "running"` directly ([LSP-PUSH-NOTIFICATIONS]).
async fn push_analysis_state(client: &Client, state: AnalysisState) {
    let _ = ANALYSIS_STATE; // referenced via the notification type, suppress lint
    let payload = match state {
        AnalysisState::Idle => "idle".to_owned(),
        AnalysisState::Running { .. } => "running".to_owned(),
        AnalysisState::Errored { .. } => "errored".to_owned(),
    };
    client
        .send_notification::<AnalysisStateLspNotification>(payload)
        .await;
}
