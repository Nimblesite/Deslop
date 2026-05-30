//! Shared MCP+LSP integration test helpers.
//!
//! Each integration test file is a separate binary, so cross-file reuse
//! is wired via `mod common;` declarations rather than `pub use`. Keeps
//! the per-test files small and prevents duplicated handshake / spawn
//! plumbing across LSP-backed E2E suites.

#![cfg(unix)]
#![allow(dead_code)]

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

/// Maximum wait for the LSP to create its IPC socket / state file.
pub const SOCKET_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Copies the C# MCP fixture into a writable temp dir so the LSP can
/// write to `.deslop-cache/`.
pub fn copied_fixture() -> Result<TempDir> {
    copied_fixture_named("csharp-mcp")
}

/// Copies the named `tests/fixtures/<name>` directory into a writable temp
/// dir so the LSP can write to `.deslop-cache/`. Lets MCP wire tests run
/// against any language fixture, not just the C# one.
pub fn copied_fixture_named(name: &str) -> Result<TempDir> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let dst = TempDir::new()?;
    copy_dir(&src, dst.path())?;
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
pub fn spawn_lsp_and_initialize(root: &Path) -> Result<Child> {
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

/// Polls until `path` exists, failing after `timeout`.
pub fn wait_for_path(path: &Path, timeout: Duration) -> Result<()> {
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

/// Spawned MCP child + an id-tracked JSON-RPC request loop.
pub struct McpHandle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpHandle {
    /// Spawns the `deslop-mcp` binary against `root` over stdio.
    pub fn spawn(root: &Path) -> Result<Self> {
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

    /// Sends a JSON-RPC request and reads the matching response,
    /// skipping any pushed notifications in between.
    pub fn request(&mut self, method: &str, params: &Value) -> Result<Value> {
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

/// Owned `Child` that is killed and reaped when dropped.
pub struct ChildKillOnDrop(pub Child);

impl Drop for ChildKillOnDrop {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// Spawns + initializes an MCP child against `root`.
pub fn initialized_mcp(root: &Path) -> Result<McpHandle> {
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

/// Extracts the `structuredContent` envelope from a successful tools/call.
pub fn structured_content(response: &Value, tool: &str) -> Result<Value> {
    ensure!(
        response.get("error").is_none(),
        "{tool} must succeed when the LSP is live, got: {response}"
    );
    response
        .pointer("/result/structuredContent")
        .cloned()
        .ok_or_else(|| anyhow!("response missing structuredContent: {response}"))
}
