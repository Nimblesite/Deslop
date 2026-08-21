//! E2E coverage for [LSP-COMMANDS] `workspace/executeCommand`.
//!
//! Drives the real `deslop-lsp` binary over stdio and responds to
//! client-bound `window/showDocument` requests so command handlers run
//! through the same transport a real editor uses.

mod common;

use std::process::{ChildStdin, ChildStdout};

use anyhow::{anyhow, Result};
use futures::{FutureExt, SinkExt, StreamExt};
use serde_json::{json, Value};
use tower::Service;
use tower_lsp::{
    jsonrpc::{Error, ErrorCode, Request, Response},
    ClientSocket, LspService,
};

use crate::common::{
    call, copy_fixture, handshake, read_frame, request, spawn_lsp_on_fixture, write_frame,
};
use deslop_lsp::{
    commands::{
        OPEN_CLUSTER, OPEN_REPORT, PICK_EMBEDDING_MODEL, REFRESH_REPORT, TOGGLE_INCREMENTAL,
    },
    LspBackend,
};

const EXECUTE_COMMAND: &str = "workspace/executeCommand";
const COMMAND_FIELD: &str = "command";
const RESULT_COMMAND_POINTER: &str = "/result/command";
const PROVIDER_ID_FIELD: &str = "provider_id";

#[test]
fn execute_command_provider_advertises_and_opens_virtual_documents() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let init = handshake(&mut stdin, &mut stdout)?;
    assert_advertised_commands(&init)?;

    let (report_response, report_shows) = call_with_show_document_response(
        &mut stdin,
        &mut stdout,
        &json!({ (COMMAND_FIELD): OPEN_REPORT }),
    )?;
    assert_eq!(report_shows.len(), 1, "expected one showDocument request");
    let report_show = only_show_request(&report_shows)?;
    assert_eq!(show_uri(report_show)?, "deslop://report");
    assert_eq!(show_take_focus(report_show), Some(true));
    assert_eq!(show_external(report_show), Some(false));
    assert_eq!(
        report_response
            .pointer("/result/uri")
            .and_then(Value::as_str),
        Some("deslop://report")
    );

    let (cluster_response, cluster_shows) = call_with_show_document_response(
        &mut stdin,
        &mut stdout,
        &json!({ (COMMAND_FIELD): OPEN_CLUSTER, "arguments": ["abc123"] }),
    )?;
    assert_eq!(cluster_shows.len(), 1, "expected one cluster document open");
    let cluster_show = only_show_request(&cluster_shows)?;
    assert_eq!(show_uri(cluster_show)?, "deslop://cluster/abc123");
    assert_eq!(
        cluster_response
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(OPEN_CLUSTER)
    );
    assert_eq!(
        cluster_response
            .pointer("/result/shown")
            .and_then(Value::as_bool),
        Some(true)
    );

    let _ = child.kill();
    Ok(())
}

#[test]
fn execute_command_dispatches_refresh_models_and_incremental_toggle() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    let initial_config = call(&mut stdin, &mut stdout, "deslop/sessionConfig", &json!({}))?;
    assert_eq!(
        initial_config
            .pointer("/result/incremental")
            .and_then(Value::as_bool),
        Some(true)
    );

    let toggled = execute(
        &mut stdin,
        &mut stdout,
        &json!({
            (COMMAND_FIELD): TOGGLE_INCREMENTAL
        }),
    )?;
    assert_eq!(
        toggled
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(TOGGLE_INCREMENTAL)
    );
    assert_eq!(
        toggled
            .pointer("/result/incremental")
            .and_then(Value::as_bool),
        Some(false)
    );
    let updated_config = call(&mut stdin, &mut stdout, "deslop/sessionConfig", &json!({}))?;
    assert_eq!(
        updated_config
            .pointer("/result/incremental")
            .and_then(Value::as_bool),
        Some(false)
    );

    let refreshed = execute(
        &mut stdin,
        &mut stdout,
        &json!({
            (COMMAND_FIELD): REFRESH_REPORT
        }),
    )?;
    assert_eq!(
        refreshed
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(REFRESH_REPORT)
    );
    assert_eq!(
        refreshed
            .pointer("/result/generation")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert!(refreshed.pointer("/result/clustersAdded").is_some());
    assert!(refreshed.pointer("/result/clustersRemoved").is_some());
    assert!(refreshed.pointer("/result/clustersUpdated").is_some());

    let models = execute(
        &mut stdin,
        &mut stdout,
        &json!({
            (COMMAND_FIELD): PICK_EMBEDDING_MODEL
        }),
    )?;
    assert_eq!(
        models
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(PICK_EMBEDDING_MODEL)
    );
    // [REMOVE-STUB] Production listing only carries Ollama-provided
    // entries — when Ollama is unreachable the list is empty, when it
    // is running every row reports `provider_id == "ollama"`. Either
    // way no `stub` row may appear.
    let models_array = models
        .pointer("/result/models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("models field missing: {models}"))?;
    for entry in models_array {
        assert_ne!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("stub"),
            "production payload must not include the deterministic stub: {entry}",
        );
        assert_eq!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("ollama"),
            "production listing must only expose ollama-provided models: {entry}",
        );
    }

    let _ = child.kill();
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn execute_command_handlers_run_in_process_for_coverage() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let (mut service, mut socket) = in_process_lsp(workspace.path());
    let init = initialize_in_process(&mut service).await?;
    assert_advertised_commands(&init)?;
    assert_open_report_command(&mut service, &mut socket).await?;
    assert_render_html_report_command(&mut service, &mut socket).await?;
    assert_report_json_command(&mut service, &mut socket).await?;
    assert_open_cluster_command(&mut service, &mut socket).await?;
    assert_toggle_incremental_command(&mut service, &mut socket).await?;
    assert_pick_embedding_model_command(&mut service, &mut socket).await?;
    assert_open_cluster_invalid_id(&mut service, &mut socket).await?;
    assert_unknown_command(&mut service, &mut socket).await?;
    assert_refresh_report_command(&mut service, &mut socket).await?;
    Ok(())
}

fn assert_advertised_commands(init: &Value) -> Result<()> {
    let commands = advertised_commands(init)?;
    assert_eq!(commands.len(), 7, "unexpected command list: {commands:?}");
    assert!(commands.contains(&REFRESH_REPORT.to_owned()));
    assert!(commands.contains(&OPEN_CLUSTER.to_owned()));
    assert!(commands.contains(&OPEN_REPORT.to_owned()));
    assert!(commands.contains(&"deslop.lsp.renderHtmlReport".to_owned()));
    assert!(commands.contains(&"deslop.lsp.reportJson".to_owned()));
    assert!(commands.contains(&PICK_EMBEDDING_MODEL.to_owned()));
    assert!(commands.contains(&TOGGLE_INCREMENTAL.to_owned()));
    Ok(())
}

async fn assert_open_report_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) =
        execute_in_process(service, socket, json!({ (COMMAND_FIELD): OPEN_REPORT })).await?;
    assert_eq!(shows.len(), 1, "expected one showDocument");
    let show = only_show_request(&shows)?;
    assert_eq!(show_uri(show)?, "deslop://report");
    assert_eq!(show_take_focus(show), Some(true));
    assert_eq!(show_external(show), Some(false));
    assert_json_str(&response, "/command", OPEN_REPORT);
    Ok(())
}

async fn assert_render_html_report_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): "deslop.lsp.renderHtmlReport" }),
    )
    .await?;
    assert!(shows.is_empty(), "render must not open documents");
    let html = response
        .as_str()
        .ok_or_else(|| anyhow!("renderHtmlReport must return an HTML string: {response}"))?;
    assert_well_formed_report_html(html);
    assert_report_shows_real_clusters(html);
    Ok(())
}

/// Exercises `deslop.lsp.reportJson`: the structured report the `JetBrains`
/// tool window groups natively when the IDE has no embedded browser. Locks
/// the contract that native surface depends on — a JSON string carrying a
/// worst-first `clusters` array, each cluster with a clone-type `bucket` and
/// located `occurrences` — and the lean wire shape (no `schema_doc`).
async fn assert_report_json_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): "deslop.lsp.reportJson" }),
    )
    .await?;
    assert!(shows.is_empty(), "reportJson must not open documents");
    let json = response
        .as_str()
        .ok_or_else(|| anyhow!("reportJson must return a JSON string: {response}"))?;
    let report: Value = serde_json::from_str(json)?;
    assert_report_json_shape(&report)
}

/// Asserts the native-tree wire contract on a parsed `reportJson` payload.
fn assert_report_json_shape(report: &Value) -> Result<()> {
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("reportJson needs a clusters array: {report}"))?;
    let first = clusters
        .first()
        .ok_or_else(|| anyhow!("csharp-small must yield clusters: {report}"))?;
    let bucket = first.get("bucket").and_then(Value::as_str);
    assert!(
        bucket.is_some(),
        "cluster needs a clone-type bucket: {first}"
    );
    let path = first.pointer("/occurrences/0/path").and_then(Value::as_str);
    assert!(path.is_some(), "cluster occurrence needs a path: {first}");
    let schema_doc = report.get("schema_doc").and_then(Value::as_str);
    assert!(
        schema_doc.map_or(true, str::is_empty),
        "reportJson must strip schema_doc for the lean wire shape",
    );
    Ok(())
}

/// Asserts the rendered report is not an empty styled shell but carries the
/// fixture's real worst-offender: the `csharp-small` clone is two renamed copies
/// of one method, so a populated `cluster-card` element — with a title, the
/// two-occurrence count, and a real source path — must appear. This is the exact
/// content every editor panel (the VS Code webview, the IDE tool window) displays,
/// so it guards against the "panel shows nothing" regression at the data source
/// ([OUTPUT-HUMAN-HTML]).
fn assert_report_shows_real_clusters(html: &str) {
    assert!(
        html.contains("<article class=\"cluster-card"),
        "report must render a populated cluster-card element, not just the stylesheet rule"
    );
    assert!(
        html.contains("cluster-card__title"),
        "the populated cluster card must carry its title heading"
    );
    assert!(
        html.contains("in 2 places"),
        "the csharp-small clone spans exactly two occurrences: {}",
        &html[..html.len().min(160)]
    );
    assert!(
        html.contains("Alpha.cs") || html.contains("Beta.cs"),
        "the card must reference a real fixture occurrence (Alpha.cs / Beta.cs)"
    );
}

/// Asserts the rendered string is the self-contained, CSS-bearing HTML
/// document the editor clients open in a browser tab ([OUTPUT-HUMAN-HTML]).
fn assert_well_formed_report_html(html: &str) {
    assert!(
        html.starts_with("<!doctype html"),
        "report must be a full HTML document: {}",
        &html[..html.len().min(48)]
    );
    assert!(
        html.contains("data-theme=\"dark\""),
        "report must carry the dark-theme attribute the design system targets"
    );
    assert!(
        html.contains("class=\"report-shell\""),
        "report body must use the styled report-shell container"
    );
    assert!(
        html.contains(".report-shell{max-width:"),
        "report must inline its stylesheet so it renders offline with CSS"
    );
    assert!(
        html.contains(".cluster-card{"),
        "report must inline the cluster-card styling"
    );
    assert!(
        html.trim_end().ends_with("</html>"),
        "report must be a well-formed, fully-closed document"
    );
}

async fn assert_open_cluster_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): OPEN_CLUSTER, "arguments": ["abc123"] }),
    )
    .await?;
    assert_eq!(shows.len(), 1, "expected one cluster open");
    assert_eq!(
        show_uri(only_show_request(&shows)?)?,
        "deslop://cluster/abc123"
    );
    assert_json_bool(&response, "/shown", true);
    Ok(())
}

async fn assert_toggle_incremental_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): TOGGLE_INCREMENTAL }),
    )
    .await?;
    assert!(shows.is_empty(), "toggle must not open documents");
    assert_json_bool(&response, "/incremental", false);
    Ok(())
}

async fn assert_pick_embedding_model_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): PICK_EMBEDDING_MODEL }),
    )
    .await?;
    assert!(shows.is_empty(), "model picker must not open documents");
    // [REMOVE-STUB] When Ollama is unreachable in CI the models list
    // is empty; when Ollama is reachable every row reports `provider_id
    // == "ollama"`. The deterministic stub must never appear.
    let models = response
        .pointer("/models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("/models field missing: {response}"))?;
    for entry in models {
        assert_ne!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("stub"),
            "pickEmbeddingModel response must not include the stub: {entry}",
        );
        assert_eq!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("ollama"),
            "pickEmbeddingModel response must only expose ollama rows: {entry}",
        );
    }
    Ok(())
}

async fn assert_open_cluster_invalid_id(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, requests) = execute_raw_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): OPEN_CLUSTER, "arguments": ["bad/id"] }),
    )
    .await?;
    assert!(
        requests.is_empty(),
        "invalid cluster command must not contact the client"
    );
    let error = response_error(response)?;
    assert_eq!(error.code, ErrorCode::InvalidParams);
    assert_eq!(
        error.data,
        Some(Value::String(
            "deslop.lsp.openCluster requires a cluster id".to_owned()
        ))
    );
    Ok(())
}

async fn assert_unknown_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, requests) = execute_raw_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): "deslop.unknown" }),
    )
    .await?;
    assert!(
        requests.is_empty(),
        "unknown command must not contact the client"
    );
    assert_eq!(response_error(response)?.code, ErrorCode::MethodNotFound);
    Ok(())
}

async fn assert_refresh_report_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) =
        execute_in_process(service, socket, json!({ (COMMAND_FIELD): REFRESH_REPORT })).await?;
    assert!(shows.is_empty(), "refresh must not open documents");
    assert_json_str(&response, "/command", REFRESH_REPORT);
    assert!(response.pointer("/generation").is_some());
    assert!(response.pointer("/clustersAdded").is_some());
    Ok(())
}

fn advertised_commands(response: &Value) -> Result<Vec<String>> {
    let result = response.get("result").unwrap_or(response);
    result
        .pointer("/capabilities/executeCommandProvider/commands")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("missing executeCommandProvider commands: {response}"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("non-string command in capabilities: {value}"))
        })
        .collect()
}

fn assert_json_str(value: &Value, pointer: &str, expected: &str) {
    assert_eq!(
        value.pointer(pointer).and_then(Value::as_str),
        Some(expected)
    );
}

fn assert_json_bool(value: &Value, pointer: &str, expected: bool) {
    assert_eq!(
        value.pointer(pointer).and_then(Value::as_bool),
        Some(expected)
    );
}

fn in_process_lsp(workspace_root: &std::path::Path) -> (LspService<LspBackend>, ClientSocket) {
    let root = workspace_root.to_path_buf();
    LspService::build(
        move |client| match LspBackend::new_with_defaults(client, root.clone(), 30) {
            Ok(backend) => backend,
            Err(error) => {
                tracing::error!(%error, "in-process lsp test backend failed to initialise");
                std::process::exit(1);
            }
        },
    )
    .finish()
}

async fn initialize_in_process(service: &mut LspService<LspBackend>) -> Result<Value> {
    let request = Request::build("initialize")
        .params(json!({ "capabilities": {} }))
        .id(1_i64)
        .finish();
    response_result(call_service(service, request).await?)
}

async fn execute_in_process(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
    params: Value,
) -> Result<(Value, Vec<Value>)> {
    let (response, client_requests) = execute_raw_in_process(service, socket, params).await?;
    Ok((response_result(response)?, client_requests))
}

async fn execute_raw_in_process(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
    params: Value,
) -> Result<(Option<Response>, Vec<Value>)> {
    let request = Request::build(EXECUTE_COMMAND)
        .params(params)
        .id(2_i64)
        .finish();
    call_service_with_client(service, socket, request).await
}

async fn call_service(
    service: &mut LspService<LspBackend>,
    request: Request,
) -> Result<Option<Response>> {
    futures::future::poll_fn(|cx| service.poll_ready(cx)).await?;
    Ok(service.call(request).await?)
}

async fn call_service_with_client(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
    request: Request,
) -> Result<(Option<Response>, Vec<Value>)> {
    let service_call = async {
        futures::future::poll_fn(|cx| service.poll_ready(cx)).await?;
        Ok::<Option<Response>, anyhow::Error>(service.call(request).await?)
    }
    .fuse();
    futures::pin_mut!(service_call);
    let mut client_requests = Vec::new();

    loop {
        futures::select! {
            response = service_call => return Ok((response?, client_requests)),
            client_request = socket.next().fuse() => {
                let request = client_request
                    .ok_or_else(|| anyhow!("client socket closed before response"))?;
                client_requests.push(client_request_frame(&request));
                respond_to_client_request(socket, request).await?;
            }
        }
    }
}

fn response_result(response: Option<Response>) -> Result<Value> {
    let response = response.ok_or_else(|| anyhow!("expected JSON-RPC response"))?;
    let (_id, body) = response.into_parts();
    Ok(body?)
}

fn response_error(response: Option<Response>) -> Result<Error> {
    let response = response.ok_or_else(|| anyhow!("expected JSON-RPC response"))?;
    let (_id, body) = response.into_parts();
    match body {
        Ok(value) => Err(anyhow!("expected JSON-RPC error, got result: {value}")),
        Err(error) => Ok(error),
    }
}

fn client_request_frame(request: &Request) -> Value {
    json!({
        "method": request.method(),
        "params": request.params().cloned().unwrap_or(Value::Null),
    })
}

async fn respond_to_client_request(socket: &mut ClientSocket, request: Request) -> Result<()> {
    let (method, id, _params) = request.into_parts();
    let Some(id) = id else {
        return Ok(());
    };
    let result = if method.as_ref() == "window/showDocument" {
        json!({ "success": true })
    } else {
        json!({})
    };
    Ok(socket.send(Response::from_ok(id, result)).await?)
}

fn execute(
    stdin: &mut ChildStdin,
    stdout: &mut std::io::BufReader<ChildStdout>,
    params: &Value,
) -> Result<Value> {
    call(stdin, stdout, EXECUTE_COMMAND, params)
}

fn call_with_show_document_response(
    stdin: &mut ChildStdin,
    stdout: &mut std::io::BufReader<ChildStdout>,
    params: &Value,
) -> Result<(Value, Vec<Value>)> {
    let (id, payload) = request(EXECUTE_COMMAND, params)?;
    write_frame(stdin, &payload)?;
    let mut show_requests = Vec::new();
    loop {
        let frame = read_frame(stdout)?;
        if frame.get("id").and_then(Value::as_i64) == Some(id) {
            return Ok((frame, show_requests));
        }
        if frame.get("method").and_then(Value::as_str) == Some("window/showDocument") {
            respond_to_show_document(stdin, &frame)?;
            show_requests.push(frame);
        }
    }
}

fn respond_to_show_document(stdin: &mut ChildStdin, frame: &Value) -> Result<()> {
    let id = frame
        .get("id")
        .ok_or_else(|| anyhow!("showDocument request missing id: {frame}"))?;
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": { "success": true }
    });
    write_frame(stdin, &serde_json::to_string(&response)?)?;
    Ok(())
}

fn only_show_request(requests: &[Value]) -> Result<&Value> {
    requests
        .first()
        .ok_or_else(|| anyhow!("expected one showDocument request"))
}

fn show_uri(frame: &Value) -> Result<&str> {
    frame
        .pointer("/params/uri")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("showDocument URI missing: {frame}"))
}

fn show_take_focus(frame: &Value) -> Option<bool> {
    frame.pointer("/params/takeFocus").and_then(Value::as_bool)
}

fn show_external(frame: &Value) -> Option<bool> {
    frame.pointer("/params/external").and_then(Value::as_bool)
}
