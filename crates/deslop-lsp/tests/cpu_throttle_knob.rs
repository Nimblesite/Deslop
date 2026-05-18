//! End-to-end coverage for issue #28: the LSP needs a `--worker-threads N`
//! knob so users can background-ise the analyser when it is saturating
//! CPU on large workspaces.
//!
//! Audience: HUMAN. The editor user who runs `deslop-lsp` on a
//! sprawling monorepo needs a way to constrain the tokio worker
//! count. The flag must be accepted without error; the exact tokio
//! configuration is covered at unit test level inside the LSP crate
//! but the CLI contract is owned here.

use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicI64, Ordering},
    thread,
    time::Duration,
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

static NEXT_ID: AtomicI64 = AtomicI64::new(120_000);

/// Audience: HUMAN. Issue #28. Positive invariant: when the LSP
/// user passes `--worker-threads N`, the startup log line records
/// the chosen value so the user can confirm the knob took effect
/// by tailing the log. The log line goes to stderr; we scrape it
/// briefly after spawn.
#[test]
fn lsp_startup_log_records_the_worker_threads_knob() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let child = Command::new(assert_cmd::cargo::cargo_bin("deslop-lsp"))
        .arg(workspace.path())
        .arg("--worker-threads")
        .arg("2")
        .env("RUST_LOG", "info")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Give the startup log line a moment to land, then kill and
    // drain stderr through `wait_with_output` so the pipe closes
    // cleanly and the tracing buffer reaches us.
    thread::sleep(Duration::from_millis(1500));
    #[allow(unused_mut)]
    let mut handle = child;
    let _ = handle.kill();
    let output = handle.wait_with_output()?;
    let stderr_buf = String::from_utf8_lossy(&output.stderr).into_owned();

    assert!(
        stderr_buf.contains("worker_threads=2"),
        "startup log must record the honored worker_threads value so users can confirm \
         the throttle knob took effect; stderr was:\n{stderr_buf}"
    );

    Ok(())
}

/// Audience: HUMAN. Issue #28. Positive invariant: the user-facing
/// throttle settings are honored at startup, and the structured CPU
/// report can prove the LSP is idle with no queued watcher or embedding
/// work when the workspace is quiet.
#[test]
fn lsp_nice_and_worker_knobs_preserve_idle_cpu_report() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let mut child = Command::new(assert_cmd::cargo::cargo_bin("deslop-lsp"))
        .arg(workspace.path())
        .arg("--worker-threads")
        .arg("1")
        .arg("--nice")
        .arg("5")
        .env("RUST_LOG", "info")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    thread::sleep(Duration::from_millis(250));
    if child.try_wait()?.is_some() {
        let output = child.wait_with_output()?;
        return Err(anyhow!(
            "deslop-lsp exited before handshake; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("child stdin missing"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("child stdout missing"))?;
    let mut stdin = stdin;
    let mut reader = BufReader::new(stdout);

    let _init = handshake(&mut stdin, &mut reader)?;
    let response = call(&mut stdin, &mut reader, "deslop/cpuReport", &json!({}))?;
    let result = response
        .get("result")
        .ok_or_else(|| anyhow!("cpu report missing result: {response}"))?;
    assert_eq!(
        result.get("current_phase").and_then(Value::as_str),
        Some("idle"),
        "quiet workspace should report idle CPU phase: {result}"
    );
    assert!(
        result
            .get("last_100_phases")
            .and_then(Value::as_array)
            .is_some(),
        "cpu report should expose phase history even when empty: {result}"
    );
    assert_eq!(
        result
            .get("in_flight")
            .and_then(|value| value.get("pending_watcher_events"))
            .and_then(Value::as_u64),
        Some(0),
        "idle report should have no pending watcher events: {result}"
    );
    assert_eq!(
        result
            .get("in_flight")
            .and_then(|value| value.get("pending_embed_requests"))
            .and_then(Value::as_u64),
        Some(0),
        "idle report should have no pending embedding requests: {result}"
    );
    assert!(
        result
            .get("handler_counts")
            .and_then(|value| value.get("deslop/cpuReport"))
            .and_then(Value::as_u64)
            .is_some_and(|count| count >= 1),
        "cpu report should count the diagnostic request itself: {result}"
    );

    let shutdown_response = shutdown(&mut stdin, &mut reader)?;
    assert!(
        shutdown_response.get("error").is_none(),
        "shutdown should complete cleanly after idle cpu report: {shutdown_response}"
    );
    drop(stdin);

    let output = child.wait_with_output()?;
    let stderr_buf = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr_buf.contains("worker_threads=1"),
        "startup log must record worker_threads=1; stderr was:\n{stderr_buf}"
    );
    assert!(
        stderr_buf.contains("nice=5"),
        "startup log must record nice=5; stderr was:\n{stderr_buf}"
    );
    Ok(())
}

fn request(method: &str, params: &Value) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
    Ok((id, serde_json::to_string(&payload)?))
}

fn request_without_params(method: &str) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = json!({"jsonrpc":"2.0","id":id,"method":method});
    Ok((id, serde_json::to_string(&payload)?))
}

fn notification(method: &str, params: &Value) -> Result<String> {
    let payload = json!({"jsonrpc":"2.0","method":method,"params":params});
    Ok(serde_json::to_string(&payload)?)
}

fn write_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(payload.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

fn read_content_length(reader: &mut BufReader<ChildStdout>) -> Result<usize> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(anyhow!("stdout closed before Content-Length"));
        }
        if line == "\r\n" {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = Some(rest.trim().parse::<usize>()?);
        }
    }
    content_length.ok_or_else(|| anyhow!("missing Content-Length"))
}

fn read_frame(reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    let length = read_content_length(reader)?;
    let mut buf = vec![0_u8; length];
    reader.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

fn send_and_recv(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: i64,
    payload: &str,
) -> Result<Value> {
    write_frame(stdin, payload)?;
    loop {
        let frame = read_frame(reader)?;
        if frame.get("id").and_then(Value::as_i64) == Some(id) {
            return Ok(frame);
        }
    }
}

fn handshake(stdin: &mut ChildStdin, reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    let (id, payload) = request(
        "initialize",
        &json!({"processId": null, "rootUri": null, "capabilities": {}}),
    )?;
    let response = send_and_recv(stdin, reader, id, &payload)?;
    write_frame(stdin, &notification("initialized", &json!({}))?)?;
    Ok(response)
}

fn shutdown(stdin: &mut ChildStdin, reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    let (id, payload) = request_without_params("shutdown")?;
    let response = send_and_recv(stdin, reader, id, &payload)?;
    write_frame(stdin, &notification("exit", &json!({}))?)?;
    Ok(response)
}

fn call(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    method: &str,
    params: &Value,
) -> Result<Value> {
    let (id, payload) = request(method, params)?;
    send_and_recv(stdin, reader, id, &payload)
}
