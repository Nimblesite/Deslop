//! IPC server for the MCP bridge ([LSP-IPC], [LIVE-IPC-SOCKET],
//! [LIVE-IPC-TCP]).
//!
//! The LSP exposes the live in-memory analysis state to the MCP server
//! through [`deslop_core::live::transport`]: a Unix domain socket at
//! `.deslop-cache/deslop.sock` by default, or TCP loopback (published
//! in `.deslop-cache/deslop.port`) on Windows and under
//! `--ipc-transport tcp` ([MCP-IPC-CLIENT]). No on-disk cache is
//! involved on the read path. Methods served:
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
//! Protocol: line-delimited JSON-RPC 2.0. TCP clients prepend one
//! token line (from the discovery record), verified before any
//! JSON-RPC is read.

mod dispatch;

use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use deslop_core::live::{
    transport::{authenticate, IpcConnection, IpcMode, IpcServerListener, IpcStream},
    LiveService, ReportChangedNotification,
};
use serde_json::{json, Value};
use tokio::{runtime::Handle, sync::broadcast};

/// IPC server bound under the workspace's `.deslop-cache`.
///
/// Dropping this value removes the endpoint artifacts (socket file or
/// TCP discovery record) from the filesystem.
#[derive(Debug)]
pub struct IpcServer {
    /// Endpoint artifacts removed on drop.
    cleanup_paths: Vec<PathBuf>,
}

impl IpcServer {
    /// Binds the requested transport and starts the accept loop on a
    /// background thread.
    ///
    /// `report_changed` is the scheduler's broadcast sender; the
    /// `report/subscribe` arm calls `subscribe()` on it once per
    /// long-lived subscriber connection.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] when the cache directory cannot be
    /// created or the endpoint bind fails.
    pub fn start(
        root: &Path,
        transport: IpcMode,
        service: Arc<LiveService>,
        report_changed: broadcast::Sender<ReportChangedNotification>,
    ) -> Result<Self, std::io::Error> {
        let listener = IpcServerListener::bind(root, transport)?;
        let server = Self {
            cleanup_paths: listener.cleanup_paths(root),
        };
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
        for path in &self.cleanup_paths {
            let _removed = std::fs::remove_file(path);
            tracing::debug!(path = %path.display(), "ipc_endpoint_removed");
        }
    }
}

/// Spawns the accept loop on a dedicated thread. The thread exits when
/// the listener is closed (server shutdown).
fn spawn_accept_loop(
    listener: IpcServerListener,
    service: Arc<LiveService>,
    report_changed: broadcast::Sender<ReportChangedNotification>,
    handle: Handle,
) {
    let _thread = std::thread::spawn(move || loop {
        match listener.accept() {
            Ok(connection) => spawn_connection(
                connection,
                Arc::clone(&service),
                report_changed.clone(),
                handle.clone(),
            ),
            Err(error) => {
                tracing::warn!(%error, "ipc_accept_error");
                break;
            }
        }
    });
}

/// Spawns a thread to handle a single client connection.
fn spawn_connection(
    connection: IpcConnection,
    service: Arc<LiveService>,
    report_changed: broadcast::Sender<ReportChangedNotification>,
    handle: Handle,
) {
    let _thread = std::thread::spawn(move || {
        handle_connection(connection, &service, &report_changed, &handle);
    });
}

/// Verifies the transport token (TCP only), reads one JSON-RPC line
/// from the client, then either runs a single-shot dispatch (write
/// response, close) or — for `report/subscribe` — keeps the connection
/// open and forwards broadcast notifications until the client
/// disconnects.
fn handle_connection(
    connection: IpcConnection,
    service: &Arc<LiveService>,
    report_changed: &broadcast::Sender<ReportChangedNotification>,
    handle: &Handle,
) {
    let IpcConnection {
        stream,
        expected_token,
    } = connection;
    let Some((mut writer, mut reader)) = split_stream(stream) else {
        return;
    };
    if !authenticate(&mut reader, expected_token.as_deref()) {
        return;
    }
    let Some(request) = read_request(&mut reader, &mut writer) else {
        return;
    };
    respond(&request, &mut writer, service, report_changed, handle);
}

/// Clones the connection into a write half and a buffered read half.
fn split_stream(stream: IpcStream) -> Option<(IpcStream, BufReader<IpcStream>)> {
    match stream.try_clone() {
        Ok(writer) => Some((writer, BufReader::new(stream))),
        Err(error) => {
            tracing::warn!(%error, "ipc_stream_clone_failed");
            None
        }
    }
}

/// Reads and parses one JSON-RPC request line. Answers a parse-error
/// frame and returns `None` when the payload is not valid JSON.
fn read_request(reader: &mut BufReader<IpcStream>, writer: &mut IpcStream) -> Option<Value> {
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return None;
    }
    if let Ok(value) = serde_json::from_str(line.trim()) {
        Some(value)
    } else {
        let _written = write_frame(writer, &parse_error());
        None
    }
}

/// Routes a parsed request to the subscribe loop or single-shot
/// dispatch and writes the response frame.
fn respond(
    request: &Value,
    writer: &mut IpcStream,
    service: &Arc<LiveService>,
    report_changed: &broadcast::Sender<ReportChangedNotification>,
    handle: &Handle,
) {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    if method == "report/subscribe" {
        run_subscribe_loop(writer, &id, service, report_changed, handle);
        return;
    }
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let result = dispatch::dispatch(method, &params, service, handle);
    let response = json_rpc_response(&id, result);
    let _written = write_frame(writer, &response);
}

/// Long-lived broadcast forwarder. Sends one JSON-RPC notification
/// frame per [`ReportChangedNotification`] received. The ack frame
/// carries the current generation so the subscriber can sync its
/// own counter without a follow-up IPC call ([MCP-IPC-CLIENT]).
/// Returns when the client disconnects (broken pipe) or the
/// broadcast channel closes.
fn run_subscribe_loop(
    writer: &mut IpcStream,
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
    if write_frame(writer, &ack).is_err() {
        return;
    }
    let mut receiver = report_changed.subscribe();
    loop {
        let next = handle.block_on(receiver.recv());
        match next {
            Ok(notification) => {
                let frame = subscribe_notification_frame(&notification);
                if write_frame(writer, &frame).is_err() {
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
fn write_frame(stream: &mut IpcStream, value: &Value) -> std::io::Result<()> {
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
