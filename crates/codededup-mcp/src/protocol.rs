//! JSON-RPC 2.0 envelope types for the MCP server.
//!
//! MCP rides on JSON-RPC 2.0. We model requests / responses /
//! notifications / errors directly instead of pulling in a dedicated
//! JSON-RPC crate so every byte crossing the stdio boundary stays
//! under our review.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Fixed JSON-RPC version string. The MCP spec mandates `"2.0"`.
pub const JSONRPC_VERSION: &str = "2.0";

/// Request or notification identifier. MCP permits strings, integers,
/// or `null`; we model the three via an untagged enum so the wire
/// representation round-trips without a custom (de)serialiser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric identifier (most JSON-RPC clients).
    Number(i64),
    /// String identifier.
    String(String),
}

/// Incoming JSON-RPC request frame (request or notification).
///
/// A *notification* is a request without an `id`; handlers must not
/// reply. A *request* carries an `id` and expects exactly one response.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    /// Must equal [`JSONRPC_VERSION`].
    pub jsonrpc: String,
    /// Method name, e.g. `"tools/list"`.
    pub method: String,
    /// Optional parameters payload.
    #[serde(default)]
    pub params: Option<Value>,
    /// Request identifier. Absent for notifications.
    #[serde(default)]
    pub id: Option<RequestId>,
}

/// Successful JSON-RPC response frame.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    /// Must equal [`JSONRPC_VERSION`].
    pub jsonrpc: &'static str,
    /// Echo of the request's `id`.
    pub id: RequestId,
    /// Method-specific result payload.
    pub result: Value,
}

impl JsonRpcResponse {
    /// Builds a success response for `id` with `result`.
    #[must_use]
    pub const fn ok(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result,
        }
    }
}

/// Error-shaped JSON-RPC response frame. Distinct struct (not a
/// `Result` variant) so `serde_json::to_vec` emits the exact JSON-RPC
/// shape: `{ jsonrpc, id, error }` with **no** `result` key.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcErrorResponse {
    /// Must equal [`JSONRPC_VERSION`].
    pub jsonrpc: &'static str,
    /// Echo of the request's `id` (or `null` when the request was
    /// unparseable).
    pub id: Option<RequestId>,
    /// Structured error payload.
    pub error: JsonRpcError,
}

impl JsonRpcErrorResponse {
    /// Builds an error response for `id` with `error`.
    #[must_use]
    pub const fn new(id: Option<RequestId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            error,
        }
    }
}

/// JSON-RPC error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code — see [`ErrorCode`] for the canonical set.
    pub code: i32,
    /// Short human-readable message.
    pub message: String,
    /// Optional structured payload (tree-sitter error ranges, list of
    /// supported languages, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Constructs a new error with no `data`.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: None,
        }
    }

    /// Constructs a new error with a structured `data` payload.
    #[must_use]
    pub fn with_data(code: ErrorCode, message: impl Into<String>, data: Value) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Canonical JSON-RPC / MCP error codes.
///
/// `-32700..=-32600` are JSON-RPC 2.0 reserved; `-32000..=-32099` are
/// reserved by JSON-RPC for server-defined errors and are where MCP-
/// specific codes live.
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    /// Invalid JSON received by the server.
    ParseError = -32_700,
    /// The JSON sent is not a valid Request object.
    InvalidRequest = -32_600,
    /// The method does not exist / is not available.
    MethodNotFound = -32_601,
    /// Invalid method parameter(s).
    InvalidParams = -32_602,
    /// Internal JSON-RPC error.
    InternalError = -32_603,
    /// `find-similar` received a snippet tree-sitter could not parse
    /// ([MCP-TOOL-FINDSIMILAR]).
    UnparseableInput = -32_001,
    /// `find-similar` received a language id the session does not
    /// know ([MCP-TOOL-FINDSIMILAR]).
    UnsupportedLanguage = -32_002,
    /// Path argument resolved outside the workspace root
    /// ([MCP-SAFETY]).
    PathOutsideRoot = -32_003,
    /// Tool call failed at the `LiveApi` / session layer.
    BackendError = -32_004,
}

/// Server → client notification frame (no `id`).
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    /// Must equal [`JSONRPC_VERSION`].
    pub jsonrpc: &'static str,
    /// Notification method, e.g. `"notifications/resources/updated"`.
    pub method: String,
    /// Method-specific parameters.
    pub params: Value,
}

impl JsonRpcNotification {
    /// Builds a notification with the given method + params.
    #[must_use]
    pub const fn new(method: String, params: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            method,
            params,
        }
    }
}
