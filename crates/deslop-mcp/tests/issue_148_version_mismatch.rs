//! [Deslop#148] Surfaces an actionable LSP/MCP version-mismatch hint
//! when the LSP responds to a tool-driving IPC method with JSON-RPC
//! `-32601 method not found`.
//!
//! Without this guard, agents see `MCP error -32004: ipc transport
//! failure: ipc rpc error: {...,"code":-32601,...}` and assume the
//! server is "offline". The new path names the rejected method, the
//! `-32601` code, and points at the VSIX so the fix is discoverable.

#![cfg(unix)]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    thread,
};

use anyhow::{ensure, Result};
use serde_json::{json, Value};
use tempfile::TempDir;

mod common;
use common::{error_and_message, initialized_mcp};

#[test]
fn issue_148_top_offenders_reports_version_mismatch_when_lsp_rejects_report_get() -> Result<()> {
    let workspace = TempDir::new()?;
    fs::create_dir_all(workspace.path().join(".deslop/cache"))?;
    let socket = workspace.path().join(".deslop/cache/deslop.sock");
    spawn_stale_lsp(&socket)?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 5 } }),
    )?;

    let (_error, message) = error_and_message(&response)?;
    ensure!(
        message.contains("report/get"),
        "error must name the rejected method so users can match logs: {message}"
    );
    ensure!(
        message.contains("-32601"),
        "error must echo the JSON-RPC code so users can grep their LSP logs: {message}"
    );
    ensure!(
        message.contains("VSIX"),
        "error must point at the VSIX reinstall as the fix: {message}"
    );
    ensure!(
        !message.contains("ipc rpc error: {"),
        "error must not fall through to the generic catch-all: {message}"
    );
    Ok(())
}

#[test]
fn issue_148_session_config_reports_version_mismatch_when_lsp_rejects_method() -> Result<()> {
    let workspace = TempDir::new()?;
    fs::create_dir_all(workspace.path().join(".deslop/cache"))?;
    let socket = workspace.path().join(".deslop/cache/deslop.sock");
    spawn_stale_lsp(&socket)?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "session-config", "arguments": {} }),
    )?;

    let (_error, message) = error_and_message(&response)?;
    ensure!(
        message.contains("session/config"),
        "error must name the rejected method: {message}"
    );
    ensure!(
        message.contains("-32601"),
        "error must echo the JSON-RPC code: {message}"
    );
    ensure!(
        message.contains("VSIX"),
        "error must point at the VSIX reinstall: {message}"
    );
    Ok(())
}

/// Binds a Unix socket at `path` and spawns a detached accept loop that
/// answers every JSON-RPC request with `-32601 method not found`. The
/// `report/subscribe` ack is honoured so the MCP's subscribe handshake
/// does not block.
fn spawn_stale_lsp(path: &std::path::Path) -> Result<()> {
    let listener = UnixListener::bind(path)?;
    let _thread = thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(stream) = incoming else { continue };
            let _conn = thread::spawn(move || serve_one_connection(stream));
        }
    });
    Ok(())
}

fn serve_one_connection(stream: UnixStream) {
    let Ok(writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let request: Value = match serde_json::from_str(line.trim()) {
        Ok(value) => value,
        Err(_) => return,
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let frame = if method == "report/subscribe" {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "subscribed": true, "generation": 0 }
        })
    } else {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32_601, "message": "method not found" }
        })
    };
    let _written = write_frame(&writer, &frame);
}

fn write_frame(mut stream: &UnixStream, value: &Value) -> std::io::Result<()> {
    let mut payload = serde_json::to_vec(value).unwrap_or_default();
    payload.push(b'\n');
    stream.write_all(&payload)
}
