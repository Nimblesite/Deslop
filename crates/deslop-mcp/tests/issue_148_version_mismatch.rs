//! [Deslop#148] Surfaces an actionable LSP/MCP version-mismatch hint
//! when the LSP responds to a tool-driving IPC method with JSON-RPC
//! `-32601 method not found`.
//!
//! Without this guard, agents see `MCP error -32004: ipc transport
//! failure: ipc rpc error: {...,"code":-32601,...}` and assume the
//! server is "offline". The new path names the rejected method, the
//! `-32601` code, and points at the VSIX so the fix is discoverable.

#![cfg(unix)]

use std::sync::Arc;

use anyhow::{ensure, Result};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::common;
use common::stub_lsp::{bind_stub_lsp, method_not_found};
use common::{error_and_message, initialized_mcp};

/// Clusters requested per page; the error path never reads them.
const PAGE_LIMIT: u64 = 5;

/// Calls `tool` against an LSP that knows no method and returns the
/// error message the MCP renders for it.
fn rejection_message_for(tool: &str, arguments: &Value) -> Result<String> {
    let workspace = TempDir::new()?;
    bind_stub_lsp(workspace.path(), Arc::new(method_not_found))?;
    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": tool, "arguments": arguments }),
    )?;
    let (_error, message) = error_and_message(&response)?;
    Ok(message)
}

/// The hint, element by element: the rejected method, the JSON-RPC code
/// users can grep their LSP logs for, and the VSIX reinstall as the fix.
fn ensure_version_mismatch_hint(message: &str, method: &str) -> Result<()> {
    ensure!(
        message.contains(method),
        "error must name the rejected method so users can match logs: {message}"
    );
    ensure!(
        message.contains("-32601"),
        "error must echo the JSON-RPC code so users can grep their LSP logs: {message}"
    );
    ensure!(
        message.contains("VSIX"),
        "error must point at the VSIX reinstall as the fix: {message}"
    );
    ensure!(
        !message.contains("ipc rpc error: {"),
        "error must not fall through to the generic catch-all: {message}"
    );
    Ok(())
}

#[test]
fn issue_148_top_offenders_reports_version_mismatch_when_lsp_rejects_report_get() -> Result<()> {
    let message = rejection_message_for(
        "duplicates",
        &json!({ "offset": 0, "limit": PAGE_LIMIT, "detail": "summary" }),
    )?;
    ensure_version_mismatch_hint(&message, "report/get")
}

#[test]
fn issue_148_session_config_reports_version_mismatch_when_lsp_rejects_method() -> Result<()> {
    let message = rejection_message_for("session", &json!({}))?;
    ensure_version_mismatch_hint(&message, "session/config")
}
