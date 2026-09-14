//! Filesystem watcher startup and scheduler-to-LSP notification
//! forwarding ([LIVE-WATCHER], [LSP-PUSH]).
//! Implements [PRINCIPLES-LIVE-IS-REACTIVE].
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
    config::watched_config_paths,
    live::{
        AnalysisSession, AnalysisState, LiveError, LiveExclusion, LiveWatcher,
        ReportChangedNotification, Scheduler,
    },
    pipeline::watched_source_extensions,
};
use tokio::sync::{broadcast::Receiver, Mutex};
use tower_lsp::{lsp_types::MessageType, Client};

use crate::notifications::{AnalysisStateLspNotification, ReportChangedLspNotification};

/// Starts the filesystem watcher + scheduler for `root` and spawns
/// the background task that forwards change broadcasts to `client`.
///
/// `exclusion` is the session's own policy handle, not a fresh config:
/// the watcher decides what reaches the scheduler and the session
/// decides what reaches the corpus, so two independently-built configs
/// let the live report disagree with the batch report over the same
/// files ([CONFIG-EXCLUDE-DEPENDENCIES]). Built-in artefact directories
/// stay excluded through that shared policy — the dependency opt-in
/// widens the corpus, it does not disable exclusion.
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
    config_path: Option<&Path>,
    session: Arc<Mutex<AnalysisSession>>,
    exclusion: LiveExclusion,
    client: Client,
) -> Result<(LiveWatcher, Scheduler), LiveError> {
    let extensions: Vec<String> = watched_source_extensions()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let config_paths = watched_config_paths(root, config_path);
    let (watcher, watcher_rx) =
        LiveWatcher::start(root, extensions, exclusion, config_paths.clone()).map_err(|err| {
            LiveError::WatcherInit {
                message: err.to_string(),
            }
        })?;
    let scheduler = Scheduler::with_system_clock(session, watcher_rx);
    let report_rx = scheduler.subscribe_report_changed();
    let state_rx = scheduler.subscribe_state();
    let _join = tokio::spawn(forward_broadcasts(client, report_rx, state_rx));
    tracing::info!(
        root = %root.display(),
        config_paths = ?config_paths,
        "file_watch started",
    );
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
///
/// Also surfaces the pass on the LSP `window/logMessage` channel — the
/// one log stream every client renders (the LSP4IJ Logs tab, the VS Code
/// output channel) — so the running engine is visible, not silent behind
/// stderr-only `tracing` the client never shows ([LSP-PUSH]).
async fn push_report_changed(client: &Client, notification: ReportChangedNotification) {
    tracing::debug!(
        generation = notification.generation,
        "file_watch pushing deslop/reportChanged",
    );
    client
        .log_message(MessageType::INFO, analysis_pass_log(&notification))
        .await;
    client
        .send_notification::<ReportChangedLspNotification>(notification)
        .await;
}

/// Human-readable one-liner announcing an analysis pass on the LSP client
/// console, e.g. `Deslop analysis pass 7: 0 added, 1 removed, 0 updated
/// (worst mass 1234)`.
fn analysis_pass_log(notification: &ReportChangedNotification) -> String {
    let summary = &notification.summary;
    format!(
        "Deslop analysis pass {}: {} added, {} removed, {} updated (worst mass {})",
        notification.generation,
        summary.clusters_added,
        summary.clusters_removed,
        summary.clusters_updated,
        summary.worst_mass,
    )
}

/// Pushes `deslop/analysisState` carrying the generated tagged
/// [`AnalysisState`] object so the VSIX reads `state.state` and drives
/// its lifecycle ([LSP-PUSH-NOTIFICATIONS], [VSIX reactivity]).
async fn push_analysis_state(client: &Client, state: AnalysisState) {
    client
        .send_notification::<AnalysisStateLspNotification>(state)
        .await;
}
