//! Unix-domain socket IPC server ([LSP-IPC], [LIVE-IPC-SOCKET]).
//!
//! The LSP exposes `.deslop-cache/deslop.sock` so the MCP server can
//! query the live in-memory analysis state directly — no on-disk cache
//! involved on the read path ([MCP-IPC-CLIENT]). Methods served:
//!
//! Single-shot reads (one request → one response, connection closes):
//! - `report/get`, `report/forFile`, `report/forRange`,
//!   `cluster/byId`, `session/config`
//!
//! Single-shot compute:
//! - `duplicates/findSimilar`, `embedding/listModels`,
//!   `deslop.lsp.refreshReport`
//!
//! Long-lived subscription:
//! - `report/subscribe` — connection stays open; the server writes one
//!   JSON-RPC notification frame per generation bump until the client
//!   disconnects.
//!
//! Protocol: line-delimited JSON-RPC 2.0.

#[cfg(unix)]
/// Unix-domain socket IPC server implementation.
mod unix {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::{UnixListener, UnixStream},
        path::{Path, PathBuf},
        sync::Arc,
    };

    use deslop_core::live::{LiveApi, LiveService, ReportChangedNotification};
    use serde_json::{json, Value};
    use tokio::{runtime::Handle, sync::broadcast};

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
        /// `report_changed` is the scheduler's broadcast sender; the
        /// `report/subscribe` arm calls `subscribe()` on it once per
        /// long-lived subscriber connection.
        ///
        /// # Errors
        ///
        /// Returns [`std::io::Error`] when the cache directory cannot be
        /// created or the socket bind fails.
        pub fn start(
            root: &Path,
            service: Arc<LiveService>,
            report_changed: broadcast::Sender<ReportChangedNotification>,
        ) -> Result<Self, std::io::Error> {
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
            spawn_accept_loop(listener, service, report_changed, handle);
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
    fn spawn_accept_loop(
        listener: UnixListener,
        service: Arc<LiveService>,
        report_changed: broadcast::Sender<ReportChangedNotification>,
        handle: Handle,
    ) {
        let _thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => spawn_connection(
                        stream,
                        Arc::clone(&service),
                        report_changed.clone(),
                        handle.clone(),
                    ),
                    Err(error) => {
                        tracing::warn!(%error, "ipc_accept_error");
                        break;
                    }
                }
            }
        });
    }

    /// Spawns a thread to handle a single client connection.
    fn spawn_connection(
        stream: UnixStream,
        service: Arc<LiveService>,
        report_changed: broadcast::Sender<ReportChangedNotification>,
        handle: Handle,
    ) {
        let _thread = std::thread::spawn(move || {
            handle_connection(stream, &service, &report_changed, &handle);
        });
    }

    /// Reads one JSON-RPC line from the client, then either runs a
    /// single-shot dispatch (write response, close) or — for
    /// `report/subscribe` — keeps the connection open and forwards
    /// broadcast notifications until the client disconnects.
    fn handle_connection(
        stream: UnixStream,
        service: &Arc<LiveService>,
        report_changed: &broadcast::Sender<ReportChangedNotification>,
        handle: &Handle,
    ) {
        let writer = match stream.try_clone() {
            Ok(w) => w,
            Err(error) => {
                tracing::warn!(%error, "ipc_stream_clone_failed");
                return;
            }
        };
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        let request: Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(_) => {
                let _written = write_frame(&writer, &parse_error());
                return;
            }
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        if method == "report/subscribe" {
            run_subscribe_loop(writer, &id, service, report_changed, handle);
            return;
        }
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let result = dispatch(method, params, service, handle);
        let response = json_rpc_response(&id, result);
        let _written = write_frame(&writer, &response);
    }

    /// Long-lived broadcast forwarder. Sends one JSON-RPC notification
    /// frame per [`ReportChangedNotification`] received. The ack frame
    /// carries the current generation so the subscriber can sync its
    /// own counter without a follow-up IPC call ([MCP-IPC-CLIENT]).
    /// Returns when the client disconnects (broken pipe) or the
    /// broadcast channel closes.
    fn run_subscribe_loop(
        writer: UnixStream,
        id: &Value,
        service: &Arc<LiveService>,
        report_changed: &broadcast::Sender<ReportChangedNotification>,
        handle: &Handle,
    ) {
        let initial_generation = handle.block_on(async {
            let session = service.session();
            let guard = session.lock().await;
            guard.generation()
        });
        let ack = json_rpc_response(
            id,
            Ok(json!({"subscribed": true, "generation": initial_generation})),
        );
        if write_frame(&writer, &ack).is_err() {
            return;
        }
        let mut receiver = report_changed.subscribe();
        loop {
            let next = handle.block_on(receiver.recv());
            match next {
                Ok(notification) => {
                    let frame = subscribe_notification_frame(&notification);
                    if write_frame(&writer, &frame).is_err() {
                        tracing::debug!("ipc_subscribe_client_disconnected");
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "ipc_subscribe_lagged");
                }
            }
        }
    }

    /// Writes one line-delimited JSON frame to the client.
    fn write_frame(mut stream: &UnixStream, value: &Value) -> std::io::Result<()> {
        let mut payload = serde_json::to_vec(value).unwrap_or_default();
        payload.push(b'\n');
        stream.write_all(&payload)
    }

    /// Wraps a [`ReportChangedNotification`] in a JSON-RPC notification
    /// envelope (no `id`).
    fn subscribe_notification_frame(notification: &ReportChangedNotification) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": "report/changed",
            "params": notification,
        })
    }

    /// Routes a JSON-RPC method to the appropriate [`LiveService`] call.
    fn dispatch(
        method: &str,
        params: Value,
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        match method {
            "report/get" => dispatch_report_get(service, handle),
            "report/forFile" => dispatch_report_for_file(params, service, handle),
            "report/forRange" => dispatch_report_for_range(params, service, handle),
            "cluster/byId" => dispatch_cluster_by_id(params, service, handle),
            "session/config" => dispatch_session_config(service, handle),
            "duplicates/findSimilar" => dispatch_find_similar(params, service, handle),
            "embedding/listModels" => dispatch_list_models(service, handle),
            crate::commands::REFRESH_REPORT => dispatch_refresh_report(service, handle),
            _ => Err(json!({"code": -32601, "message": "method not found"})),
        }
    }

    /// Delegates `report/get` to [`LiveApi::report_get`].
    fn dispatch_report_get(
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        let report = handle.block_on(service.report_get());
        serde_json::to_value(report.as_ref()).map_err(rpc_serialise_error)
    }

    /// Delegates `report/forFile` to [`LiveApi::report_for_file`].
    fn dispatch_report_for_file(
        params: Value,
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        let path = params
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| json!({"code": -32602, "message": "missing path"}))?;
        let file_report = handle.block_on(service.report_for_file(Path::new(path)));
        serde_json::to_value(&file_report).map_err(rpc_serialise_error)
    }

    /// Delegates `report/forRange` to [`LiveApi::report_for_range`].
    fn dispatch_report_for_range(
        params: Value,
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        let path = params
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| json!({"code": -32602, "message": "missing path"}))?;
        let start_byte = params
            .get("start_byte")
            .and_then(Value::as_u64)
            .ok_or_else(|| json!({"code": -32602, "message": "missing start_byte"}))?;
        let end_byte = params
            .get("end_byte")
            .and_then(Value::as_u64)
            .ok_or_else(|| json!({"code": -32602, "message": "missing end_byte"}))?;
        let start = usize::try_from(start_byte)
            .map_err(|_| json!({"code": -32602, "message": "start_byte overflow"}))?;
        let end = usize::try_from(end_byte)
            .map_err(|_| json!({"code": -32602, "message": "end_byte overflow"}))?;
        let clusters = handle.block_on(service.report_for_range(Path::new(path), start, end));
        serde_json::to_value(&clusters).map_err(rpc_serialise_error)
    }

    /// Delegates `cluster/byId` to [`LiveApi::cluster_by_id`].
    fn dispatch_cluster_by_id(
        params: Value,
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        let id = params
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| json!({"code": -32602, "message": "missing id"}))?;
        let cluster = handle
            .block_on(service.cluster_by_id(id))
            .map_err(|err| json!({"code": -32603, "message": err.to_string()}))?;
        serde_json::to_value(&cluster).map_err(rpc_serialise_error)
    }

    /// Delegates `session/config` to [`LiveApi::session_config`].
    fn dispatch_session_config(
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        let config = handle.block_on(service.session_config());
        serde_json::to_value(&config).map_err(rpc_serialise_error)
    }

    /// Delegates `duplicates/findSimilar` to [`LiveService::find_similar`].
    fn dispatch_find_similar(
        params: Value,
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        let request: deslop_core::live::FindSimilarRequest = serde_json::from_value(params)
            .map_err(|e| json!({"code": -32602, "message": format!("invalid params: {e}")}))?;
        let result = handle
            .block_on(service.find_similar(&request))
            .map_err(|e| json!({"code": -32603, "message": e.to_string()}))?;
        serde_json::to_value(&result).map_err(rpc_serialise_error)
    }

    /// Delegates `embedding/listModels` to [`LiveService::embedding_list_models`].
    fn dispatch_list_models(
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        let models = handle.block_on(service.embedding_list_models());
        serde_json::to_value(&models).map_err(rpc_serialise_error)
    }

    /// Forces the same full refresh as `workspace/executeCommand`
    /// `deslop.lsp.refreshReport`, but over the MCP-facing IPC socket.
    fn dispatch_refresh_report(
        service: &Arc<LiveService>,
        handle: &Handle,
    ) -> Result<Value, Value> {
        handle.block_on(async {
            let session = service.session();
            let (previous_generation, previous_report, delta) = {
                let mut guard = session.lock().await;
                let previous_generation = guard.generation();
                let previous_report = guard.report();
                let delta = guard
                    .refresh_full()
                    .map_err(|error| json!({"code": -32603, "message": error.to_string()}))?;
                (previous_generation, previous_report, delta)
            };
            service
                .remember_snapshot(previous_generation, previous_report)
                .await;
            Ok(json!({
                "command": crate::commands::REFRESH_REPORT,
                "generation": delta.to_generation,
                "clustersAdded": delta.clusters_added.len(),
                "clustersRemoved": delta.clusters_removed.len(),
                "clustersUpdated": delta.clusters_updated.len(),
            }))
        })
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

    /// Maps a serialisation failure to a JSON-RPC internal-error envelope.
    fn rpc_serialise_error(err: serde_json::Error) -> Value {
        json!({"code": -32603, "message": err.to_string()})
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
        _report_changed: tokio::sync::broadcast::Sender<
            deslop_core::live::ReportChangedNotification,
        >,
    ) -> Result<Self, std::io::Error> {
        Ok(Self)
    }
}
