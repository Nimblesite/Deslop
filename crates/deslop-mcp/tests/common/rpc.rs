//! Newline-delimited JSON-RPC over a child process's stdio — the framing
//! every MCP harness in this suite speaks, kept in one place so the
//! id-matching loop and the MCP `initialize` request cannot drift
//! between test files.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout},
};

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

/// The MCP protocol revision every harness negotiates.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// The client version every harness reports in `clientInfo`.
const HARNESS_VERSION: &str = "0.1.0";

/// An id-tracked JSON-RPC conversation over a child's stdin and stdout.
pub struct StdioRpc {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl StdioRpc {
    /// Takes the piped stdin and stdout of a freshly spawned `child`.
    pub fn take(child: &mut Child) -> Result<Self> {
        let stdin = child.stdin.take().context("child stdin")?;
        let stdout = child.stdout.take().context("child stdout")?;
        Ok(Self {
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 0,
        })
    }

    /// Sends a request and returns the response carrying its id, skipping
    /// any notifications the server pushes in between.
    pub fn request(&mut self, method: &str, params: &Value) -> Result<Value> {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.send_frame(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))?;
        loop {
            let frame = self.read_frame()?;
            if frame.get("id").and_then(Value::as_i64) == Some(id) {
                return Ok(frame);
            }
            if frame.get("method").is_none() {
                return Err(anyhow!("unexpected frame without id match: {frame}"));
            }
        }
    }

    /// Sends the MCP `initialize` request as a client named `client_name`.
    pub fn initialize(&mut self, client_name: &str) -> Result<Value> {
        self.request(
            "initialize",
            &json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": client_name, "version": HARNESS_VERSION }
            }),
        )
    }

    /// Sends a notification: no id, so no response is awaited.
    pub fn notify(&mut self, method: &str, params: &Value) -> Result<()> {
        self.send_frame(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
    }

    /// Writes one frame as a single line.
    pub fn send_frame(&mut self, frame: &Value) -> Result<()> {
        self.send_raw_line(&serde_json::to_string(frame)?)
    }

    /// Writes `line` verbatim plus a newline, for malformed-frame scenarios.
    pub fn send_raw_line(&mut self, line: &str) -> Result<()> {
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Reads the next frame, failing when the child closes its stdout.
    pub fn read_frame(&mut self) -> Result<Value> {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line)?;
        if bytes == 0 {
            return Err(anyhow!("mcp stdout closed unexpectedly"));
        }
        serde_json::from_str(line.trim())
            .with_context(|| format!("invalid JSON from mcp: frame was: {line}"))
    }
}
