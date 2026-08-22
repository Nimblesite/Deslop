//! End-to-end notification coverage for the LSP binary.
//!
//! Drives real JSON-RPC frames over stdio so normal VS Code
//! notifications cannot regress to tower-lsp's "not implemented"
//! warnings.

use std::{
    fs,
    io::{BufReader, Read},
    path::Path,
    process::{Child, ChildStderr, ChildStdin, ChildStdout},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Result};

use crate::common::*;

type FrameResult = std::result::Result<serde_json::Value, String>;

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

/// [LIVE-NOTIFICATIONS] [PRINCIPLES-LIVE-IS-REACTIVE]
/// A pure-removal edit must push reportChanged and expose the removed cluster
/// ids through reportDelta.
#[test]
fn report_changed_fires_for_pure_removal_delta() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let (_guard, mut stdin, frames, initial) = spawn_lsp_with_initial_report(workspace.path())?;
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

/// [LSP-PUSH-NOTIFICATIONS] The LSP client console must show that Deslop
/// is working. Every analysis pass surfaces a human-readable
/// `window/logMessage`, not only a `tracing` line on stderr the LSP
/// client never renders. Regression for "there is no logging": the
/// LSP4IJ Logs tab (and the VS Code output channel) showed a single
/// `deslop-lsp initialised` line at startup and then stayed silent while
/// Deslop re-analysed the workspace on every save, so the running engine
/// looked dead. An external save must push an Info `window/logMessage`
/// naming the pass and its cluster delta.
#[test]
fn analysis_pass_surfaces_a_window_log_message() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let (_guard, _stdin, frames, initial) = spawn_lsp_with_initial_report(workspace.path())?;
    ensure!(
        cluster_count(&initial) > 0,
        "fixture must start with duplicate clusters"
    );

    // Break the clone pair on disk. The watcher runs a fresh analysis
    // pass, which must announce itself on the LSP log channel.
    fs::write(&beta, unrelated_csharp())?;
    let logged = recv_method(&frames, "window/logMessage", Duration::from_secs(10))?;
    let message_type = json_u64(&logged, "/params/type")?;
    ensure!(
        message_type == 3,
        "an analysis-pass log must be Info (LSP MessageType 3), got {message_type}: {logged}"
    );
    let message = logged
        .pointer("/params/message")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("logMessage frame carries no message: {logged}"))?;
    ensure!(
        message.contains("Deslop") && message.contains("pass") && message.contains("removed"),
        "the analysis-pass log must name Deslop, the pass, and its cluster delta so the \
         console shows real activity: {message}"
    );
    Ok(())
}

/// [LIVE-WATCHER] [PRINCIPLES-LIVE-IS-REACTIVE]
/// Pure-fs edits — without any LSP `didChangeWatchedFiles`
/// notification — must each push a `deslop/reportChanged` so AI agents
/// editing files outside the editor still drive the VSIX tree update
/// ([VSIX-REACTIVITY-TREE]). Two consecutive saves of the same path
/// must both surface as separate notifications; the watcher's batch
/// dedup guard must not silently swallow the second save.
#[test]
fn report_changed_fires_for_each_external_save_of_the_same_path() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let (_guard, _stdin, frames, initial) = spawn_lsp_with_initial_report(workspace.path())?;
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

/// [LIVE-WATCHER] A pure-fs edit to a Dart file must drive the live
/// loop exactly like C#/Rust/Python. The watcher's extension filter
/// must cover every registered `LanguageParser`; a `.dart` save with
/// no LSP notification must still push `deslop/reportChanged`.
#[test]
fn report_changed_fires_for_external_save_of_dart_file() -> Result<()> {
    let workspace = copy_fixture("dart-small")?;
    let beta = workspace.path().join("beta.dart");
    let (_guard, _stdin, frames, initial) = spawn_lsp_with_initial_report(workspace.path())?;
    ensure!(
        cluster_count(&initial) > 0,
        "dart fixture must start with duplicate clusters"
    );

    // Pure external save: rewrite beta.dart on disk to break the clone
    // pair. No didChangeWatchedFiles is sent — only the filesystem
    // watcher can deliver this event to the scheduler.
    fs::write(&beta, unrelated_dart())?;
    let changed = recv_method(&frames, "deslop/reportChanged", Duration::from_secs(10))?;
    let removed = json_u64(&changed, "/params/summary/clusters_removed")?;
    ensure!(
        removed > 0,
        "external .dart save must remove the broken clone cluster"
    );
    Ok(())
}

/// [CI-DESLOP] Issue #194: the live LSP surface must push a single
/// non-blocking warning when the measured duplication smashes the
/// `.deslop.toml [threshold] max_duplication_percent` budget, and the
/// resolved verdict must ride `RepoMetrics.threshold` so the DUPLICATION
/// panel can display the gate. The threshold stays a CLI gate — on the
/// live surface it only drives this informational `window/showMessage`
/// and the read-only `RepoMetrics.threshold` verdict, never suppression,
/// ranking, or any other live behaviour. The breach is confirmed up
/// front so the test can only fail because the warning is missing, not
/// because the fixture is clean.
#[test]
fn threshold_breach_pushes_non_blocking_warning_on_startup() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    fs::write(
        workspace.path().join(".deslop.toml"),
        "[threshold]\nmax_duplication_percent = 5.0\n",
    )?;
    let (_guard, mut stdin, frames, initial) = spawn_lsp_with_initial_report(workspace.path())?;
    let measured = initial
        .pointer("/result/metrics/duplication_percent")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| anyhow!("report carries no duplication_percent: {initial}"))?;
    ensure!(
        measured > 5.0,
        "fixture + config must smash the 5% budget so a missing warning is the only gap (measured {measured}%)"
    );
    // The live render path must resolve the .deslop.toml gate onto
    // RepoMetrics.threshold so the LSP/MCP DUPLICATION panel can show it —
    // not only the CLI. Before the fix this stayed source="none".
    let threshold = initial
        .pointer("/result/metrics/threshold")
        .ok_or_else(|| anyhow!("live report carries no metrics.threshold: {initial}"))?;
    let source = threshold
        .pointer("/source")
        .and_then(serde_json::Value::as_str);
    let breached = threshold
        .pointer("/breached")
        .and_then(serde_json::Value::as_bool);
    let percent = threshold
        .pointer("/percent")
        .and_then(serde_json::Value::as_f64);
    ensure!(
        source == Some("config"),
        "the live threshold must be sourced from .deslop.toml config: {threshold}"
    );
    ensure!(
        breached == Some(true),
        "a smashed 5% budget must mark the live threshold breached: {threshold}"
    );
    ensure!(
        percent == Some(5.0),
        "the live threshold must carry the configured 5% gate: {threshold}"
    );

    write_frame(
        &mut stdin,
        &notification("initialized", &serde_json::json!({}))?,
    )?;
    let warning = recv_method(&frames, "window/showMessage", Duration::from_secs(15))?;
    let message_type = json_u64(&warning, "/params/type")?;
    ensure!(
        message_type == 2,
        "a smashed budget must surface as a non-blocking Warning (type 2), got {message_type}: {warning}"
    );
    let message = warning
        .pointer("/params/message")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("showMessage frame carries no message: {warning}"))?;
    ensure!(
        message.contains("smashed") && message.contains("5%") && message.contains(".deslop.toml"),
        "the warning must name the smashed 5% threshold and its .deslop.toml source: {message}"
    );
    Ok(())
}

/// Replacement Dart body with no duplicated logic.
fn unrelated_dart() -> &'static str {
    "String describe(String name) {\n  return 'value: ' + name;\n}\n"
}

/// Second replacement body for the back-to-back saves test above.
fn second_unrelated_csharp() -> &'static str {
    "public class Beta {\n    public string Description() {\n        return \"distinct\";\n    }\n}\n"
}

/// Drives the `initialize` handshake over a frame channel.
fn frame_handshake(stdin: &mut ChildStdin, frames: &Receiver<FrameResult>) -> Result<()> {
    let (init_id, init) = initialize_request()?;
    let _init = send_and_recv_frame(stdin, frames, init_id, &init)?;
    Ok(())
}

/// Requests the current live report over a frame channel.
fn frame_report_get(
    stdin: &mut ChildStdin,
    frames: &Receiver<FrameResult>,
) -> Result<serde_json::Value> {
    request_response(stdin, frames, "deslop/reportGet", &serde_json::json!({}))
}

/// Spawns a guarded LSP, starts its frame reader, completes the handshake, and
/// returns the guard, stdin, frame channel, and the initial `deslop/reportGet`.
fn spawn_lsp_with_initial_report(
    workspace: &Path,
) -> Result<(
    LspGuard,
    ChildStdin,
    Receiver<FrameResult>,
    serde_json::Value,
)> {
    let (guard, mut stdin, stdout) = spawn_lsp_guarded(workspace)?;
    let frames = spawn_frame_reader(stdout);
    frame_handshake(&mut stdin, &frames)?;
    let initial = frame_report_get(&mut stdin, &frames)?;
    Ok((guard, stdin, frames, initial))
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

/// Replacement content with no duplicated method body.
fn unrelated_csharp() -> &'static str {
    "public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n"
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
