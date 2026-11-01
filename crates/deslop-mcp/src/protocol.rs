//! JSON-RPC 2.0 envelope re-exports + small constructor helpers.
//!
//! Every wire shape on the MCP transport is generated from
//! `docs/models/live-ipc.td` and re-exported through this module so the
//! consumers (`server.rs`, `tools/*`, `resources.rs`) keep one import
//! path. `JSONRPC_VERSION` is the single non-generated item — it's a
//! string constant (`"2.0"`), not a wire model.
//!
//! See `docs/plans/typeDiagram-migration.md` for the seven typeDiagram
//! features tracked at upstream nimblesite/typeDiagram#24..#30; each
//! `TYPE_CONFIG` workaround in `scripts/typediagram/` carries a
//! `TODO(typeDiagram#NN)` link back to the issue that retires it.

pub use deslop_core::wire_generated::{
    ErrorCode, JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, JsonRpcResponse, RequestId,
};
use serde::Serialize;
use serde_json::{json, Value};

/// Fixed JSON-RPC version string. The MCP spec mandates `"2.0"`.
pub const JSONRPC_VERSION: &str = "2.0";

/// [MCP-RESULT-SIZE-CAP] Budget for tool success and protocol error frames, including newline.
pub(crate) const MAX_RESPONSE_FRAME_BYTES: usize = 200 * 1024;
/// Explains why the full error payload could not be sent.
const OVERSIZED_RESPONSE_MESSAGE: &str = "response exceeded the MCP transport byte budget";
/// Tells callers how to recover details explicitly omitted from the bounded error.
const FULL_RESPONSE_RECOVERY: &str = "retry with a shorter request identifier and smaller tool arguments to retrieve the complete response and recovery guidance";

/// Serialize the complete frame exactly as emitted on stdio.
pub(crate) fn serialise_frame<T: Serialize>(value: &T) -> serde_json::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// [MCP-RESULT-SIZE-CAP] Bound error frames without changing full resource responses.
pub(crate) fn bounded_response_frame<T: Serialize>(value: &T) -> serde_json::Result<Vec<u8>> {
    let bytes = serialise_frame(value)?;
    if bytes.len() <= MAX_RESPONSE_FRAME_BYTES {
        return Ok(bytes);
    }
    let value = serde_json::to_value(value)?;
    if value.get("error").is_none() {
        return Ok(bytes);
    }
    tracing::warn!(
        original_bytes = bytes.len(),
        cap_bytes = MAX_RESPONSE_FRAME_BYTES,
        "mcp_error_response_too_large"
    );
    bounded_error_frame(&value)
}

/// Retain the request identifier when it fits, otherwise emit a bounded null-id error.
fn bounded_error_frame(value: &Value) -> serde_json::Result<Vec<u8>> {
    let id = value
        .get("id")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok());
    let mut response = jsonrpc_error_response(id, oversized_response_error(value));
    let bounded = serialise_frame(&response)?;
    if bounded.len() <= MAX_RESPONSE_FRAME_BYTES {
        return Ok(bounded);
    }
    response.id = None;
    serialise_frame(&response)
}

/// Preserve the error category and disclose omitted details with recovery guidance.
fn oversized_response_error(value: &Value) -> JsonRpcError {
    let mut error = jsonrpc_error(ErrorCode::InvalidRequest, OVERSIZED_RESPONSE_MESSAGE);
    if let Some(code) = value
        .pointer("/error/code")
        .and_then(Value::as_i64)
        .and_then(|code| i32::try_from(code).ok())
    {
        error.code = code;
    }
    error.data = Some(
        json!({"truncated": true, "truncated_reason": OVERSIZED_RESPONSE_MESSAGE,
        "truncated_at_bytes": MAX_RESPONSE_FRAME_BYTES, "next_action": FULL_RESPONSE_RECOVERY}),
    );
    error
}

/// Builds a success [`JsonRpcResponse`] for `id` with `result`.
#[must_use]
pub fn jsonrpc_response_ok(id: RequestId, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION.to_owned(),
        id,
        result,
    }
}

/// Builds an error-shaped [`JsonRpcErrorResponse`] for `id` with `error`.
#[must_use]
pub fn jsonrpc_error_response(id: Option<RequestId>, error: JsonRpcError) -> JsonRpcErrorResponse {
    JsonRpcErrorResponse {
        jsonrpc: JSONRPC_VERSION.to_owned(),
        id,
        error,
    }
}

/// Builds a new [`JsonRpcError`] with no `data`.
#[must_use]
pub fn jsonrpc_error(code: ErrorCode, message: impl Into<String>) -> JsonRpcError {
    JsonRpcError {
        code: code as i32,
        message: message.into(),
        data: None,
    }
}
