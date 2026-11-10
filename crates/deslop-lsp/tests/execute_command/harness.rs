//! Transport plumbing for the [LSP-COMMANDS] suite: the in-process
//! `LspService`, JSON-RPC request/response shaping, the stdio
//! `workspace/executeCommand` round trip and the client-bound
//! `window/showDocument` handshake.

use std::process::{ChildStdin, ChildStdout};

use anyhow::{anyhow, Result};
use deslop_lsp::LspBackend;
use futures::{FutureExt, SinkExt, StreamExt};
use serde_json::{json, Value};
use tower::Service;
use tower_lsp::{
    jsonrpc::{Error, Request, Response},
    ClientSocket, LspService,
};

use super::EXECUTE_COMMAND;
use crate::common::{call, read_frame, request, write_frame};

/// Collects every client-bound frame already queued on `socket` without
/// blocking. `call_service_with_client` returns the instant the response
/// resolves, so a notification the handler pushed just before returning is
/// still sitting in the client channel; polling it out here observes the push
/// without a sleep or a timeout.
pub(super) fn drain_client_frames(socket: &mut ClientSocket) -> Vec<Value> {
    let mut frames = Vec::new();
    while let Some(Some(request)) = socket.next().now_or_never() {
        frames.push(client_request_frame(&request));
    }
    frames
}

pub(super) fn advertised_commands(response: &Value) -> Result<Vec<String>> {
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

pub(super) fn assert_json_str(value: &Value, pointer: &str, expected: &str) {
    assert_eq!(
        value.pointer(pointer).and_then(Value::as_str),
        Some(expected)
    );
}

pub(super) fn assert_json_bool(value: &Value, pointer: &str, expected: bool) {
    assert_eq!(
        value.pointer(pointer).and_then(Value::as_bool),
        Some(expected)
    );
}

pub(super) fn in_process_lsp(
    workspace_root: &std::path::Path,
) -> (LspService<LspBackend>, ClientSocket) {
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

pub(super) async fn initialize_in_process(service: &mut LspService<LspBackend>) -> Result<Value> {
    let request = Request::build("initialize")
        .params(json!({ "capabilities": {} }))
        .id(1_i64)
        .finish();
    response_result(call_service(service, request).await?)
}

pub(super) async fn execute_in_process(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
    params: Value,
) -> Result<(Value, Vec<Value>)> {
    let (response, client_requests) = execute_raw_in_process(service, socket, params).await?;
    Ok((response_result(response)?, client_requests))
}

pub(super) async fn execute_raw_in_process(
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

pub(super) async fn call_service(
    service: &mut LspService<LspBackend>,
    request: Request,
) -> Result<Option<Response>> {
    futures::future::poll_fn(|cx| service.poll_ready(cx)).await?;
    Ok(service.call(request).await?)
}

pub(super) async fn call_service_with_client(
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

pub(super) fn response_result(response: Option<Response>) -> Result<Value> {
    let response = response.ok_or_else(|| anyhow!("expected JSON-RPC response"))?;
    let (_id, body) = response.into_parts();
    Ok(body?)
}

pub(super) fn response_error(response: Option<Response>) -> Result<Error> {
    let response = response.ok_or_else(|| anyhow!("expected JSON-RPC response"))?;
    let (_id, body) = response.into_parts();
    match body {
        Ok(value) => Err(anyhow!("expected JSON-RPC error, got result: {value}")),
        Err(error) => Ok(error),
    }
}

pub(super) fn client_request_frame(request: &Request) -> Value {
    json!({
        "method": request.method(),
        "params": request.params().cloned().unwrap_or(Value::Null),
    })
}

pub(super) async fn respond_to_client_request(
    socket: &mut ClientSocket,
    request: Request,
) -> Result<()> {
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

pub(super) fn execute(
    stdin: &mut ChildStdin,
    stdout: &mut std::io::BufReader<ChildStdout>,
    params: &Value,
) -> Result<Value> {
    call(stdin, stdout, EXECUTE_COMMAND, params)
}

pub(super) fn call_with_show_document_response(
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

pub(super) fn respond_to_show_document(stdin: &mut ChildStdin, frame: &Value) -> Result<()> {
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

pub(super) fn only_show_request(requests: &[Value]) -> Result<&Value> {
    requests
        .first()
        .ok_or_else(|| anyhow!("expected one showDocument request"))
}

pub(super) fn show_uri(frame: &Value) -> Result<&str> {
    frame
        .pointer("/params/uri")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("showDocument URI missing: {frame}"))
}

pub(super) fn show_take_focus(frame: &Value) -> Option<bool> {
    frame.pointer("/params/takeFocus").and_then(Value::as_bool)
}

pub(super) fn show_external(frame: &Value) -> Option<bool> {
    frame.pointer("/params/external").and_then(Value::as_bool)
}
