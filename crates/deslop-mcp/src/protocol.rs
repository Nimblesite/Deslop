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
//! `TYPE_CONFIG` workaround in `scripts/typediagram-gen/` carries a
//! `TODO(typeDiagram#NN)` link back to the issue that retires it.

use serde_json::Value;

pub use deslop_core::wire_generated::{
    ErrorCode, JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, JsonRpcResponse, RequestId,
};

/// Fixed JSON-RPC version string. The MCP spec mandates `"2.0"`.
pub const JSONRPC_VERSION: &str = "2.0";

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
