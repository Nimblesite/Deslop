//! JSON-RPC 2.0 / stdio server loop.
//!
//! MCP stdio transport: each JSON-RPC message is a single line of
//! UTF-8 JSON terminated by `\n`. One frame in; zero or one frames
//! out (notifications produce zero, responses produce one). Server →
//! client notifications are also line-delimited.
//!
//! This module is transport-only — routing lives in [`dispatch`], which
//! the server dispatches through. The server can be driven against
//! arbitrary `Read` / `Write` pairs (stdin/stdout in production; two
//! `Pipe` halves in tests) so the E2E suite drives the real binary.

use std::{
    io::{self, BufRead, BufReader, Read, Write},
    sync::{Arc, Mutex},
};

use serde_json::{json, Value};
use thiserror::Error;
use tracing::{debug, error, info};

use crate::{
    backend::McpBackend,
    notify::NotificationSender,
    protocol::{
        ErrorCode, JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, JsonRpcResponse, RequestId,
        JSONRPC_VERSION,
    },
    resources::{read_resource, resources_list_payload},
    tools::{dispatch_tool_call, tools_list_payload, wrap_tool_result},
    MCP_PROTOCOL_VERSION, MCP_SERVER_NAME,
};

/// Fatal server errors. Non-fatal per-request errors are surfaced as
/// JSON-RPC `error` frames instead.
#[derive(Debug, Error)]
pub enum ServerError {
    /// I/O failure on the transport (stdin closed mid-frame, etc.).
    #[error("transport I/O failure: {0}")]
    Io(#[from] io::Error),
}

/// Server instance. Owns the backend reference.
///
/// The output writer is supplied at [`run`](Self::run) time and wrapped
/// in a [`NotificationSender`] so background worker threads (embedding
/// refresh) can push notifications without waiting for the read loop.
#[derive(Debug)]
pub struct McpServer<B: McpBackend> {
    /// Shared backend. `Arc` so future async variants can share it.
    backend: Arc<B>,
}

impl<B: McpBackend> McpServer<B> {
    /// Constructs a new server bound to `backend`.
    #[must_use]
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }

    /// Drives the server over an explicit `(reader, writer)` pair.
    ///
    /// `writer` is boxed into a [`NotificationSender`] and wired into
    /// the backend so both the request loop and detached worker threads
    /// share the same serialised write path. Returns when EOF is
    /// reached or a fatal I/O error occurs.
    ///
    /// # Errors
    ///
    /// Returns [`ServerError::Io`] on unrecoverable transport failures.
    pub fn run<R: Read, W: Write + Send + 'static>(
        &self,
        reader: R,
        writer: W,
    ) -> Result<(), ServerError> {
        let sender: NotificationSender = Arc::new(Mutex::new(Box::new(writer)));
        self.backend.set_notification_sender(Arc::clone(&sender));
        let mut buffered = BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = buffered.read_line(&mut line)?;
            if bytes_read == 0 {
                info!("mcp_stdio_eof");
                return Ok(());
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.handle_frame(trimmed, &sender)?;
        }
    }

    /// Handles one JSON-RPC frame — request, notification, or a
    /// malformed line (which produces a parse-error response).
    fn handle_frame(&self, line: &str, sender: &NotificationSender) -> Result<(), ServerError> {
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(parse_err) => {
                error!(reason = %parse_err, "mcp_parse_error");
                let error = JsonRpcErrorResponse::new(
                    None,
                    JsonRpcError::new(ErrorCode::ParseError, parse_err.to_string()),
                );
                return write_json(sender, &error);
            }
        };
        if request.jsonrpc != JSONRPC_VERSION {
            let error = JsonRpcErrorResponse::new(
                request.id.clone(),
                JsonRpcError::new(
                    ErrorCode::InvalidRequest,
                    format!("unsupported jsonrpc version {:?}", request.jsonrpc),
                ),
            );
            return write_json(sender, &error);
        }
        request.id.clone().map_or_else(
            || {
                self.handle_notification(&request);
                Ok(())
            },
            |id| self.handle_request(id, &request, sender),
        )
    }

    /// Handles a request (id-bearing frame).
    fn handle_request(
        &self,
        id: RequestId,
        request: &JsonRpcRequest,
        sender: &NotificationSender,
    ) -> Result<(), ServerError> {
        let method = request.method.as_str();
        let params = request.params.clone().unwrap_or_else(|| json!({}));
        debug!(method = method, "mcp_request_received");
        let outcome = match method {
            "initialize" => Ok(Self::handle_initialize()),
            "initialized" | "notifications/initialized" | "ping" => Ok(json!({})),
            "shutdown" => Ok(json!(null)),
            "tools/list" => Ok(tools_list_payload()),
            "tools/call" => self.handle_tool_call(&params),
            "resources/list" => Ok(resources_list_payload()),
            "resources/read" => self.handle_resource_read(&params),
            other => Err(JsonRpcError::new(
                ErrorCode::MethodNotFound,
                format!("method {other:?} is not implemented"),
            )),
        };
        match outcome {
            Ok(result) => write_json(sender, &JsonRpcResponse::ok(id, result)),
            Err(error) => write_json(sender, &JsonRpcErrorResponse::new(Some(id), error)),
        }
    }

    /// Handles a notification (no `id`).
    ///
    /// `notifications/deslop/filesChanged` — carries `{ paths: [...] }`
    /// and re-runs analysis. The backend pushes `notifications/resources/updated`
    /// and `notifications/deslop/reportChanged` through its stored
    /// [`NotificationSender`] after the analysis completes.
    fn handle_notification(&self, request: &JsonRpcRequest) {
        debug!(method = %request.method, "mcp_notification_received");
        if request.method == "notifications/deslop/filesChanged" {
            if let Some(params) = request.params.as_ref() {
                let paths: Vec<std::path::PathBuf> = params
                    .get("paths")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(std::path::PathBuf::from)
                            .collect()
                    })
                    .unwrap_or_default();
                if !paths.is_empty() {
                    if let Err(err) = self.backend.mark_changed(&paths) {
                        error!(reason = %err, "mcp_mark_changed_failed");
                    }
                }
            }
        }
    }

    /// Builds the `initialize` response payload.
    fn handle_initialize() -> Value {
        json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": {
                    "subscribe": true,
                    "listChanged": false
                },
            },
            "serverInfo": {
                "name": MCP_SERVER_NAME,
                "version": crate::version(),
            },
        })
    }

    /// Handles `tools/call` — extracts `name` + `arguments` and
    /// forwards to [`dispatch_tool_call`].
    fn handle_tool_call(&self, params: &Value) -> Result<Value, JsonRpcError> {
        let name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "tools/call requires a 'name'")
        })?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let payload = dispatch_tool_call(self.backend.as_ref(), name, &arguments)?;
        Ok(wrap_tool_result(&payload))
    }

    /// Handles `resources/read` — extracts `uri` and forwards to
    /// [`read_resource`].
    fn handle_resource_read(&self, params: &Value) -> Result<Value, JsonRpcError> {
        let uri = params.get("uri").and_then(Value::as_str).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "resources/read requires a 'uri'")
        })?;
        read_resource(self.backend.as_ref(), uri)
    }
}

/// Serialises `value` and writes it as one newline-terminated frame
/// under the sender mutex so responses never interleave on the wire.
fn write_json<T: serde::Serialize>(
    sender: &NotificationSender,
    value: &T,
) -> Result<(), ServerError> {
    let bytes = serde_json::to_vec(value).map_err(|err| io_from_serde(&err))?;
    let mut writer = sender
        .lock()
        .map_err(|_| ServerError::Io(io::Error::other("mcp stdout mutex poisoned")))?;
    writer.write_all(&bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

/// Converts a `serde_json::Error` into an `io::Error` so the
/// transport layer has one error type.
fn io_from_serde(err: &serde_json::Error) -> io::Error {
    io::Error::other(format!("serde_json failure: {err}"))
}
