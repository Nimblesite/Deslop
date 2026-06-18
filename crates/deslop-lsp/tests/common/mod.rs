//! Shared E2E helpers for the `deslop-lsp` integration tests. Drives the
//! real binary over stdio with LSP framing — no mocked transport, no
//! fake service.
//!
//! Each integration binary pulls in only the subset of helpers it needs,
//! so the unused-symbol lint is silenced for this shared module (matching
//! the `deslop-core` and `deslop-mcp` test commons).

#![allow(dead_code)]

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicI64, Ordering},
};

use anyhow::{anyhow, Result};

/// JSON-RPC id counter shared across every harness call.
static NEXT_ID: AtomicI64 = AtomicI64::new(10_000);

/// Returns the absolute path to a fixture under `crates/deslop/tests/fixtures/`.
#[must_use]
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("deslop")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copies a fixture into a temp directory so the LSP can write caches.
pub fn copy_fixture(name: &str) -> Result<tempfile::TempDir> {
    let src = fixture(name);
    let dst = tempfile::tempdir()?;
    for entry in fs::read_dir(&src)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let _bytes = fs::copy(entry.path(), dst.path().join(entry.file_name()))?;
        }
    }
    Ok(dst)
}

/// Spawns the LSP binary against `workspace_root`.
pub fn spawn_lsp(workspace_root: &Path) -> Result<Child> {
    let bin = assert_cmd::cargo::cargo_bin("deslop-lsp");
    Ok(Command::new(bin)
        .arg(workspace_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?)
}

/// Acquires child stdio handles after a successful spawn.
pub fn take_io(child: &mut Child) -> Result<(ChildStdin, BufReader<ChildStdout>, ChildStderr)> {
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("child stdin missing"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("child stdout missing"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("child stderr missing"))?;
    Ok((stdin, BufReader::new(stdout), stderr))
}

/// Copies the named fixture into a temp workspace, spawns the LSP against
/// it, and takes the child's stdio handles. Returns the workspace temp dir
/// (keep it bound — dropping it deletes the workspace), the child process,
/// and its stdin / buffered stdout / stderr. The caller binds whichever it
/// needs `mut`.
pub fn spawn_lsp_on_fixture(
    name: &str,
) -> Result<(
    tempfile::TempDir,
    Child,
    ChildStdin,
    BufReader<ChildStdout>,
    ChildStderr,
)> {
    let workspace = copy_fixture(name)?;
    let mut child = spawn_lsp(workspace.path())?;
    let (stdin, stdout, stderr) = take_io(&mut child)?;
    Ok((workspace, child, stdin, stdout, stderr))
}

/// Writes one LSP framed payload.
pub fn write_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(payload.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

/// Reads one framed JSON-RPC response.
pub fn read_frame(reader: &mut BufReader<ChildStdout>) -> Result<serde_json::Value> {
    let length = read_content_length(reader)?;
    let mut buf = vec![0_u8; length];
    reader.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

/// Reads the `Content-Length` header block.
fn read_content_length(reader: &mut BufReader<ChildStdout>) -> Result<usize> {
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
    content_length.ok_or_else(|| anyhow!("missing Content-Length"))
}

/// Sends a request and waits for the matching response id.
pub fn send_and_recv(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: i64,
    payload: &str,
) -> Result<serde_json::Value> {
    write_frame(stdin, payload)?;
    loop {
        let frame = read_frame(reader)?;
        if frame.get("id").and_then(serde_json::Value::as_i64) == Some(id) {
            return Ok(frame);
        }
    }
}

/// Builds an `initialize` request.
pub fn initialize_request() -> Result<(i64, String)> {
    request(
        "initialize",
        &serde_json::json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }),
    )
}

/// Builds a JSON-RPC request envelope.
pub fn request(method: &str, params: &serde_json::Value) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });
    Ok((id, serde_json::to_string(&payload)?))
}

/// Builds a JSON-RPC notification.
pub fn notification(method: &str, params: &serde_json::Value) -> Result<String> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    Ok(serde_json::to_string(&payload)?)
}

/// Drives `initialize` + `initialized` and returns the server response.
pub fn handshake(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
) -> Result<serde_json::Value> {
    let (init_id, init) = initialize_request()?;
    let response = send_and_recv(stdin, stdout, init_id, &init)?;
    write_frame(stdin, &notification("initialized", &serde_json::json!({}))?)?;
    Ok(response)
}

/// Sends a request, waits for the paired response, and returns the full
/// JSON-RPC frame. Errors surface to the caller verbatim for inspection.
pub fn call(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    method: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let (id, payload) = request(method, params)?;
    send_and_recv(stdin, stdout, id, &payload)
}

/// RAII guard that kills and reaps the spawned LSP child when it drops, so a
/// failing assertion never leaks the process.
pub struct KillOnDrop<'a>(pub &'a mut Child);

impl Drop for KillOnDrop<'_> {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// Owning RAII guard: holds the spawned LSP child and kills it on drop. Unlike
/// [`KillOnDrop`] it owns the process, so a helper can return the guard already
/// armed — any later failure (handshake, request) still reaps the child.
pub struct LspGuard(Child);

impl Drop for LspGuard {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// Spawns the LSP against `workspace`, takes its stdin/stdout, and returns the
/// process wrapped in an armed [`LspGuard`] alongside those handles. The guard
/// is live before the caller runs the handshake, matching the spawn-then-guard
/// ordering of the inline setup it replaces.
pub fn spawn_lsp_guarded(
    workspace: &Path,
) -> Result<(LspGuard, ChildStdin, BufReader<ChildStdout>)> {
    let mut child = spawn_lsp(workspace)?;
    let (stdin, stdout, _stderr) = take_io(&mut child)?;
    Ok((LspGuard(child), stdin, stdout))
}

/// Copies the named fixture into a temp workspace, spawns the LSP, and returns
/// the workspace (keep it bound — dropping it deletes the workspace) plus an
/// armed [`LspGuard`] and the child's stdin/stdout.
pub fn spawn_lsp_on_fixture_guarded(
    name: &str,
) -> Result<(
    tempfile::TempDir,
    LspGuard,
    ChildStdin,
    BufReader<ChildStdout>,
)> {
    let workspace = copy_fixture(name)?;
    let (guard, stdin, stdout) = spawn_lsp_guarded(workspace.path())?;
    Ok((workspace, guard, stdin, stdout))
}

/// Builds the `workspace/didChangeWatchedFiles` notification VS Code sends
/// after a save of `path`.
pub fn watched_file_changed(path: &Path) -> Result<String> {
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path is not absolute: {}", path.display()))?;
    notification(
        "workspace/didChangeWatchedFiles",
        &serde_json::json!({ "changes": [{ "uri": uri.as_str(), "type": 2 }] }),
    )
}

/// Returns the report cluster count from a JSON-RPC response frame.
#[must_use]
pub fn cluster_count(frame: &serde_json::Value) -> usize {
    frame
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}
