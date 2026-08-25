//! Shared E2E helpers for the `deslop-lsp` integration tests. Drives the
//! real binary over stdio with LSP framing — no mocked transport, no
//! fake service.
//!
//! Each integration binary pulls in only the subset of helpers it needs,
//! so the unused-symbol lint is silenced for this shared module (matching
//! the `deslop-core` and `deslop-mcp` test commons).

#![allow(dead_code)]

pub mod reports;

use std::{
    fs,
    io::BufReader,
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio},
    sync::atomic::{AtomicI64, Ordering},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

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
    deslop_test_support::spawn_deslop_lsp(workspace_root, Stdio::piped())
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
    deslop_test_support::write_lsp_frame(stdin, payload)
}

/// Reads one framed JSON-RPC response.
pub fn read_frame(reader: &mut BufReader<ChildStdout>) -> Result<serde_json::Value> {
    deslop_test_support::read_lsp_frame(reader)
}

/// Sends a request and waits for the matching response id, discarding
/// any server-initiated frames seen on the way.
pub fn send_and_recv(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: i64,
    payload: &str,
) -> Result<serde_json::Value> {
    write_frame(stdin, payload)?;
    Ok(recv_response(reader, id)?.0)
}

/// Reads frames until the response carrying `id` arrives, collecting every
/// server-initiated frame (notifications and server→client requests)
/// emitted before it.
fn recv_response(reader: &mut BufReader<ChildStdout>, id: i64) -> Result<(Value, Vec<Value>)> {
    let mut server_frames = Vec::new();
    loop {
        let frame = read_frame(reader)?;
        if frame.get("id").and_then(Value::as_i64) == Some(id) && frame.get("method").is_none() {
            return Ok((frame, server_frames));
        }
        server_frames.push(frame);
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

/// Sends a request, waits for the paired response, and returns it together
/// with every server-initiated frame (e.g. `window/showMessage`) the
/// server emitted before the response.
pub fn call_capturing(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    method: &str,
    params: &serde_json::Value,
) -> Result<(Value, Vec<Value>)> {
    let (id, payload) = request(method, params)?;
    write_frame(stdin, &payload)?;
    recv_response(stdout, id)
}

/// RAII guard that closes the spawned LSP child's stdin and reaps it when it
/// drops, so a failing assertion never leaks the process — and never signals
/// one, which would discard everything the child executed
/// (`deslop_test_support::reap`).
pub struct ReapOnDrop<'a>(pub &'a mut Child);

impl Drop for ReapOnDrop<'_> {
    fn drop(&mut self) {
        let _status = deslop_test_support::reap::reap(self.0);
    }
}

/// Owning RAII guard: holds the spawned LSP child and reaps it on drop. Unlike
/// [`ReapOnDrop`] it owns the process, so a helper can return the guard already
/// armed — any later failure (handshake, request) still reaps the child.
///
/// Reaping means closing stdin and waiting, never signalling: a signalled
/// child writes no coverage profile, so a `kill` here silently deletes every
/// line the server executed (`deslop_test_support::reap`). The caller's own
/// [`ChildStdin`] drops before this guard does, so the child already has EOF
/// by the time it is waited on.
pub struct LspGuard {
    child: Child,
    /// Continuously drained for the guard's whole lifetime (GH #370).
    _stderr: StderrDrain,
}

impl Drop for LspGuard {
    fn drop(&mut self) {
        let _status = deslop_test_support::reap::reap(&mut self.child);
    }
}

/// Reads a spawned LSP's stderr to EOF on a background thread, discarding
/// it, and joins that thread on drop.
///
/// GH #370: the server logs every stage through `tracing` to stderr. A
/// piped stderr that is merely *held open* still fills its fixed kernel
/// pipe buffer, and the next `tracing` event then blocks its thread inside
/// `Stderr::write_all` while holding the subscriber's stderr lock. Every
/// other thread that logs — including the `tower-lsp` serve loop — queues
/// behind that lock, so the server stops reading stdin and writing stdout
/// altogether and the test waits forever on a response the server can no
/// longer send. It is not a protocol defect: the terminal progress frame
/// is produced, and the transport that would carry it is wedged.
///
/// The rejection paths hit it first because they log per failed subtree
/// and per bisect retry, so they are the first to exceed the buffer.
/// Keeping the pipe empty is the fix; keeping the handle open is not
/// enough.
pub struct StderrDrain(Option<JoinHandle<()>>);

impl StderrDrain {
    /// Starts draining `stderr`. The thread ends when the child exits and
    /// closes the write end.
    fn spawn(mut stderr: ChildStderr) -> Self {
        Self(Some(std::thread::spawn(move || {
            let _drained = std::io::copy(&mut stderr, &mut std::io::sink());
        })))
    }
}

impl Drop for StderrDrain {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            let _joined = handle.join();
        }
    }
}

/// Spawns the LSP against `workspace`, takes its stdin/stdout, and returns the
/// process wrapped in an armed [`LspGuard`] alongside those handles. The guard
/// is live before the caller runs the handshake, matching the spawn-then-guard
/// ordering of the inline setup it replaces. The guard drains the child's
/// stderr for the whole test — see [`StderrDrain`].
pub fn spawn_lsp_guarded(
    workspace: &Path,
) -> Result<(LspGuard, ChildStdin, BufReader<ChildStdout>)> {
    let mut child = spawn_lsp(workspace)?;
    let (stdin, stdout, stderr) = take_io(&mut child)?;
    Ok((
        LspGuard {
            child,
            _stderr: StderrDrain::spawn(stderr),
        },
        stdin,
        stdout,
    ))
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

/// Polls the real `deslop/reportGet` method until `predicate` accepts the
/// returned report or `timeout` expires. The bounded poll waits on observable
/// report state rather than assuming how long a cold or incremental pass takes.
pub fn wait_for_report_matching(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    timeout: Duration,
    predicate: impl Fn(&Value) -> bool,
) -> Result<Value> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    loop {
        let frame = call(stdin, stdout, "deslop/reportGet", &json!({}))?;
        let report = frame
            .get("result")
            .ok_or_else(|| anyhow!("reportGet returned no result: {frame}"))?;
        if predicate(report) {
            return Ok(report.clone());
        }
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "report state did not converge within {timeout:?}: {report}"
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Analysis must settle within this budget on any dev machine; the
/// code-action fixtures are single small files.
pub const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(20);

/// Poll cadence while waiting for the first analysis pass.
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Builds a `textDocument/codeAction` params payload for `uri` covering
/// the zero-indexed `line` span.
#[must_use]
pub fn code_action_params(uri: &str, start_line: u32, end_line: u32) -> Value {
    json!({
        "textDocument": { "uri": uri },
        "range": {
            "start": { "line": start_line, "character": 0 },
            "end": { "line": end_line, "character": 0 }
        },
        "context": { "diagnostics": [] }
    })
}

/// Polls `textDocument/codeAction` until the first analysis pass
/// surfaces an action (bounded, no arbitrary sleeps beyond the poll
/// cadence).
pub fn wait_for_actions(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    params: &Value,
) -> Result<Vec<Value>> {
    let deadline = Instant::now()
        .checked_add(ANALYSIS_TIMEOUT)
        .unwrap_or_else(Instant::now);
    loop {
        let response = call(stdin, stdout, "textDocument/codeAction", params)?;
        if let Some(actions) = response.pointer("/result").and_then(Value::as_array) {
            if !actions.is_empty() {
                return Ok(actions.clone());
            }
        }
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "no code action surfaced within {ANALYSIS_TIMEOUT:?}"
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Finds the lazily-resolved `refactor.rewrite` offer with `title`
/// among `actions`, asserting the shared offer shape
/// ([AUTOFIX-MERGE-CODE-ACTION] step 1): kind `refactor.rewrite`, edit
/// omitted, cluster id in `data`.
pub fn rewrite_offer<'a>(actions: &'a [Value], title: &str) -> Result<&'a Value> {
    let offer = actions
        .iter()
        .find(|action| action.pointer("/title").and_then(Value::as_str) == Some(title))
        .with_context(|| format!("rewrite offer `{title}` present"))?;
    ensure!(
        offer.pointer("/kind").and_then(Value::as_str) == Some("refactor.rewrite"),
        "offer kind must be refactor.rewrite"
    );
    ensure!(
        offer.pointer("/edit").is_none(),
        "the offer omits the edit — lazy resolve"
    );
    ensure!(
        offer
            .pointer("/data/cluster_id")
            .and_then(Value::as_str)
            .is_some(),
        "the offer carries the cluster id"
    );
    Ok(offer)
}

/// Reads `value[key]` without the `Index` impl.
///
/// `serde_json`'s `Index` panics on a type mismatch and trips
/// `clippy::indexing_slicing`, so tests reach fields through this. A
/// missing key yields `Value::Null` — identical to what `Index` returns
/// — so an assertion against an absent field still fails loudly rather
/// than being skipped.
pub fn at<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

/// Reads a nested path, one key per element.
pub fn path<'a>(value: &'a Value, keys: &[&str]) -> &'a Value {
    keys.iter().fold(value, |current, key| at(current, key))
}

/// Reads the `index`th element of a JSON array field.
pub fn nth<'a>(value: &'a Value, key: &str, index: usize) -> &'a Value {
    at(value, key)
        .as_array()
        .and_then(|items| items.get(index))
        .unwrap_or(&Value::Null)
}
