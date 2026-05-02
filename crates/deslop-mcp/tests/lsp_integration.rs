//! End-to-end test for the LSP+MCP side-by-side architecture
//! ([MCP-WHY-LIVE], [MCP-IPC-CLIENT]).
//!
//! Spawns the real `deslop-lsp` binary, waits for its IPC socket at
//! `.deslop-cache/deslop.sock`, then spawns `deslop-mcp` against the
//! same workspace and calls `find-similar` over the MCP wire. The
//! call traverses MCP → IPC socket → LSP → live analysis → IPC reply
//! → MCP response — the same chain agents will hit in production.
//!
//! Without this test the MCP `find-similar` path is only exercised in
//! the `LspNotRunning` error case, leaving every success branch in
//! `tools/handlers.rs` uncovered.

#![cfg(unix)]

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use tempfile::TempDir;

const SOCKET_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Copies the MCP fixture into a writable temp dir so the LSP can
/// write to `.deslop-cache/`.
fn copied_fixture() -> Result<TempDir> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/csharp-mcp");
    let dst = TempDir::new()?;
    copy_dir(&src, dst.path())?;
    // Remove any pre-committed cache so the LSP writes a fresh one.
    let cache = dst.path().join(".deslop-cache");
    if cache.exists() {
        fs::remove_dir_all(&cache).context("clear cache dir")?;
    }
    Ok(dst)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            let _bytes = fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Spawns the LSP binary, drives the LSP `initialize`+`initialized`
/// handshake, and returns the running process.
fn spawn_lsp_and_initialize(root: &Path) -> Result<Child> {
    let bin = cargo_bin("deslop-lsp");
    let mut child = Command::new(bin)
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn deslop-lsp")?;
    let mut stdin = child.stdin.take().context("lsp stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().context("lsp stdout")?);
    lsp_handshake(&mut stdin, &mut stdout).context("lsp handshake")?;
    // Park stdin/stdout inside the child so the LSP keeps running for
    // the rest of the test. Re-attach them to the Child handle.
    child.stdin = Some(stdin);
    child.stdout = Some(stdout.into_inner());
    Ok(child)
}

fn lsp_handshake(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) -> Result<()> {
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "processId": null, "rootUri": null, "capabilities": {} }
    });
    write_lsp_frame(stdin, &serde_json::to_string(&init)?)?;
    let _response = read_lsp_frame(stdout)?;
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_lsp_frame(stdin, &serde_json::to_string(&initialized)?)
}

fn write_lsp_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(payload.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

fn read_lsp_frame(reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let _read = reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = Some(rest.trim().parse::<usize>()?);
        }
    }
    let length = content_length.ok_or_else(|| anyhow!("missing Content-Length"))?;
    let mut buf = vec![0_u8; length];
    reader.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

fn wait_for_path(path: &Path, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    loop {
        if path.exists() {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            return Err(anyhow!(
                "timed out after {:?} waiting for {}",
                timeout,
                path.display()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

struct McpHandle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpHandle {
    fn spawn(root: &Path) -> Result<Self> {
        let bin = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut child = Command::new(bin)
            .arg("--root")
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn deslop-mcp")?;
        let stdin = child.stdin.take().context("mcp stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("mcp stdout")?);
        Ok(Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        })
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value> {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut bytes = serde_json::to_vec(&payload)?;
        bytes.push(b'\n');
        self.stdin.write_all(&bytes)?;
        self.stdin.flush()?;
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line)?;
            ensure!(read > 0, "mcp stdout closed unexpectedly");
            let frame: Value = serde_json::from_str(line.trim())
                .with_context(|| format!("invalid mcp frame: {line}"))?;
            if frame.get("id").and_then(Value::as_i64) == Some(id) {
                return Ok(frame);
            }
            // Skip notifications between request and response.
            if frame.get("method").is_none() {
                return Err(anyhow!("unexpected frame without id: {frame}"));
            }
        }
    }
}

impl Drop for McpHandle {
    fn drop(&mut self) {
        let _kill = self.child.kill();
        let _wait = self.child.wait();
    }
}

struct ChildKillOnDrop(Child);

impl Drop for ChildKillOnDrop {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// [MCP-IPC-CLIENT] When the LSP is running, MCP must delegate
/// `find-similar` to the LSP IPC socket and return real cluster data
/// — never `LspNotRunning`. This is the success path that lives
/// behind the IPC chain in production.
#[test]
fn find_similar_via_mcp_delegates_to_running_lsp() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let state_file = workspace.path().join(".deslop-cache/live-report.json");
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for state file")?;

    let mut mcp = initialized_mcp(workspace.path())?;

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

    let structured = structured_content(&response, "find-similar")?;
    let clusters = structured
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("clusters must be an array: {response}"))?;
    ensure!(
        !clusters.is_empty(),
        "find-similar must return live LSP clusters: {response}"
    );
    ensure!(
        structured.get("below_min_nodes") == Some(&Value::Bool(false)),
        "fixture snippet must be large enough to fingerprint: {response}"
    );
    Ok(())
}

/// [MCP-IPC-CLIENT] `list-embedding-models` is another compute tool:
/// MCP must delegate it to the live LSP IPC socket and expose model
/// metadata through the normal MCP tool result envelope.
#[test]
fn list_embedding_models_via_mcp_delegates_to_running_lsp() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "list-embedding-models", "arguments": {} }),
    )?;
    let structured = structured_content(&response, "list-embedding-models")?;
    let models = structured
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("models must be an array: {response}"))?;
    ensure!(
        models
            .iter()
            .any(|model| model.get("name") == Some(&json!("blake3-stub"))),
        "list-embedding-models must include the built-in stub model: {response}"
    );
    Ok(())
}

/// [MCP-IPC-CLIENT] Agent `rescan` must ask the running LSP to execute
/// `deslop.lsp.refreshReport`, then return top offenders from the
/// refreshed state file.
#[test]
fn rescan_via_mcp_triggers_lsp_reanalysis() -> Result<()> {
    let workspace = copied_fixture()?;
    let beta = workspace.path().join("Beta.cs");
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let state_file = workspace.path().join(".deslop-cache/live-report.json");
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for state file")?;
    let initial_bytes = fs::read(&state_file)?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let before = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 100 } }),
    )?;
    let before_structured = structured_content(&before, "top-offenders")?;
    let before_count = before_structured
        .get("total_clusters")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    ensure!(
        before_count > 0,
        "fixture must start with at least one cluster: {before}"
    );

    fs::write(
        &beta,
        b"namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;

    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "rescan",
            "arguments": {
                "paths": [beta.to_string_lossy().into_owned()],
                "n": 100
            }
        }),
    )?;
    let after = structured_content(&response, "rescan")?;
    let after_count = after
        .get("total_clusters")
        .and_then(Value::as_u64)
        .unwrap_or(before_count);
    ensure!(
        after_count < before_count,
        "rescan must trigger LSP re-analysis and drop the edited Beta.cs clone: {before_count} -> {after_count}; response {response}"
    );
    ensure!(
        after.get("n").and_then(Value::as_u64) == Some(100),
        "rescan must echo the requested top-offenders count: {response}"
    );
    let clusters = after
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("rescan clusters must be an array: {response}"))?;
    ensure!(
        clusters.len() as u64 == after_count,
        "with n=100, rescan clusters must match total_clusters: {response}"
    );

    let updated_bytes = fs::read(&state_file)?;
    ensure!(
        updated_bytes != initial_bytes,
        "MCP rescan must cause the LSP to rewrite live-report.json"
    );
    let state: Value = serde_json::from_slice(&updated_bytes)?;
    let state_count = state
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(0_u64, |items| items.len() as u64);
    ensure!(
        state_count == after_count,
        "MCP rescan response must be loaded from the refreshed LSP state file: response {after_count}, state {state_count}"
    );
    Ok(())
}

fn initialized_mcp(root: &Path) -> Result<McpHandle> {
    let mut mcp = McpHandle::spawn(root)?;
    let response = mcp.request(
        "initialize",
        &json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "phase5-e2e", "version": "0.1.0" }
        }),
    )?;
    ensure!(
        response.get("error").is_none(),
        "MCP initialize failed: {response}"
    );
    Ok(mcp)
}

fn structured_content(response: &Value, tool: &str) -> Result<Value> {
    ensure!(
        response.get("error").is_none(),
        "{tool} must succeed when the LSP is live, got: {response}"
    );
    response
        .pointer("/result/structuredContent")
        .cloned()
        .ok_or_else(|| anyhow!("response missing structuredContent: {response}"))
}
