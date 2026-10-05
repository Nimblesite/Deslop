//! Cache startup and retained-state behavior ([LIVE-CACHE-SEED-READINESS]).

mod readiness;
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

const METHOD_POINTER: &str = "/method";
const STATE_POINTER: &str = "/params/state";
const IDLE_STATE: &str = "idle";

#[test]
fn open_session_reports_cache_seed_status() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture(temp.path())?;

    let (_fresh, _provider, seeded) = open_fixture_session(temp.path())?;
    assert!(
        !seeded,
        "first open must run a fresh analysis when no state file exists"
    );

    let (_cached, _provider, seeded) = open_fixture_session(temp.path())?;
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
    let mut fixture = refresh_fixture(temp.path(), new_startup_state(true)).await?;
    assert!(
        !fixture.seeded,
        "test setup should start with a fresh session"
    );

    let (pipeline, report) = initialise_in_background(&fixture.task).await?;

    let join = tokio::spawn(commit_refresh(fixture.task, pipeline, report));
    let first = next_client_frame(&mut fixture.socket).await?;
    let second = next_client_frame(&mut fixture.socket).await?;
    join.await?;

    assert_eq!(
        first.pointer(METHOD_POINTER).and_then(Value::as_str),
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
        second.pointer(METHOD_POINTER).and_then(Value::as_str),
        Some(ANALYSIS_STATE),
        "commit must publish the idle analysis state after the report: {second}"
    );
    assert!(
        second.pointer("/params").is_some_and(Value::is_object),
        "analysisState params must be the tagged AnalysisState object, not a bare \
         string the VSIX reads as `state.state === undefined`: {second}"
    );
    assert_eq!(
        second.pointer(STATE_POINTER).and_then(Value::as_str),
        Some(IDLE_STATE),
        "the tagged object must carry state=idle so the editor settles to ready: {second}"
    );
    Ok(())
}

#[tokio::test]
async fn spawn_refresh_pushes_running_then_report_then_idle(
) -> Result<(), Box<dyn std::error::Error>> {
    // Drives the detached deferred-refresh task end-to-end (the test
    // above only exercises the inner `initialise_in_background` /
    // `commit_refresh` helpers directly). Asserts the full
    // running → reportChanged → idle sequence the editor relies on
    // and that the retained cold-pass state becomes Idle once committed.
    let temp = tempfile::tempdir()?;
    let startup_state = new_startup_state(true);
    let mut fixture = refresh_fixture(temp.path(), Arc::clone(&startup_state)).await?;

    spawn_refresh(fixture.task);

    let running = next_client_frame(&mut fixture.socket).await?;
    assert_eq!(
        running.pointer(STATE_POINTER).and_then(Value::as_str),
        Some("running"),
        "spawn_refresh must push the Running state first: {running}"
    );
    let changed = next_client_frame(&mut fixture.socket).await?;
    assert_eq!(
        changed.pointer(METHOD_POINTER).and_then(Value::as_str),
        Some(REPORT_CHANGED),
        "the committed cold pass must publish reportChanged: {changed}"
    );
    let idle = next_client_frame(&mut fixture.socket).await?;
    assert_eq!(
        idle.pointer(STATE_POINTER).and_then(Value::as_str),
        Some(IDLE_STATE),
        "the cold pass must settle to idle once committed: {idle}"
    );
    assert!(
        matches!(*startup_state.lock().await, AnalysisState::Idle),
        "spawn_refresh must retain Idle once the pass commits"
    );
    Ok(())
}

#[tokio::test]
async fn refresh_error_pushes_errored_state() -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut socket) = initialized_loopback_client().await?;
    let startup_state = new_startup_state(true);
    report_refresh_error(
        &client,
        &startup_state,
        &LiveError::SchedulerBusy {
            message: "fixture".to_owned(),
        },
    )
    .await;
    assert!(matches!(
        *startup_state.lock().await,
        AnalysisState::Errored { .. }
    ));

    let frame = next_client_frame(&mut socket).await?;
    assert_eq!(
        frame.pointer(METHOD_POINTER).and_then(Value::as_str),
        Some(ANALYSIS_STATE),
        "refresh errors must publish analysis-state changes: {frame}"
    );
    assert_eq!(
        frame.pointer(STATE_POINTER).and_then(Value::as_str),
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
async fn initial_state_is_running_while_cold_pass_active() -> Result<(), Box<dyn std::error::Error>>
{
    let (client, mut socket) = initialized_loopback_client().await?;
    push_initial_state(&client, &new_startup_state(true)).await;

    let frame = next_client_frame(&mut socket).await?;
    assert_eq!(
        frame.pointer(METHOD_POINTER).and_then(Value::as_str),
        Some(ANALYSIS_STATE),
        "initialized() must publish the startup analysis state: {frame}"
    );
    assert_eq!(
        frame.pointer(STATE_POINTER).and_then(Value::as_str),
        Some("running"),
        "a late-connecting editor must see Running while the cold pass is still in flight: {frame}"
    );
    Ok(())
}

#[tokio::test]
async fn initial_state_is_idle_once_the_scan_has_settled() -> Result<(), Box<dyn std::error::Error>>
{
    let (client, mut socket) = initialized_loopback_client().await?;
    push_initial_state(&client, &new_startup_state(false)).await;

    let frame = next_client_frame(&mut socket).await?;
    assert_eq!(
        frame.pointer(STATE_POINTER).and_then(Value::as_str),
        Some(IDLE_STATE),
        "a settled (fresh or committed) session must report Idle so the panel can reach ready: {frame}"
    );
    Ok(())
}

/// Minimum subtree size every cache-seed test analyses the fixture with.
const FIXTURE_MIN_NODES: u32 = 30;

/// A deferred-refresh task wired to a loopback LSP client, plus the socket
/// its push notifications land on. Every cold-pass input the refresh tests
/// share is fixed here; only `startup_state` varies between them.
struct RefreshFixture {
    task: RefreshTask,
    socket: ClientSocket,
    seeded: bool,
    /// Held so the broadcast channel keeps a subscriber for the test.
    _report_changed_rx: tokio::sync::broadcast::Receiver<ReportChangedNotification>,
}

/// Writes the two-duplicate fixture under `root`, opens a live session over
/// it, and assembles the cold-pass task against a loopback LSP client.
async fn refresh_fixture(
    root: &Path,
    startup_state: Arc<Mutex<AnalysisState>>,
) -> Result<RefreshFixture, Box<dyn std::error::Error>> {
    write_fixture(root)?;
    let (session, provider, seeded) = open_fixture_session(root)?;
    let session = Arc::new(Mutex::new(session));
    let (client, socket) = initialized_loopback_client().await?;
    let (report_changed, report_changed_rx) = tokio::sync::broadcast::channel(8);
    let task = RefreshTask {
        service: Arc::new(LiveService::new(Arc::clone(&session))),
        session,
        client,
        root: root.to_path_buf(),
        min_nodes: FIXTURE_MIN_NODES,
        incremental: true,
        config_path: None,
        provider,
        mode: EmbeddingMode::Off,
        report_changed,
        startup_state,
    };
    Ok(RefreshFixture {
        task,
        socket,
        seeded,
        _report_changed_rx: report_changed_rx,
    })
}

/// Opens a live session over the fixture repo at `root` with the settings
/// every cache-seed test shares — min-nodes 30, fingerprint cache on, no
/// explicit config, embeddings off — plus its provider and seeded flag.
fn open_fixture_session(
    root: &Path,
) -> Result<(AnalysisSession, Arc<dyn EmbeddingProvider>, bool), LiveError> {
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(StubProvider::new());
    let (session, seeded) = open_session(
        root.to_path_buf(),
        FIXTURE_MIN_NODES,
        true,
        None,
        Arc::clone(&provider),
        EmbeddingMode::Off,
    )?;
    Ok((session, provider, seeded))
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

async fn initialized_loopback_client() -> Result<(Client, ClientSocket), Box<dyn std::error::Error>>
{
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

async fn next_client_frame(socket: &mut ClientSocket) -> Result<Value, Box<dyn std::error::Error>> {
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
    let response = response.ok_or_else(|| std::io::Error::other("initialize response missing"))?;
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
