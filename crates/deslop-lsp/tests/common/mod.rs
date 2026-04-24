//! Shared E2E helpers for the `deslop-lsp` integration tests. Drives the
//! real binary over stdio with LSP framing — no mocked transport, no
//! fake service.

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
pub fn spawn_lsp(workspace_root: &Path, min_nodes: u32) -> Result<Child> {
    let bin = assert_cmd::cargo::cargo_bin("deslop-lsp");
    Ok(Command::new(bin)
        .arg(workspace_root)
        .arg("--min-nodes")
        .arg(min_nodes.to_string())
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
