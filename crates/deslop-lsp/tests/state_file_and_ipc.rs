//! E2E tests for the state-file and IPC surfaces added by the
//! MCP-architecture fix.
//!
//! [LIVE-STATE-FILE] The LSP writes `.deslop-cache/live-report.json`
//! after every analysis pass so the MCP can read it without running its
//! own pipeline.
//!
//! [LSP-IPC] The LSP exposes `.deslop-cache/deslop.sock` (Unix only)
//! so the MCP can delegate `duplicates/findSimilar` and
//! `embedding/listModels` without duplicating compute.

mod common;

use std::{
    fs,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Result};
use common::{call, copy_fixture, handshake, notification, spawn_lsp, take_io, write_frame};

const STATE_FILE: &str = ".deslop-cache/live-report.json";
const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// [LIVE-STATE-FILE] The LSP must write the state file during
/// initialization so the MCP has something to read immediately.
#[test]
fn state_file_exists_after_initialize() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let state_path = workspace.path().join(STATE_FILE);
    wait_for_file(&state_path, ANALYSIS_TIMEOUT)?;

    let bytes = fs::read(&state_path)?;
    let report: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| anyhow!("state file is not valid JSON: {error}"))?;
    ensure!(
        report.get("report_schema_version").is_some(),
        "state file must contain report_schema_version: {report}"
    );
    let count = cluster_count(&report);
    ensure!(
        count > 0,
        "csharp-small fixture must produce at least one cluster in the state file"
    );

    // Verify the live API cluster count matches the state file content.
    let live = call(
        &mut stdin,
        &mut stdout,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
    let live_count = live
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    ensure!(
        live_count == count,
        "state file cluster count ({count}) must match live reportGet count ({live_count})"
    );
    Ok(())
}

/// [LIVE-STATE-FILE] After a file edit triggers re-analysis, the state
/// file must be overwritten with a report reflecting the change.
#[test]
fn state_file_updated_after_file_change() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let state_path = workspace.path().join(STATE_FILE);
    wait_for_file(&state_path, ANALYSIS_TIMEOUT)?;

    let initial_bytes = fs::read(&state_path)?;
    let initial: serde_json::Value = serde_json::from_slice(&initial_bytes)?;
    let initial_count = cluster_count(&initial);
    ensure!(initial_count > 0, "initial state must have clusters");

    fs::write(
        &beta,
        b"public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n",
    )?;
    write_frame(&mut stdin, &watched_file_changed(&beta)?)?;

    wait_for_state_file_change(&state_path, &initial_bytes, ANALYSIS_TIMEOUT)?;

    let updated: serde_json::Value = serde_json::from_slice(&fs::read(&state_path)?)
        .map_err(|error| anyhow!("updated state file is not valid JSON: {error}"))?;
    let updated_count = cluster_count(&updated);
    ensure!(
        updated_count < initial_count,
        "removing Beta.cs duplicates must reduce cluster count: \
         {initial_count} → {updated_count}"
    );
    Ok(())
}

/// [LSP-IPC] The LSP must bind the Unix socket and respond to a
/// `duplicates/findSimilar` JSON-RPC request with a valid result.
#[cfg(unix)]
#[test]
fn ipc_socket_handles_find_similar_request() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "duplicates/findSimilar",
            "params": {
                "input": {
                    "kind": "snippet",
                    "snippet": "namespace N { class C { void M(int x) { return; } } }",
                    "language": "csharp"
                },
                "max_results": 5
            }
        }),
    )?;
    ensure!(
        response.get("error").is_none(),
        "findSimilar IPC request must not return a JSON-RPC error: {response}"
    );
    ensure!(
        response.get("result").is_some(),
        "findSimilar IPC response must contain a result field: {response}"
    );
    Ok(())
}

/// [LSP-IPC] The IPC socket must respond to `embedding/listModels`
/// with a non-empty array of available model entries.
#[cfg(unix)]
#[test]
fn ipc_socket_handles_list_models_request() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "embedding/listModels",
            "params": {}
        }),
    )?;
    ensure!(
        response.get("error").is_none(),
        "listModels IPC request must not return a JSON-RPC error: {response}"
    );
    let models = response
        .pointer("/result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("listModels result must be an array: {response}"))?;
    ensure!(
        !models.is_empty(),
        "listModels must return at least one model entry"
    );
    Ok(())
}

/// [LSP-IPC] MCP uses `deslop.lsp.refreshReport` over IPC to force a
/// full re-analysis after an agent edit, then reloads the state file.
#[cfg(unix)]
#[test]
fn ipc_socket_handles_refresh_report_request() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;
    let state_path = workspace.path().join(STATE_FILE);
    wait_for_file(&state_path, ANALYSIS_TIMEOUT)?;

    let initial_bytes = fs::read(&state_path)?;
    let initial: serde_json::Value = serde_json::from_slice(&initial_bytes)?;
    let initial_count = cluster_count(&initial);
    ensure!(initial_count > 0, "initial state must have clusters");

    fs::write(
        &beta,
        b"public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n",
    )?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "deslop.lsp.refreshReport",
            "params": {}
        }),
    )?;
    ensure!(
        response.get("error").is_none(),
        "refreshReport IPC request must not return a JSON-RPC error: {response}"
    );
    ensure!(
        response.pointer("/result/command") == Some(&serde_json::json!("deslop.lsp.refreshReport")),
        "refreshReport result must echo the LSP command id: {response}"
    );
    ensure!(
        response
            .pointer("/result/generation")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|generation| generation >= 2),
        "refreshReport result must advance or expose a live generation: {response}"
    );
    ensure!(
        response
            .pointer("/result/clustersRemoved")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|removed| removed >= 1),
        "refreshReport must report removed clusters after the Beta.cs edit: {response}"
    );

    let updated_bytes = fs::read(&state_path)?;
    ensure!(
        updated_bytes != initial_bytes,
        "refreshReport must rewrite the LSP state file"
    );
    let updated: serde_json::Value = serde_json::from_slice(&updated_bytes)?;
    let updated_count = cluster_count(&updated);
    ensure!(
        updated_count < initial_count,
        "refreshReport must rescan edited files and reduce cluster count: {initial_count} -> {updated_count}"
    );
    Ok(())
}

/// [LSP-IPC] Unrecognised IPC methods must return a JSON-RPC method-not-found
/// error rather than silently dropping the request.
#[cfg(unix)]
#[test]
fn ipc_socket_returns_method_not_found_for_unknown_method() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "nonexistent/method",
            "params": {}
        }),
    )?;
    let code = response
        .pointer("/error/code")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| anyhow!("unknown method must return error.code: {response}"))?;
    ensure!(
        code == -32601,
        "unknown method must return JSON-RPC -32601 method-not-found, got {code}: {response}"
    );
    Ok(())
}

struct KillOnDrop<'a>(&'a mut std::process::Child);

impl Drop for KillOnDrop<'_> {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// Polls until `path` exists or `timeout` elapses.
fn wait_for_file(path: &Path, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if path.exists() {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    Err(anyhow!("timed out waiting for {}", path.display()))
}

/// Polls until `path` contains bytes different from `previous` or `timeout` elapses.
fn wait_for_state_file_change(path: &Path, previous: &[u8], timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if let Ok(bytes) = fs::read(path) {
            if bytes != previous {
                return Ok(());
            }
        }
        if start.elapsed() >= timeout {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    Err(anyhow!("timed out waiting for state file to change"))
}

/// Sends one JSON-RPC envelope over the Unix socket and returns the response line.
#[cfg(unix)]
fn ipc_call(socket_path: &Path, req: &serde_json::Value) -> Result<serde_json::Value> {
    use std::{io::BufRead, os::unix::net::UnixStream};

    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| anyhow!("failed to connect to IPC socket: {error}"))?;
    let mut payload = serde_json::to_vec(req)?;
    payload.push(b'\n');
    stream
        .write_all(&payload)
        .map_err(|error| anyhow!("IPC write failed: {error}"))?;
    stream
        .flush()
        .map_err(|error| anyhow!("IPC flush failed: {error}"))?;
    let mut line = String::new();
    let _bytes_read = std::io::BufReader::new(&stream)
        .read_line(&mut line)
        .map_err(|error| anyhow!("IPC read failed: {error}"))?;
    serde_json::from_str(line.trim())
        .map_err(|error| anyhow!("IPC response is not valid JSON: {error} — raw: {line}"))
}

fn cluster_count(report: &serde_json::Value) -> usize {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

fn watched_file_changed(path: &Path) -> Result<String> {
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path must be absolute: {}", path.display()))?;
    notification(
        "workspace/didChangeWatchedFiles",
        &serde_json::json!({"changes": [{"uri": uri.as_str(), "type": 2}]}),
    )
}
