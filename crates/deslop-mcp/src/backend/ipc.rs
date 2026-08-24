//! IPC client for the LSP's live-analysis endpoint ([MCP-IPC-CLIENT],
//! [MCP-IPC-DISCOVERY]).
//!
//! `find-similar` and `list-embedding-models` cannot be served from the
//! state file alone — they need a live analysis pass. The MCP backend
//! delegates them (and every other read) to the LSP via a single-shot
//! JSON-RPC call over [`deslop_core::live::transport`]: the Unix
//! domain socket `.deslop/cache/deslop.sock` where it exists, or — on
//! Windows and wherever the LSP was started with `--ipc-transport
//! tcp` — the TCP loopback endpoint published in
//! `.deslop/cache/deslop.port` ([LIVE-IPC-TCP]). When neither endpoint
//! is live, the backend returns [`BackendError::LspNotRunning`] so
//! agents can offer to start the LSP.

use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
};

use deslop_core::live::transport::{self, IpcConnectError, IpcStream};
use serde_json::{json, Value};

use super::BackendError;

/// Sends one JSON-RPC request to the LSP IPC endpoint and returns the
/// `result` field. Errors with `LspNotRunning` when no endpoint is
/// live so agents can render an actionable hint.
///
/// # Errors
///
/// Returns [`BackendError::LspNotRunning`] when neither the socket nor
/// the TCP discovery record answers, [`BackendError::StateFileCorrupt`]
/// when the response is malformed, and [`BackendError::Core`]-style
/// errors propagated as [`BackendError::StateFileCorrupt`] when the
/// LSP returns a JSON-RPC `error` envelope. (`StateFileCorrupt`
/// doubles as the catch-all transport-failure variant — IPC failures
/// are reported the same way as a corrupt state file because both
/// indicate the LSP and MCP have drifted out of sync.) JSON-RPC
/// `-32601 method not found` from the LSP is surfaced with a
/// version-mismatch hint so agents diagnose stale bundled binaries
/// instead of generic "ipc rpc error" noise.
pub fn ipc_call(root: &Path, method: &str, params: &Value) -> Result<Value, BackendError> {
    let stream = connect(root)?;
    call_over(stream, method, params)
}

/// Opens a connection to whichever transport the LSP published for
/// `root` ([MCP-IPC-DISCOVERY]). On TCP the token handshake has
/// already happened when this returns.
///
/// # Errors
///
/// [`BackendError::LspNotRunning`] when no endpoint answers;
/// [`BackendError::StateFileCorrupt`] for live-but-broken endpoints.
pub fn connect(root: &Path) -> Result<IpcStream, BackendError> {
    transport::connect(root).map_err(|error| match error {
        IpcConnectError::NotRunning { socket_path, .. } => {
            BackendError::LspNotRunning { socket_path }
        }
        other => BackendError::StateFileCorrupt(format!("ipc connect failed: {other}")),
    })
}

/// Writes the request frame, reads one line back, and decodes the
/// JSON-RPC envelope.
fn call_over(mut stream: IpcStream, method: &str, params: &Value) -> Result<Value, BackendError> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let mut payload = serde_json::to_vec(&request)
        .map_err(|err| BackendError::StateFileCorrupt(format!("ipc serialize: {err}")))?;
    payload.push(b'\n');
    stream
        .write_all(&payload)
        .map_err(|err| BackendError::StateFileCorrupt(format!("ipc write: {err}")))?;
    stream
        .flush()
        .map_err(|err| BackendError::StateFileCorrupt(format!("ipc flush: {err}")))?;
    let mut line = String::new();
    let _bytes = BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|err| BackendError::StateFileCorrupt(format!("ipc read: {err}")))?;
    decode_response(&line, method)
}

/// Parses the JSON-RPC response envelope, mapping `-32601` to the
/// version-mismatch hint.
fn decode_response(line: &str, method: &str) -> Result<Value, BackendError> {
    let response: Value = serde_json::from_str(line.trim()).map_err(|err| {
        BackendError::StateFileCorrupt(format!("ipc response not valid JSON: {err}"))
    })?;
    if let Some(error) = response.get("error") {
        if error.get("code").and_then(Value::as_i64) == Some(-32_601) {
            return Err(BackendError::StateFileCorrupt(format!(
                "LSP rejected {method:?} with method-not-found (-32601). The LSP and MCP binaries are from different Deslop releases — reinstall the Deslop VSIX so both come from the same bundle. See https://github.com/Nimblesite/Deslop/issues/148."
            )));
        }
        return Err(BackendError::StateFileCorrupt(format!(
            "ipc rpc error: {error}"
        )));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| BackendError::StateFileCorrupt("ipc response missing result".to_owned()))
}
