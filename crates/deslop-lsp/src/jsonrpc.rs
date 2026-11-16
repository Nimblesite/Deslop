//! JSON-RPC fault vocabulary shared by every handler ([LSP-TRANSPORT]).
//!
//! The error object itself is `lspkit-server`'s; this module only names the
//! reserved codes Deslop answers with, so no handler spells a number.

pub use lspkit_server::JsonRpcError as Error;

/// Result every request handler returns.
pub type LspResult<T> = Result<T, Error>;

/// The request is not valid in the server's current lifecycle phase.
pub const INVALID_REQUEST: i32 = -32_600;
/// No handler is registered for the method.
pub const METHOD_NOT_FOUND: i32 = -32_601;
/// The params could not be read as the method's parameter type.
pub const INVALID_PARAMS: i32 = -32_602;
/// The handler failed.
pub const INTERNAL_ERROR: i32 = -32_603;
/// The client cancelled the request.
pub const REQUEST_CANCELLED: i32 = -32_800;
/// A request arrived before `initialize` was answered.
pub const SERVER_NOT_INITIALIZED: i32 = -32_002;

/// `-32600 Invalid request`.
#[must_use]
pub fn invalid_request() -> Error {
    Error::new(INVALID_REQUEST, "Invalid request")
}

/// `-32601 Method not found`.
#[must_use]
pub fn method_not_found() -> Error {
    Error::new(METHOD_NOT_FOUND, "Method not found")
}

/// `-32602 Invalid params` carrying `message`.
#[must_use]
pub fn invalid_params(message: impl Into<String>) -> Error {
    Error::new(INVALID_PARAMS, message)
}

/// `-32603 Internal error` carrying `message`.
#[must_use]
pub fn internal_error(message: impl Into<String>) -> Error {
    Error::new(INTERNAL_ERROR, message)
}

/// `-32002 Server not initialized`.
#[must_use]
pub fn not_initialized() -> Error {
    Error::new(SERVER_NOT_INITIALIZED, "Server not initialized")
}

/// `-32800 Request cancelled`.
#[must_use]
pub fn request_cancelled() -> Error {
    Error::new(REQUEST_CANCELLED, "Request cancelled")
}
