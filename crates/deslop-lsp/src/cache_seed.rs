//! Cache-seeded LSP startup for GH #73.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

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
        push_state(
            &task.client,
            AnalysisState::Running {
                started_at_ms: now_ms(),
            },
        )
        .await;
        let result = initialise_in_background(&task).await;
        // The cold pass is about to commit or error. Clear the in-flight
        // flag before the terminal idle/errored push so a freshly
        // connected editor reading it in `initialized()` sees the settled
        // state instead of a phantom Running ([VSIX reactivity]).
        task.cold_pass_active.store(false, Ordering::SeqCst);
        match result {
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
    /// Broadcast sender shared with the scheduler so MCP IPC
    /// subscribers see the cache-seed cold-pass commit alongside
    /// scheduler-driven passes.
    pub(crate) report_changed: ReportChangedSender,
    /// Shared "cold pass still running" flag. Set true while this pass
    /// is in flight and cleared as it commits, so `initialized()` can
    /// report the correct startup state to a late-connecting editor.
    pub(crate) cold_pass_active: Arc<AtomicBool>,
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
            report_refresh_error(&task.client, &error).await;
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
    push_state(&task.client, AnalysisState::Idle).await;
}

/// Logs the refresh failure and pushes an `errored` analysis-state
/// notification so the editor surfaces the failure.
async fn report_refresh_error(client: &Client, error: &LiveError) {
    tracing::error!(%error, "cache_seed_refresh_failed");
    push_state(
        client,
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
/// reports `Running` ([VSIX reactivity]).
pub(crate) async fn push_initial_state(client: &Client, cold_pass_active: &AtomicBool) {
    let state = if cold_pass_active.load(Ordering::SeqCst) {
        AnalysisState::Running {
            started_at_ms: now_ms(),
        }
    } else {
        AnalysisState::Idle
    };
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
mod tests {
    use std::{path::Path, sync::Arc};

    use deslop_core::{embedding::test_support::StubProvider, live::LiveService};
    use futures::StreamExt as _;
    use serde_json::{json, Value};
    use tower::Service as _;
    use tower_lsp::{
        async_trait,
        jsonrpc::{Request, Response},
        lsp_types::{InitializeParams, InitializeResult, ServerCapabilities},
        Client, ClientSocket, LanguageServer, LspService,
    };

    use super::*;
    use crate::notifications::{ANALYSIS_STATE, REPORT_CHANGED};

    #[test]
    fn open_session_reports_cache_seed_status() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_fixture(temp.path())?;
        let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());

        let (_fresh, seeded) = open_session(
            temp.path().to_path_buf(),
            30,
            true,
            None,
            Arc::clone(&provider),
            EmbeddingMode::Off,
        )?;
        assert!(
            !seeded,
            "first open must run a fresh analysis when no state file exists"
        );

        let (_cached, seeded) = open_session(
            temp.path().to_path_buf(),
            30,
            true,
            None,
            provider,
            EmbeddingMode::Off,
        )?;
        assert!(
            seeded,
            "second open must load the valid state file written by the first session"
        );
        Ok(())
    }

    #[test]
    fn live_batch_yield_tracks_embedding_mode() {
        assert_eq!(live_batch_yield(EmbeddingMode::Off), None);
        assert_eq!(
            live_batch_yield(EmbeddingMode::Auto),
            Some(LIVE_EMBEDDING_BATCH_SLEEP)
        );
        assert_eq!(
            live_batch_yield(EmbeddingMode::Required),
            Some(LIVE_EMBEDDING_BATCH_SLEEP)
        );
    }

    #[tokio::test]
    async fn background_initialise_and_commit_pushes_report_and_idle_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_fixture(temp.path())?;
        let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());
        let (session, seeded) = open_session(
            temp.path().to_path_buf(),
            30,
            true,
            None,
            Arc::clone(&provider),
            EmbeddingMode::Off,
        )?;
        assert!(!seeded, "test setup should start with a fresh session");

        let session = Arc::new(Mutex::new(session));
        let service = Arc::new(LiveService::new(Arc::clone(&session)));
        let (client, mut socket) = initialized_loopback_client().await?;
        let (report_changed, _rx) = tokio::sync::broadcast::channel(8);
        let task = RefreshTask {
            session,
            service,
            client,
            root: temp.path().to_path_buf(),
            min_nodes: 30,
            incremental: true,
            config_path: None,
            provider,
            mode: EmbeddingMode::Off,
            report_changed,
            cold_pass_active: Arc::new(AtomicBool::new(true)),
        };
        let (pipeline, report) = initialise_in_background(&task).await?;

        let join = tokio::spawn(commit_refresh(task, pipeline, report));
        let first = next_client_frame(&mut socket).await?;
        let second = next_client_frame(&mut socket).await?;
        join.await?;

        assert_eq!(
            first.pointer("/method").and_then(Value::as_str),
            Some(REPORT_CHANGED),
            "commit must publish a reportChanged notification first: {first}"
        );
        assert!(
            first
                .pointer("/params/summary")
                .is_some_and(serde_json::Value::is_object),
            "reportChanged must include a delta summary: {first}"
        );
        assert_eq!(
            second.pointer("/method").and_then(Value::as_str),
            Some(ANALYSIS_STATE),
            "commit must publish the idle analysis state after the report: {second}"
        );
        assert!(
            second.pointer("/params").is_some_and(Value::is_object),
            "analysisState params must be the tagged AnalysisState object, not a bare \
             string the VSIX reads as `state.state === undefined`: {second}"
        );
        assert_eq!(
            second.pointer("/params/state").and_then(Value::as_str),
            Some("idle"),
            "the tagged object must carry state=idle so the editor settles to ready: {second}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn refresh_error_pushes_errored_state() -> Result<(), Box<dyn std::error::Error>> {
        let (client, mut socket) = initialized_loopback_client().await?;
        report_refresh_error(
            &client,
            &LiveError::SchedulerBusy {
                message: "fixture".to_owned(),
            },
        )
        .await;

        let frame = next_client_frame(&mut socket).await?;
        assert_eq!(
            frame.pointer("/method").and_then(Value::as_str),
            Some(ANALYSIS_STATE),
            "refresh errors must publish analysis-state changes: {frame}"
        );
        assert_eq!(
            frame.pointer("/params/state").and_then(Value::as_str),
            Some("errored"),
            "refresh errors must surface the errored state as a tagged object: {frame}"
        );
        assert!(
            frame
                .pointer("/params/message")
                .and_then(Value::as_str)
                .is_some(),
            "the errored analysis state must carry a human-readable message: {frame}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn initial_state_is_running_while_cold_pass_active(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (client, mut socket) = initialized_loopback_client().await?;
        push_initial_state(&client, &AtomicBool::new(true)).await;

        let frame = next_client_frame(&mut socket).await?;
        assert_eq!(
            frame.pointer("/method").and_then(Value::as_str),
            Some(ANALYSIS_STATE),
            "initialized() must publish the startup analysis state: {frame}"
        );
        assert_eq!(
            frame.pointer("/params/state").and_then(Value::as_str),
            Some("running"),
            "a late-connecting editor must see Running while the cold pass is still in flight: {frame}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn initial_state_is_idle_once_the_scan_has_settled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (client, mut socket) = initialized_loopback_client().await?;
        push_initial_state(&client, &AtomicBool::new(false)).await;

        let frame = next_client_frame(&mut socket).await?;
        assert_eq!(
            frame.pointer("/params/state").and_then(Value::as_str),
            Some("idle"),
            "a settled (fresh or committed) session must report Idle so the panel can reach ready: {frame}"
        );
        Ok(())
    }

    fn write_fixture(root: &Path) -> std::io::Result<()> {
        std::fs::write(
            root.join("Alpha.cs"),
            "class Alpha { int Add(int a, int b) { return a + b; } }\n",
        )?;
        std::fs::write(
            root.join("Beta.cs"),
            "class Beta { int Add(int a, int b) { return a + b; } }\n",
        )
    }

    async fn initialized_loopback_client(
    ) -> Result<(Client, ClientSocket), Box<dyn std::error::Error>> {
        let captured = Arc::new(std::sync::Mutex::new(None));
        let captured_client = Arc::clone(&captured);
        let (mut service, socket) = LspService::build(move |client| {
            if let Ok(mut captured) = captured_client.lock() {
                *captured = Some(client.clone());
            }
            DummyBackend
        })
        .finish();
        let request = Request::build("initialize")
            .params(json!({ "capabilities": {} }))
            .id(1_i64)
            .finish();
        futures::future::poll_fn(|cx| service.poll_ready(cx)).await?;
        let response = service.call(request).await?;
        assert_initialize_ok(response)?;
        let client = captured_client_from(&captured)?;
        Ok((client, socket))
    }

    fn captured_client_from(
        captured: &Arc<std::sync::Mutex<Option<Client>>>,
    ) -> Result<Client, Box<dyn std::error::Error>> {
        let guard = captured
            .lock()
            .map_err(|_| std::io::Error::other("capture client lock poisoned"))?;
        guard
            .clone()
            .ok_or_else(|| std::io::Error::other("loopback client was not captured").into())
    }

    async fn next_client_frame(
        socket: &mut ClientSocket,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let request = socket
            .next()
            .await
            .ok_or_else(|| std::io::Error::other("client socket closed before notification"))?;
        let (method, id, params) = request.into_parts();
        assert!(id.is_none(), "expected notification without request id");
        Ok(json!({
            "method": method,
            "params": params.unwrap_or(Value::Null),
        }))
    }

    fn assert_initialize_ok(response: Option<Response>) -> Result<(), Box<dyn std::error::Error>> {
        let response =
            response.ok_or_else(|| std::io::Error::other("initialize response missing"))?;
        let (_id, body) = response.into_parts();
        let _result = body.map_err(|_| std::io::Error::other("initialize returned an error"))?;
        Ok(())
    }

    #[derive(Debug)]
    struct DummyBackend;

    #[async_trait]
    impl LanguageServer for DummyBackend {
        async fn initialize(
            &self,
            _: InitializeParams,
        ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
            Ok(InitializeResult {
                capabilities: ServerCapabilities::default(),
                server_info: None,
            })
        }

        async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }
    }
}
