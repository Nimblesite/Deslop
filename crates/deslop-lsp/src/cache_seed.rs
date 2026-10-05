//! Cache-seeded LSP startup for.

use std::{path::PathBuf, sync::Arc, time::Duration};

use deslop_core::{
    embedding::{EmbeddingMode, EmbeddingProvider},
    live::{
        broadcast_report_changed, AnalysisSession, AnalysisState, ChangeSummary, Clock, LiveError,
        LiveService, ReportChangedNotification, ReportChangedSender, SystemClock,
    },
    EmbeddingSettings, PipelineSession, ReportDelta,
};
use tokio::sync::Mutex;
use tower_lsp::Client;

use crate::notifications::{AnalysisStateLspNotification, ReportChangedLspNotification};

/// Yield delay between embedding batches during the cold refresh.
/// Keeps the live editor responsive while the deferred pass runs.
const LIVE_EMBEDDING_BATCH_SLEEP: Duration = Duration::from_millis(10);

/// [LIVE-CACHE-SEED-READINESS] Retains the cold pass state for late connections.
pub(crate) fn new_startup_state(seeded: bool) -> Arc<Mutex<AnalysisState>> {
    let state = if seeded {
        AnalysisState::Running {
            started_at_ms: now_ms(),
        }
    } else {
        AnalysisState::Idle
    };
    Arc::new(Mutex::new(state))
}

/// Opens a live session, preferring `.deslop/cache/live-report.json`
/// when it is present and valid.
pub(crate) fn open_session(
    root: PathBuf,
    min_nodes: u32,
    incremental: bool,
    config_path: Option<PathBuf>,
    provider: Arc<dyn EmbeddingProvider>,
    mode: EmbeddingMode,
) -> Result<(AnalysisSession, bool), LiveError> {
    let cached = AnalysisSession::try_seeded_from_cache(
        root.clone(),
        min_nodes,
        incremental,
        config_path.clone(),
        Arc::clone(&provider),
        mode,
    );
    if let Some(session) = cached {
        return Ok((session, true));
    }
    AnalysisSession::new_with_mode(root, min_nodes, incremental, config_path, provider, mode)
        .map(|session| (session, false))
}

/// Starts the cold analysis pass without blocking cache-backed queries.
pub(crate) fn spawn_refresh(task: RefreshTask) {
    let _join = tokio::spawn(async move {
        push_initial_state(&task.client, &task.startup_state).await;
        let result = initialise_in_background(&task).await;
        finish_refresh(task, result).await;
    });
}

/// Publishes the cold pass result after background initialization returns.
async fn finish_refresh(
    task: RefreshTask,
    result: Result<(PipelineSession, deslop_core::Report), LiveError>,
) {
    match result {
        Ok((pipeline, report)) => commit_refresh(task, pipeline, report).await,
        Err(error) => report_refresh_error(&task.client, &task.startup_state, &error).await,
    }
}

/// Inputs required to run and commit the deferred cold pass.
pub(crate) struct RefreshTask {
    /// Cache-seeded session that will receive the fresh pipeline.
    pub(crate) session: Arc<Mutex<AnalysisSession>>,
    /// Live service used to retain the previous report snapshot.
    pub(crate) service: Arc<LiveService>,
    /// LSP client used for push notifications.
    pub(crate) client: Client,
    /// Workspace root.
    pub(crate) root: PathBuf,
    /// Minimum subtree size.
    pub(crate) min_nodes: u32,
    /// Whether the fingerprint cache is enabled.
    pub(crate) incremental: bool,
    /// Optional explicit `.deslop.toml` path.
    pub(crate) config_path: Option<PathBuf>,
    /// Provider used by the cold pass.
    pub(crate) provider: Arc<dyn EmbeddingProvider>,
    /// Embedding mode used by the cold pass.
    pub(crate) mode: EmbeddingMode,
    /// Broadcast sender shared with the scheduler so MCP IPC
    /// subscribers see the cache-seed cold-pass commit alongside
    /// scheduler-driven passes.
    pub(crate) report_changed: ReportChangedSender,
    /// Retained startup state, including the original error when refresh fails.
    pub(crate) startup_state: Arc<Mutex<AnalysisState>>,
}

/// Runs `PipelineSession::initialise` on a blocking thread so the
/// cache-seeded session can keep serving queries while the cold pass
/// catches up.
async fn initialise_in_background(
    task: &RefreshTask,
) -> Result<(PipelineSession, deslop_core::Report), LiveError> {
    let root = task.root.clone();
    let config_path = task.config_path.clone();
    let provider = Arc::clone(&task.provider);
    let mode = task.mode;
    let min_nodes = task.min_nodes;
    let incremental = task.incremental;
    tokio::task::spawn_blocking(move || {
        let embedding = EmbeddingSettings {
            mode,
            provider: Some(provider.as_ref()),
            batch_yield: live_batch_yield(mode),
            progress: None,
        };
        Ok(PipelineSession::initialise(
            root,
            min_nodes,
            incremental,
            config_path,
            embedding,
        )?)
    })
    .await
    .map_err(|error| LiveError::SchedulerBusy {
        message: error.to_string(),
    })?
}

/// Installs the freshly-built pipeline on the session, computes the
/// delta, retains the previous snapshot, and pushes the report-changed
/// + state notifications.
async fn commit_refresh(task: RefreshTask, pipeline: PipelineSession, report: deslop_core::Report) {
    let installed = {
        let mut guard = task.session.lock().await;
        let previous_generation = guard.generation();
        let previous_report = guard.report();
        guard.install_pipeline(pipeline, report).map(|_previous| {
            let generation = guard.generation();
            let current = guard.report();
            let delta = ReportDelta::between(
                Some((previous_generation, previous_report.as_ref())),
                generation,
                current.as_ref(),
            );
            // Persist the post-cold-pass snapshot so the next LSP
            // startup has a warm seed cache ([LIVE-SEED-CACHE]). The
            // call is the only seed-cache write path in this module.
            guard.persist_seed_cache();
            (previous_generation, previous_report, generation, delta)
        })
    };
    let (previous_generation, previous_report, generation, delta) = match installed {
        Ok(installed) => installed,
        Err(error) => {
            report_refresh_error(&task.client, &task.startup_state, &error).await;
            return;
        }
    };
    task.service
        .remember_snapshot(previous_generation, previous_report)
        .await;
    let notification = ReportChangedNotification {
        generation,
        summary: ChangeSummary::from_delta(&delta),
    };
    broadcast_report_changed(&task.report_changed, notification.clone());
    task.client
        .send_notification::<ReportChangedLspNotification>(notification)
        .await;
    // [LIVE-CACHE-SEED-READINESS] Installation and deferred replay are complete.
    publish_state(&task.client, &task.startup_state, AnalysisState::Idle).await;
}

/// Logs the refresh failure and pushes an `errored` analysis-state
/// notification so the editor surfaces the failure.
async fn report_refresh_error(
    client: &Client,
    startup_state: &Mutex<AnalysisState>,
    error: &LiveError,
) {
    tracing::error!(%error, "cache_seed_refresh_failed");
    publish_state(
        client,
        startup_state,
        AnalysisState::Errored {
            message: error.to_string(),
        },
    )
    .await;
}

/// Pushes the current [`AnalysisState`] to a freshly-connected editor
/// from `initialized()`. Closes the startup race where the cold pass's
/// `running`/`idle` broadcasts predate the VSIX notification handlers:
/// a fresh (non-seeded) session has already finished its blocking scan,
/// so it reports `Idle`; a seeded session still running its cold pass
/// reports `Running`; a failed cold pass retains `Errored` and its message.
pub(crate) async fn push_initial_state(client: &Client, startup_state: &Mutex<AnalysisState>) {
    let state = startup_state.lock().await;
    push_state(client, state.clone()).await;
}

/// Serializes retained state and notification delivery with late initialization.
async fn publish_state(
    client: &Client,
    startup_state: &Mutex<AnalysisState>,
    state: AnalysisState,
) {
    let mut current = startup_state.lock().await;
    *current = state.clone();
    push_state(client, state).await;
}

/// Pushes a `deslop/analysisState` notification carrying the tagged
/// [`AnalysisState`] object (`running`, `idle`, `errored`).
async fn push_state(client: &Client, state: AnalysisState) {
    client
        .send_notification::<AnalysisStateLspNotification>(state)
        .await;
}

/// Milliseconds since the UNIX epoch via the production clock, reused so
/// the cold-pass `started_at_ms` matches the scheduler's timestamps.
fn now_ms() -> u64 {
    SystemClock::new().now_ms()
}

/// Returns the per-batch sleep yield for the embedding pipeline. `None`
/// when embeddings are off; `Some(LIVE_EMBEDDING_BATCH_SLEEP)` otherwise.
fn live_batch_yield(mode: EmbeddingMode) -> Option<Duration> {
    if matches!(mode, EmbeddingMode::Off) {
        None
    } else {
        Some(LIVE_EMBEDDING_BATCH_SLEEP)
    }
}

#[cfg(test)]
mod tests;
