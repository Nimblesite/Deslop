//! Transport plumbing for the [LSP-COMMANDS] suite: the in-process
//! session, JSON-RPC request/response shaping, the stdio
//! `workspace/executeCommand` round trip and the client-bound
//! `window/showDocument` handshake.

use std::process::{ChildStdin, ChildStdout};

use anyhow::{anyhow, Result};
use deslop_lsp::{connect, jsonrpc::Error, ClientSocket, LspBackend, Session};
use futures::FutureExt;
use lspkit_server::{Message, RequestId};
use serde_json::{json, Value};

use super::EXECUTE_COMMAND;
use crate::common::{call, read_frame, request, write_frame};

/// Collects every client-bound frame already queued on `socket` without
/// blocking. `call_service_with_client` returns the instant the response
/// resolves, so a notification the handler pushed just before returning is
/// still sitting in the client channel; polling it out here observes the push
/// without a sleep or a timeout.
pub(super) fn drain_client_frames(socket: &mut ClientSocket) -> Vec<Value> {
    let mut frames = Vec::new();
    while let Some(request) = socket.try_next() {
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

/// Id of the in-process `initialize` request.
const INITIALIZE_ID: i64 = 1;
/// Id of every in-process `workspace/executeCommand` request.
const EXECUTE_ID: i64 = 2;

pub(super) fn in_process_lsp(workspace_root: &std::path::Path) -> Result<(Session, ClientSocket)> {
    let root = workspace_root.to_path_buf();
    Ok(connect(move |client| {
        LspBackend::new_with_defaults(client, root, 30)
    })?)
}

pub(super) async fn initialize_in_process(service: &mut Session) -> Result<Value> {
    let request = Message::request(
        RequestId::Number(INITIALIZE_ID),
        "initialize",
        json!({ "capabilities": {} }),
    );
    response_result(call_service(service, request).await)
}

pub(super) async fn execute_in_process(
    service: &mut Session,
    socket: &mut ClientSocket,
    params: Value,
) -> Result<(Value, Vec<Value>)> {
    let (response, client_requests) = execute_raw_in_process(service, socket, params).await?;
    Ok((response_result(response)?, client_requests))
}

pub(super) async fn execute_raw_in_process(
    service: &mut Session,
    socket: &mut ClientSocket,
    params: Value,
) -> Result<(Option<Message>, Vec<Value>)> {
    let request = Message::request(RequestId::Number(EXECUTE_ID), EXECUTE_COMMAND, params);
    call_service_with_client(service, socket, request).await
}

pub(super) async fn call_service(service: &mut Session, request: Message) -> Option<Message> {
    service.call(request).await
}

pub(super) async fn call_service_with_client(
    service: &mut Session,
    socket: &mut ClientSocket,
    request: Message,
) -> Result<(Option<Message>, Vec<Value>)> {
    let service_call = service.call(request).fuse();
    futures::pin_mut!(service_call);
    let mut client_requests = Vec::new();

    loop {
        futures::select! {
            response = service_call => return Ok((response, client_requests)),
            client_request = socket.next().fuse() => {
                let request = client_request
                    .ok_or_else(|| anyhow!("client socket closed before response"))?;
                client_requests.push(client_request_frame(&request));
                respond_to_client_request(socket, request);
            }
        }
    }
}

pub(super) fn response_result(response: Option<Message>) -> Result<Value> {
    let response = response.ok_or_else(|| anyhow!("expected JSON-RPC response"))?;
    match response.error {
        Some(error) => Err(anyhow!("JSON-RPC error {}: {}", error.code, error.message)),
        None => Ok(response.result.unwrap_or(Value::Null)),
    }
}

pub(super) fn response_error(response: Option<Message>) -> Result<Error> {
    let response = response.ok_or_else(|| anyhow!("expected JSON-RPC response"))?;
    response
        .error
        .ok_or_else(|| anyhow!("expected JSON-RPC error, got result: {:?}", response.result))
}

pub(super) fn client_request_frame(request: &Message) -> Value {
    json!({
        "method": request.method,
        "params": request.params.clone().unwrap_or(Value::Null),
    })
}

pub(super) fn respond_to_client_request(socket: &ClientSocket, request: Message) {
    let Some(id) = request.id else {
        return;
    };
    let result = if request.method.as_deref() == Some("window/showDocument") {
        json!({ "success": true })
    } else {
        json!({})
    };
    socket.answer(id, result);
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
