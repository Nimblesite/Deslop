//! End-to-end coverage for issue #29: the LSP must log an
//! `elapsed_ms` field on each `deslop/reportGet` call so an
//! operator can answer "is deslop-lsp chewing CPU or just idle?"
//! by tailing the log.
//!
//! Audience: HUMAN (operator). The field is emitted at INFO on
//! stderr and machine-parseable (`elapsed_ms=42`) so it doubles as
//! the agent-facing observability surface — one log line, one
//! audience-neutral record.

use std::{
    io::{Read, Write},
    process::{ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicI64, Ordering},
    thread,
    time::Duration,
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};

static NEXT_ID: AtomicI64 = AtomicI64::new(90_000);

/// Audience: HUMAN. Issue #29. Calling `deslop/reportGet` must
/// leave a structured log line on stderr containing `elapsed_ms`
/// so operators can see handler duration. Positive invariant: the
/// field appears after we issue the request.
#[test]
fn report_get_handler_logs_elapsed_ms() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let mut child = Command::new(assert_cmd::cargo::cargo_bin("deslop-lsp"))
        .arg(workspace.path())
        .env("RUST_LOG", "info")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

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
    let _report = call(&mut stdin, &mut reader, "deslop/reportGet", &json!({}))?;

    thread::sleep(Duration::from_millis(400));
    let _ = child.kill();
    let output = child.wait_with_output()?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    assert!(
        stderr.contains("elapsed_ms"),
        "report/get handler must log an elapsed_ms field so operators can spot CPU spikes; \
         stderr was:\n{stderr}"
    );
    Ok(())
}

fn request(method: &str, params: &Value) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
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

fn call(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    method: &str,
    params: &Value,
) -> Result<Value> {
    let (id, payload) = request(method, params)?;
    send_and_recv(stdin, reader, id, &payload)
}
