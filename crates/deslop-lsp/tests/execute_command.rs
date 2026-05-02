//! E2E coverage for [LSP-COMMANDS] `workspace/executeCommand`.
//!
//! Drives the real `deslop-lsp` binary over stdio and responds to
//! client-bound `window/showDocument` requests so command handlers run
//! through the same transport a real editor uses.

mod common;

use std::process::{ChildStdin, ChildStdout};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{
    call, copy_fixture, handshake, read_frame, request, spawn_lsp, take_io, write_frame,
};

const EXECUTE_COMMAND: &str = "workspace/executeCommand";

#[test]
fn execute_command_provider_advertises_and_opens_virtual_documents() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let init = handshake(&mut stdin, &mut stdout)?;
    let commands = advertised_commands(&init)?;

    assert_eq!(commands.len(), 5, "unexpected command list: {commands:?}");
    assert!(commands.contains(&"deslop.refreshReport".to_owned()));
    assert!(commands.contains(&"deslop.openCluster".to_owned()));
    assert!(commands.contains(&"deslop.openReport".to_owned()));
    assert!(commands.contains(&"deslop.pickEmbeddingModel".to_owned()));
    assert!(commands.contains(&"deslop.toggleIncremental".to_owned()));

    let (report_response, report_shows) = call_with_show_document_response(
        &mut stdin,
        &mut stdout,
        &json!({ "command": "deslop.openReport" }),
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
        &json!({ "command": "deslop.openCluster", "arguments": ["abc123"] }),
    )?;
    assert_eq!(cluster_shows.len(), 1, "expected one cluster document open");
    let cluster_show = only_show_request(&cluster_shows)?;
    assert_eq!(show_uri(cluster_show)?, "deslop://cluster/abc123");
    assert_eq!(
        cluster_response
            .pointer("/result/command")
            .and_then(Value::as_str),
        Some("deslop.openCluster")
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
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
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
            "command": "deslop.toggleIncremental"
        }),
    )?;
    assert_eq!(
        toggled.pointer("/result/command").and_then(Value::as_str),
        Some("deslop.toggleIncremental")
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
            "command": "deslop.refreshReport"
        }),
    )?;
    assert_eq!(
        refreshed.pointer("/result/command").and_then(Value::as_str),
        Some("deslop.refreshReport")
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
            "command": "deslop.pickEmbeddingModel"
        }),
    )?;
    assert_eq!(
        models.pointer("/result/command").and_then(Value::as_str),
        Some("deslop.pickEmbeddingModel")
    );
    let first = models
        .pointer("/result/models/0")
        .ok_or_else(|| anyhow!("model list is empty: {models}"))?;
    assert_eq!(
        first.get("provider_id").and_then(Value::as_str),
        Some("stub")
    );
    assert_eq!(
        first.get("model_id").and_then(Value::as_str),
        Some("blake3-stub")
    );
    assert_eq!(first.get("reachable").and_then(Value::as_bool), Some(true));

    let _ = child.kill();
    Ok(())
}

fn advertised_commands(response: &Value) -> Result<Vec<String>> {
    response
        .pointer("/result/capabilities/executeCommandProvider/commands")
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
