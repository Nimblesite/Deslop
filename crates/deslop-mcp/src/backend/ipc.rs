//! IPC client for the LSP's `.deslop-cache/deslop.sock` server
//! ([MCP-IPC-CLIENT]).
//!
//! `find-similar` and `list-embedding-models` cannot be served from the
//! state file alone — they need a live analysis pass. The MCP backend
//! delegates them to the LSP via a single-shot JSON-RPC call over a
//! Unix domain socket. When the socket is missing, the backend returns
//! [`BackendError::LspNotRunning`] so agents can offer to start the LSP.

use std::path::Path;

use serde_json::Value;

use super::BackendError;

/// Sends one JSON-RPC request to the LSP IPC socket and returns the
/// `result` field. Errors with `LspNotRunning` when the socket is
/// absent so agents can render an actionable hint.
///
/// # Errors
///
/// Returns [`BackendError::LspNotRunning`] when the socket is missing,
/// [`BackendError::StateFileCorrupt`] when the response is malformed,
/// and [`BackendError::Core`]-style errors propagated as
/// [`BackendError::StateFileCorrupt`] when the LSP returns a JSON-RPC
/// `error` envelope. (`StateFileCorrupt` doubles as the catch-all
/// transport-failure variant — IPC failures are reported the same way
/// as a corrupt state file because both indicate the LSP and MCP have
/// drifted out of sync.) JSON-RPC `-32601 method not found` from the
/// LSP is surfaced with a version-mismatch hint so agents diagnose
/// stale bundled binaries instead of generic "ipc rpc error" noise
/// ([Deslop#148]).
pub fn ipc_call(socket_path: &Path, method: &str, params: &Value) -> Result<Value, BackendError> {
    #[cfg(unix)]
    {
        unix::call(socket_path, method, params)
    }
    #[cfg(not(unix))]
    {
        let _ = (method, params);
        Err(BackendError::LspNotRunning {
            socket_path: socket_path.to_path_buf(),
        })
    }
}

#[cfg(unix)]
/// Unix-domain-socket implementation of the IPC client.
mod unix {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixStream,
        path::Path,
    };

    use serde_json::{json, Value};

    use crate::backend::BackendError;

    /// Connects to `socket_path`, sends one line-delimited JSON-RPC
    /// request, reads one line back, and returns the `result` field.
    pub fn call(socket_path: &Path, method: &str, params: &Value) -> Result<Value, BackendError> {
        let mut stream = UnixStream::connect(socket_path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound
                || err.kind() == std::io::ErrorKind::ConnectionRefused
            {
                BackendError::LspNotRunning {
                    socket_path: socket_path.to_path_buf(),
                }
            } else {
                BackendError::StateFileCorrupt(format!("ipc connect failed: {err}"))
            }
        })?;
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
        let _bytes = BufReader::new(&stream)
            .read_line(&mut line)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc read: {err}")))?;
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
}
