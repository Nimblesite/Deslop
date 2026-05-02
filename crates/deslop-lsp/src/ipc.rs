//! Unix-domain socket IPC server ([LSP-IPC]).
//!
//! The LSP exposes `.deslop-cache/deslop.sock` so the MCP server can
//! delegate compute-heavy operations (`duplicates/findSimilar`,
//! `embedding/listModels`) without running its own analysis pass.
//!
//! Protocol: line-delimited JSON-RPC 2.0 — one request per line,
//! one response per line.

#[cfg(unix)]
/// Unix-domain socket IPC server implementation.
mod unix {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::{UnixListener, UnixStream},
        path::{Path, PathBuf},
        sync::Arc,
    };

    use deslop_core::live::{LiveApi, LiveService};
    use serde_json::{json, Value};
    use tokio::runtime::Handle;

    /// IPC server bound to `.deslop-cache/deslop.sock`.
    ///
    /// Dropping this value removes the socket file from the filesystem.
    #[derive(Debug)]
    pub struct IpcServer {
        /// Absolute path to the Unix domain socket file. Removed on drop.
        socket_path: PathBuf,
    }

    impl IpcServer {
        /// Binds the socket and starts the accept loop on a background thread.
        ///
        /// # Errors
        ///
        /// Returns [`std::io::Error`] when the cache directory cannot be
        /// created or the socket bind fails.
        pub fn start(root: &Path, service: Arc<LiveService>) -> Result<Self, std::io::Error> {
            let socket_path = root.join(".deslop-cache").join("deslop.sock");
            std::fs::create_dir_all(root.join(".deslop-cache"))?;
            let _removed = std::fs::remove_file(&socket_path);
            let listener = UnixListener::bind(&socket_path)?;
            tracing::info!(path = %socket_path.display(), "ipc_socket_bound");
            let server = Self { socket_path };
            // Capture the tokio handle here — IpcServer::start is called from
            // the tokio runtime. The accept/connection threads are plain OS
            // threads and Handle::try_current() would fail inside them.
            let handle = Handle::current();
            spawn_accept_loop(listener, service, handle);
            Ok(server)
        }
    }

    impl Drop for IpcServer {
        fn drop(&mut self) {
            let _removed = std::fs::remove_file(&self.socket_path);
            tracing::debug!(path = %self.socket_path.display(), "ipc_socket_removed");
        }
    }

    /// Spawns the accept loop on a dedicated thread. The thread exits when
    /// the listener is closed (server shutdown).
    fn spawn_accept_loop(listener: UnixListener, service: Arc<LiveService>, handle: Handle) {
        let _thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => spawn_connection(stream, Arc::clone(&service), handle.clone()),
                    Err(error) => {
                        tracing::warn!(%error, "ipc_accept_error");
                        break;
                    }
                }
            }
        });
    }

    /// Spawns a thread to handle a single client connection.
    fn spawn_connection(stream: UnixStream, service: Arc<LiveService>, handle: Handle) {
        let _thread = std::thread::spawn(move || handle_connection(&stream, &service, &handle));
    }

    /// Reads one JSON-RPC line, dispatches it, writes the response, then
    /// closes the connection. Short-lived by design.
    fn handle_connection(stream: &UnixStream, service: &Arc<LiveService>, handle: &Handle) {
        let peer = stream.try_clone();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        let response = handle_line(line.trim(), service, handle);
        let mut writer = match peer {
            Ok(w) => w,
            Err(error) => {
                tracing::warn!(%error, "ipc_stream_clone_failed");
                return;
            }
        };
        let mut payload = serde_json::to_vec(&response).unwrap_or_default();
        payload.push(b'\n');
        let _written = writer.write_all(&payload);
    }

    /// Parses one JSON-RPC line and returns the response value.
    fn handle_line(line: &str, service: &Arc<LiveService>, handle: &Handle) -> Value {
        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return parse_error(),
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let result = dispatch(method, params, service, handle);
        json_rpc_response(&id, result)
    }

    /// Routes a JSON-RPC method to the appropriate [`LiveService`] call.
    fn dispatch(
        method: &str,
        params: Value,
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        match method {
            "duplicates/findSimilar" => dispatch_find_similar(params, service, handle),
            "embedding/listModels" => dispatch_list_models(service, handle),
            _ => Err(json!({"code": -32601, "message": "method not found"})),
        }
    }

    /// Delegates `duplicates/findSimilar` to [`LiveService::find_similar`].
    fn dispatch_find_similar(
        params: Value,
        service: &Arc<LiveService>,
        handle: &tokio::runtime::Handle,
    ) -> Result<Value, Value> {
        let request: deslop_core::live::FindSimilarRequest = serde_json::from_value(params)
            .map_err(|e| json!({"code": -32602, "message": format!("invalid params: {e}")}))?;
        let result = handle
            .block_on(service.find_similar(&request))
            .map_err(|e| json!({"code": -32603, "message": e.to_string()}))?;
        serde_json::to_value(&result).map_err(|e| json!({"code": -32603, "message": e.to_string()}))
    }

    /// Delegates `embedding/listModels` to [`LiveService::embedding_list_models`].
    fn dispatch_list_models(
        service: &Arc<LiveService>,
        handle: &tokio::runtime::Handle,
    ) -> Result<Value, Value> {
        let models = handle.block_on(service.embedding_list_models());
        serde_json::to_value(&models).map_err(|e| json!({"code": -32603, "message": e.to_string()}))
    }

    /// Wraps a `Result<Value, Value>` into a JSON-RPC 2.0 response envelope.
    fn json_rpc_response(id: &Value, result: Result<Value, Value>) -> Value {
        match result {
            Ok(value) => json!({"jsonrpc": "2.0", "id": id, "result": value}),
            Err(error) => json!({"jsonrpc": "2.0", "id": id, "error": error}),
        }
    }

    /// Returns a JSON-RPC parse-error response (id unknown).
    fn parse_error() -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": Value::Null,
            "error": {"code": -32700, "message": "parse error"},
        })
    }
}

#[cfg(unix)]
pub use unix::IpcServer;

/// Stub for non-Unix targets so the rest of the crate compiles.
#[cfg(not(unix))]
#[derive(Debug)]
pub struct IpcServer;

#[cfg(not(unix))]
impl IpcServer {
    /// No-op on non-Unix platforms.
    ///
    /// # Errors
    ///
    /// Always returns `Ok`.
    pub fn start(
        _root: &std::path::Path,
        _service: std::sync::Arc<deslop_core::live::LiveService>,
    ) -> Result<Self, std::io::Error> {
        Ok(Self)
    }
}
