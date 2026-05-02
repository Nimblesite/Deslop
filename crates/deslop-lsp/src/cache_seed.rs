//! Cache-seeded LSP startup for GH #73.

use std::{
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use deslop_core::{
    embedding::{EmbeddingMode, EmbeddingProvider},
    live::{
        AnalysisSession, ChangeSummary, LiveError, LiveService, ReportChangedNotification,
    },
    EmbeddingSettings, PipelineSession, ReportDelta,
};
use tokio::sync::Mutex;
use tower_lsp::Client;

use crate::backend::{
    AnalysisStateLspNotification, ReportChangedLspNotification,
};

const LIVE_EMBEDDING_BATCH_SLEEP: Duration = Duration::from_millis(10);

/// Opens a live session, preferring `.deslop-cache/live-report.json`
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
        push_state(&task.client, "running").await;
        match initialise_in_background(&task).await {
            Ok((pipeline, report)) => commit_refresh(task, pipeline, report).await,
            Err(error) => report_refresh_error(&task.client, &error).await,
        }
    });
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
}

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

async fn commit_refresh(
    task: RefreshTask,
    pipeline: PipelineSession,
    report: deslop_core::Report,
) {
    let (previous_generation, previous_report, generation, delta) = {
        let mut guard = task.session.lock().await;
        let previous_generation = guard.generation();
        let previous_report = guard.report();
        if let Err(error) = guard.install_pipeline(pipeline, report) {
            report_refresh_error(&task.client, &error).await;
            return;
        }
        let current = guard.report();
        let delta = ReportDelta::between(
            Some((previous_generation, previous_report.as_ref())),
            guard.generation(),
            current.as_ref(),
        );
        (previous_generation, previous_report, guard.generation(), delta)
    };
    task.service
        .remember_snapshot(previous_generation, previous_report)
        .await;
    task.client
        .send_notification::<ReportChangedLspNotification>(ReportChangedNotification {
            generation,
            summary: ChangeSummary::from_delta(&delta),
        })
        .await;
    push_state(&task.client, "idle").await;
}

async fn report_refresh_error(client: &Client, error: &LiveError) {
    tracing::error!(%error, "cache_seed_refresh_failed");
    push_state(client, "errored").await;
}

async fn push_state(client: &Client, state: &str) {
    client
        .send_notification::<AnalysisStateLspNotification>(state.to_owned())
        .await;
}

fn live_batch_yield(mode: EmbeddingMode) -> Option<Duration> {
    if matches!(mode, EmbeddingMode::Off) {
        None
    } else {
        Some(LIVE_EMBEDDING_BATCH_SLEEP)
    }
}
