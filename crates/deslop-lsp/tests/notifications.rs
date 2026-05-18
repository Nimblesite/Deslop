//! End-to-end notification coverage for the LSP binary.
//!
//! Drives real JSON-RPC frames over stdio so normal VS Code
//! notifications cannot regress to tower-lsp's "not implemented"
//! warnings.

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        atomic::{AtomicI64, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Result};

type FrameResult = std::result::Result<serde_json::Value, String>;

/// JSON-RPC id counter for this integration-test process.
static NEXT_ID: AtomicI64 = AtomicI64::new(10_000);

/// Accepts normal VS Code notifications without tower-lsp fallback
/// warnings.
#[test]
fn vscode_core_notifications_are_implemented_or_explicitly_nooped() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, stderr) = take_io(&mut child)?;

    let (init_id, init) = initialize_request()?;
    let _init_response = send_and_recv(&mut stdin, &mut stdout, init_id, &init)?;
    for notification in normal_vscode_notifications(&alpha)? {
        write_frame(&mut stdin, &notification)?;
    }
    let (shutdown_id, shutdown) = request("shutdown", &serde_json::Value::Null)?;
    let _shutdown_response = send_and_recv(&mut stdin, &mut stdout, shutdown_id, &shutdown)?;

    let stderr_text = shutdown_and_read_stderr(child, stderr)?;
    assert!(
        !stderr_text.contains("not implemented"),
        "normal VS Code notifications must be implemented or explicit no-ops: {stderr_text}"
    );
    Ok(())
}

/// [LIVE-NOTIFICATIONS] A pure-removal edit must push reportChanged
/// and expose the removed cluster ids through reportDelta.
#[test]
fn report_changed_fires_for_pure_removal_delta() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);
    let frames = spawn_frame_reader(stdout);

    let (init_id, init) = initialize_request()?;
    let _init = send_and_recv_frame(&mut stdin, &frames, init_id, &init)?;
    let initial = request_response(
        &mut stdin,
        &frames,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
    ensure!(
        cluster_count(&initial) > 0,
        "fixture must start with duplicate clusters"
    );

    fs::write(&beta, unrelated_csharp())?;
    write_frame(&mut stdin, &watched_file_changed(&beta)?)?;
    let changed = recv_method(&frames, "deslop/reportChanged", Duration::from_secs(5))?;
    let removed = json_u64(&changed, "/params/summary/clusters_removed")?;
    ensure!(
        removed > 0,
        "reportChanged must describe at least one removed cluster"
    );

    let params = serde_json::json!({ "since_generation": 1 });
    let delta = request_response(&mut stdin, &frames, "deslop/reportDelta", &params)?;
    ensure!(
        json_array_len(&delta, "/result/clusters_removed")? > 0,
        "reportDelta must include removed cluster ids"
    );
    Ok(())
}

/// [LIVE-WATCHER] Pure-fs edits — without any LSP `didChangeWatchedFiles`
/// notification — must each push a `deslop/reportChanged` so AI agents
/// editing files outside the editor still drive the VSIX tree update
/// ([VSIX-REACTIVITY-TREE]). Two consecutive saves of the same path
/// must both surface as separate notifications; the watcher's batch
/// dedup guard must not silently swallow the second save.
#[test]
fn report_changed_fires_for_each_external_save_of_the_same_path() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);
    let frames = spawn_frame_reader(stdout);

    let (init_id, init) = initialize_request()?;
    let _init = send_and_recv_frame(&mut stdin, &frames, init_id, &init)?;
    let initial = request_response(
        &mut stdin,
        &frames,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
    ensure!(
        cluster_count(&initial) > 0,
        "fixture must start with duplicate clusters"
    );

    // First external save: rewrite Beta.cs to break the duplication.
    fs::write(&beta, unrelated_csharp())?;
    let first = recv_method(&frames, "deslop/reportChanged", Duration::from_secs(10))?;
    let first_generation = json_u64(&first, "/params/generation")?;

    // Second external save of the same path: rewrite again to introduce a
    // different unique body. The watcher's dedup guard previously dropped
    // every event after the first one for any given path, so the LSP would
    // never push reportChanged again — exactly the "tree freezes" symptom
    // the user reported.
    fs::write(&beta, second_unrelated_csharp())?;
    let second = recv_method(&frames, "deslop/reportChanged", Duration::from_secs(10))?;
    let second_generation = json_u64(&second, "/params/generation")?;
    ensure!(
        second_generation > first_generation,
        "second save on the same path must yield a strictly newer generation \
         (first={first_generation}, second={second_generation})",
    );
    Ok(())
}

/// Second replacement body for the back-to-back saves test above.
fn second_unrelated_csharp() -> &'static str {
    "public class Beta {\n    public string Description() {\n        return \"distinct\";\n    }\n}\n"
}

struct KillOnDrop<'a>(&'a mut Child);

impl Drop for KillOnDrop<'_> {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// Returns a workspace-relative fixture path.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("deslop")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copies a fixture into a temp directory so the LSP can write caches.
fn copy_fixture(name: &str) -> Result<tempfile::TempDir> {
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
fn spawn_lsp(workspace_root: &Path) -> Result<Child> {
    let bin = assert_cmd::cargo::cargo_bin("deslop-lsp");
    Ok(Command::new(bin)
        .arg(workspace_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?)
}

/// Acquires child stdio handles after a successful spawn.
fn take_io(child: &mut Child) -> Result<(ChildStdin, BufReader<ChildStdout>, ChildStderr)> {
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

/// Writes one LSP framed payload.
fn write_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(payload.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

/// Reads one framed JSON-RPC response.
fn read_frame(reader: &mut BufReader<ChildStdout>) -> Result<serde_json::Value> {
    let length = read_content_length(reader)?;
    let mut buf = vec![0_u8; length];
    reader.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

/// Reads frames on a background thread so tests can time out waiting
/// for notifications without blocking forever on stdout.
fn spawn_frame_reader(mut stdout: BufReader<ChildStdout>) -> Receiver<FrameResult> {
    let (tx, rx) = mpsc::channel();
    let _reader = std::thread::spawn(move || loop {
        let frame = read_frame(&mut stdout).map_err(|error| error.to_string());
        if tx.send(frame).is_err() {
            break;
        }
    });
    rx
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
fn send_and_recv(
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

/// Sends a request and waits for the matching response from a frame channel.
fn send_and_recv_frame(
    stdin: &mut ChildStdin,
    frames: &Receiver<FrameResult>,
    id: i64,
    payload: &str,
) -> Result<serde_json::Value> {
    write_frame(stdin, payload)?;
    recv_response(frames, id, Duration::from_secs(10))
}

/// Builds, sends, and receives a JSON-RPC request.
fn request_response(
    stdin: &mut ChildStdin,
    frames: &Receiver<FrameResult>,
    method: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let (id, payload) = request(method, params)?;
    send_and_recv_frame(stdin, frames, id, &payload)
}

/// Waits until a response with `id` arrives.
fn recv_response(
    frames: &Receiver<FrameResult>,
    id: i64,
    timeout: Duration,
) -> Result<serde_json::Value> {
    let deadline = deadline_after(timeout)?;
    loop {
        let frame = recv_next(frames, deadline, &format!("response id {id}"))?;
        if frame.get("id").and_then(serde_json::Value::as_i64) == Some(id) {
            return Ok(frame);
        }
    }
}

/// Waits until a notification with `method` arrives.
fn recv_method(
    frames: &Receiver<FrameResult>,
    method: &str,
    timeout: Duration,
) -> Result<serde_json::Value> {
    let deadline = deadline_after(timeout)?;
    loop {
        let frame = recv_next(frames, deadline, method)?;
        if frame.get("method").and_then(serde_json::Value::as_str) == Some(method) {
            return Ok(frame);
        }
    }
}

fn deadline_after(timeout: Duration) -> Result<Instant> {
    Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| anyhow!("timeout duration is too large"))
}

/// Receives one frame before `deadline`.
fn recv_next(
    frames: &Receiver<FrameResult>,
    deadline: Instant,
    label: &str,
) -> Result<serde_json::Value> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    ensure!(!remaining.is_zero(), "timed out waiting for {label}");
    let frame = frames
        .recv_timeout(remaining)
        .map_err(|_error| anyhow!("timed out waiting for {label}"))?;
    frame.map_err(|error| anyhow!(error))
}

/// Builds an `initialize` request.
fn initialize_request() -> Result<(i64, String)> {
    request(
        "initialize",
        &serde_json::json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }),
    )
}

/// Builds a JSON-RPC request.
fn request(method: &str, params: &serde_json::Value) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });
    Ok((id, serde_json::to_string(&payload)?))
}

/// Builds the notification set VS Code sends during normal operation.
fn normal_vscode_notifications(path: &Path) -> Result<Vec<String>> {
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path is not absolute: {}", path.display()))?;
    Ok(vec![
        notification("initialized", &serde_json::json!({}))?,
        notification(
            "workspace/didChangeConfiguration",
            &serde_json::json!({ "settings": {} }),
        )?,
        notification(
            "workspace/didChangeWatchedFiles",
            &serde_json::json!({ "changes": [{ "uri": uri.as_str(), "type": 2 }] }),
        )?,
        notification(
            "textDocument/didOpen",
            &serde_json::json!({
                "textDocument": {
                    "uri": uri.as_str(),
                    "languageId": "csharp",
                    "version": 1,
                    "text": fs::read_to_string(path)?
                }
            }),
        )?,
        notification(
            "textDocument/didClose",
            &serde_json::json!({ "textDocument": { "uri": uri.as_str() } }),
        )?,
    ])
}

/// Builds the watched-file notification VS Code sends after a save.
fn watched_file_changed(path: &Path) -> Result<String> {
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path is not absolute: {}", path.display()))?;
    notification(
        "workspace/didChangeWatchedFiles",
        &serde_json::json!({ "changes": [{ "uri": uri.as_str(), "type": 2 }] }),
    )
}

/// Replacement content with no duplicated method body.
fn unrelated_csharp() -> &'static str {
    "public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n"
}

/// Builds a JSON-RPC notification.
fn notification(method: &str, params: &serde_json::Value) -> Result<String> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    Ok(serde_json::to_string(&payload)?)
}

/// Returns the report cluster count from a JSON-RPC response frame.
fn cluster_count(frame: &serde_json::Value) -> usize {
    frame
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

/// Reads a required integer field from a JSON frame.
fn json_u64(frame: &serde_json::Value, pointer: &str) -> Result<u64> {
    frame
        .pointer(pointer)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow!("missing numeric field {pointer}: {frame}"))
}

/// Reads a required array length from a JSON frame.
fn json_array_len(frame: &serde_json::Value, pointer: &str) -> Result<usize> {
    frame
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| anyhow!("missing array field {pointer}: {frame}"))
}

/// Stops the child process and returns stderr.
fn shutdown_and_read_stderr(mut child: Child, mut stderr: ChildStderr) -> Result<String> {
    let _kill = child.kill();
    let _wait = child.wait();
    let mut text = String::new();
    let _read = stderr.read_to_string(&mut text)?;
    Ok(text)
}
