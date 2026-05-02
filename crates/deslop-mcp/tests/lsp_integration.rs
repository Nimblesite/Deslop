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
        .arg("--min-nodes")
        .arg("15")
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

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
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

    let mut mcp = McpHandle::spawn(workspace.path())?;
    let _init = mcp.request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "phase5-e2e", "version": "0.1.0" }
        }),
    )?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "find-similar",
            "arguments": {
                "snippet": "namespace N { class C { void M(int x) { return; } } }",
                "language": "csharp",
                "top_n": 5
            }
        }),
    )?;

    ensure!(
        response.get("error").is_none(),
        "find-similar must succeed when the LSP is live, got: {response}"
    );
    let result = response
        .get("result")
        .ok_or_else(|| anyhow!("response missing result: {response}"))?;
    let structured = result
        .get("structuredContent")
        .ok_or_else(|| anyhow!("response missing structuredContent: {response}"))?;
    ensure!(
        structured.get("clusters").is_some(),
        "find-similar result must contain clusters array: {response}"
    );
    ensure!(
        structured.get("below_min_nodes").is_some(),
        "find-similar result must contain below_min_nodes flag: {response}"
    );
    Ok(())
}

/// [MCP-IPC-CLIENT] When the LSP is not running, MCP must fail fast
/// with the specific `LspNotRunning` JSON-RPC error code so agents
/// can offer to start the LSP. This is the negative case the existing
/// MCP tests cover, repeated here to lock the contract alongside the
/// success path above.
#[test]
fn find_similar_returns_lsp_not_running_when_socket_absent() -> Result<()> {
    let workspace = TempDir::new()?;
    fs::create_dir_all(workspace.path().join(".deslop-cache"))?;
    fs::write(
        workspace.path().join(".deslop-cache/live-report.json"),
        br#"{"report_schema_version": 1, "tool_version": "0.1.0", "min_nodes": 15, "files_analysed": 0, "languages": [], "clusters": [], "cache_stats": {"hits": 0, "misses": 0}, "embedding_provenance": null}"#,
    )?;

    let mut mcp = McpHandle::spawn(workspace.path())?;
    let _init = mcp.request(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "phase5-e2e", "version": "0.1.0" }
        }),
    )?;

    let response = mcp.request(
        "tools/call",
        json!({
            "name": "find-similar",
            "arguments": {
                "snippet": "namespace N { class C { void M(int x) { return; } } }",
                "language": "csharp",
                "top_n": 5
            }
        }),
    )?;

    let code = response
        .pointer("/error/code")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("expected error envelope, got: {response}"))?;
    ensure!(
        code == -32004,
        "find-similar without LSP must return LspNotRunning (-32004), got code {code}: {response}"
    );
    Ok(())
}
