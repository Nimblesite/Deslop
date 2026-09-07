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
use serde_json::json;
use tempfile::TempDir;

use crate::common;
use common::stub_lsp::{bind_stub_lsp, method_not_found};
use common::{error_and_message, initialized_mcp, request_duplicates_summary};

/// Clusters requested per page; the error path never reads them.
const PAGE_LIMIT: u64 = 5;

#[test]
fn issue_148_top_offenders_reports_version_mismatch_when_lsp_rejects_report_get() -> Result<()> {
    let workspace = TempDir::new()?;
    bind_stub_lsp(workspace.path(), Arc::new(method_not_found))?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = request_duplicates_summary(&mut mcp, PAGE_LIMIT)?;

    let (_error, message) = error_and_message(&response)?;
    ensure!(
        message.contains("report/get"),
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
fn issue_148_session_config_reports_version_mismatch_when_lsp_rejects_method() -> Result<()> {
    let workspace = TempDir::new()?;
    bind_stub_lsp(workspace.path(), Arc::new(method_not_found))?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request("tools/call", &json!({ "name": "session", "arguments": {} }))?;

    let (_error, message) = error_and_message(&response)?;
    ensure!(
        message.contains("session/config"),
        "error must name the rejected method: {message}"
    );
    ensure!(
        message.contains("-32601"),
        "error must echo the JSON-RPC code: {message}"
    );
    ensure!(
        message.contains("VSIX"),
        "error must point at the VSIX reinstall: {message}"
    );
    Ok(())
}
