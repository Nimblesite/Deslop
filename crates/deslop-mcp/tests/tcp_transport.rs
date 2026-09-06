//! E2E proof for the TCP loopback IPC transport ([LIVE-IPC-TCP],
//! [MCP-IPC-DISCOVERY]).
//!
//! Spawns the real `deslop-lsp` with `--ipc-transport tcp`, waits for
//! the `.deslop/cache/deslop.port` discovery record, then drives the
//! real `deslop-mcp` over stdio. Every assertion exercises the exact
//! code path Windows uses in production — no Unix sockets appear
//! anywhere in this file, so the suite runs on every platform,
//! including the Windows CI check leg.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
};

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    copied_fixture, initialized_mcp, spawn_lsp_with_args, structured_content, wait_for_path,
    ChildKillOnDrop, SOCKET_TIMEOUT,
};

/// Reads and validates the discovery record, returning `(port, token)`.
fn read_discovery_record(workspace: &std::path::Path) -> Result<(u16, String)> {
    let port_file = workspace.join(".deslop/cache/deslop.port");
    let record: Value =
        serde_json::from_slice(&fs::read(&port_file).context("read discovery record")?)
            .context("parse discovery record")?;
    let port = record
        .get("port")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("discovery record missing port: {record}"))?;
    let port = u16::try_from(port).context("port out of range")?;
    ensure!(port > 0, "discovery record must carry a bound port");
    let token = record
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("discovery record missing token: {record}"))?;
    ensure!(
        token.len() == 64,
        "token must be a 64-char hex secret, got {} chars",
        token.len()
    );
    Ok((port, token.to_owned()))
}

/// Sends `payload` lines over a fresh TCP connection and returns the
/// first response line (empty when the server dropped the connection).
fn raw_tcp_exchange(port: u16, lines: &[&str]) -> Result<String> {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).context("tcp connect")?;
    for line in lines {
        stream.write_all(line.as_bytes())?;
        stream.write_all(b"\n")?;
    }
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    let _bytes = reader.read_line(&mut response)?;
    Ok(response)
}

/// [LIVE-IPC-TCP] The full MCP tool chain — report read, compute, and
/// refresh — must work over the TCP transport exactly as it does over
/// the Unix socket, with no socket file present at all.
#[test]
fn mcp_tools_work_over_tcp_transport() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_with_args(workspace.path(), &["--ipc-transport", "tcp"])?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let port_file = workspace.path().join(".deslop/cache/deslop.port");
    wait_for_path(&port_file, SOCKET_TIMEOUT).context("wait for discovery record")?;
    ensure!(
        !workspace.path().join(".deslop/cache/deslop.sock").exists(),
        "TCP transport must not create a Unix socket"
    );
    let _record = read_discovery_record(workspace.path())?;

    let mut mcp = initialized_mcp(workspace.path())?;

    let response = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 3 } }),
    )?;
    let offenders = structured_content(&response, "top-offenders")?;
    ensure!(
        offenders
            .get("total_clusters")
            .and_then(Value::as_u64)
            .is_some(),
        "top-offenders over TCP must return the live report shape: {response}"
    );

    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "find-similar",
            "arguments": {
                "snippet": include_str!("fixtures/csharp-mcp/Alpha.cs"),
                "language": "csharp",
                "top_n": 5
            }
        }),
    )?;
    let similar = structured_content(&response, "find-similar")?;
    let clusters = similar
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("clusters must be an array: {response}"))?;
    ensure!(
        !clusters.is_empty(),
        "find-similar over TCP must return live LSP clusters: {response}"
    );

    let response = mcp.request("tools/call", &json!({ "name": "rescan", "arguments": {} }))?;
    let rescan = structured_content(&response, "rescan")?;
    ensure!(
        rescan.get("generation").and_then(Value::as_u64).is_some(),
        "rescan over TCP must return the refresh summary: {response}"
    );
    Ok(())
}

/// [LIVE-IPC-TCP] The server must drop connections presenting a wrong
/// token before any JSON-RPC is exchanged, and serve identical
/// requests once the published token is presented.
#[test]
fn tcp_token_gates_the_connection() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_with_args(workspace.path(), &["--ipc-transport", "tcp"])?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let port_file = workspace.path().join(".deslop/cache/deslop.port");
    wait_for_path(&port_file, SOCKET_TIMEOUT).context("wait for discovery record")?;
    let (port, token) = read_discovery_record(workspace.path())?;

    let request = r#"{"jsonrpc":"2.0","id":1,"method":"session/config","params":{}}"#;

    let rejected = raw_tcp_exchange(port, &["not-the-real-token", request])?;
    ensure!(
        rejected.is_empty(),
        "wrong token must close the connection without a response, got: {rejected}"
    );

    let served = raw_tcp_exchange(port, &[token.as_str(), request])?;
    let frame: Value = serde_json::from_str(served.trim())
        .with_context(|| format!("authenticated request must get JSON back: {served}"))?;
    let root = frame
        .pointer("/result/workspace_root")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("session/config must return workspace_root: {frame}"))?;
    ensure!(
        !root.is_empty(),
        "session/config over authenticated TCP must carry the workspace root"
    );
    Ok(())
}

/// [MCP-IPC-DISCOVERY] A stale discovery record (the LSP died without
/// cleanup, port no longer listening) must surface as the actionable
/// "LSP is not running" error — never garbage or a hang.
#[test]
fn stale_discovery_record_reports_lsp_not_running() -> Result<()> {
    let workspace = copied_fixture()?;
    let cache_dir = workspace.path().join(".deslop/cache");
    fs::create_dir_all(&cache_dir)?;
    // Reserve a port the OS will not immediately reuse, then free it so
    // nothing is listening when MCP dials.
    let dead_port = {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.local_addr()?.port()
    };
    fs::write(
        cache_dir.join("deslop.port"),
        format!(
            r#"{{"port":{dead_port},"token":"0000000000000000000000000000000000000000000000000000000000000000"}}"#
        ),
    )?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 3 } }),
    )?;
    let message = response
        .pointer("/error/message")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("stale record must produce an error: {response}"))?;
    ensure!(
        message.contains("LSP is not running"),
        "stale discovery record must map to the actionable LSP-down error: {message}"
    );
    Ok(())
}
