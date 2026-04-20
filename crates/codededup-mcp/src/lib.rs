//! MCP (Model Context Protocol) server exposing `CodeDedup` live analysis
//! to AI agents over JSON-RPC 2.0 / stdio.
//!
//! Implements [mcp.md]: peer of the LSP shell ([lsp.md]), thin wrapper
//! around [`codededup_core::pipeline::PipelineSession`] (swapping to
//! `codededup_core::live::LiveApi` once P7 lands is a one-line change
//! inside [`backend`]).
//!
//! Implements:
//! - [MCP-CAPABILITIES] — tools + resources + notifications surface.
//! - [MCP-TOOLS] — eight agent-facing tools with JSON schemas.
//! - [MCP-TOOL-FINDSIMILAR] — keystone `find-similar` tool with two
//!   input variants and explicit error paths.
//! - [MCP-RESOURCES] — `codededup://report` + `codededup://schema`.
//! - [MCP-NOTIFICATIONS] — `notifications/resources/updated` +
//!   `notifications/codededup/reportChanged`.
//! - [MCP-SAFETY] — read-only by default, workspace-root pinned,
//!   no path traversal, no arbitrary command execution.
//! - [MCP-AGENT-PROMPT-GUIDANCE] — tool descriptions authored for an
//!   LLM planner, not a human reader.

pub mod backend;
pub mod protocol;
pub mod resources;
pub mod safety;
pub mod server;
pub mod tools;

pub use backend::{McpBackend, PipelineSessionBackend, SessionBackendConfig};
pub use protocol::{ErrorCode, JsonRpcError, JsonRpcRequest, JsonRpcResponse, RequestId};
pub use safety::{resolve_within_root, PathResolutionError};
pub use server::{McpServer, ServerError};

/// Semantic version of the `codededup-mcp` crate.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// MCP protocol version negotiated during `initialize`. Tracks the
/// latest stable revision of the [Model Context Protocol specification](https://modelcontextprotocol.io).
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name reported to the MCP client during the `initialize`
/// handshake. Consumers (Claude Code, Claude Desktop, Cursor, Continue)
/// display this string in their tool / resource UIs.
pub const MCP_SERVER_NAME: &str = "codededup-mcp";
