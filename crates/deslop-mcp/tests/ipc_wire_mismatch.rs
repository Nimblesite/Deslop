//! [MCP-IPC-WIRE-MISMATCH] A report the MCP cannot read is refused by
//! name. The stub LSP serves the report a release before `weight`
//! became `mass` would serve, and every tool that reads it answers with
//! the one named condition — both versions, the endpoint, the remedy —
//! never with a raw field error, and never with an empty page an agent
//! could read as "no duplicates" (gh #523).

#![cfg(unix)]

use std::sync::Arc;

use anyhow::{ensure, Result};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::common;
use common::stub_lsp::{bind_stub_lsp, method_not_found, Reply};
use common::{
    error_and_message, expected_socket_fragment, initialized_mcp, request_duplicates_summary,
};

/// A report written by the wire before `weight` became `mass`, stamped
/// by the release that wrote it.
const LEGACY_REPORT: &str = include_str!("fixtures/legacy-weight-report.json");
/// The producer version the legacy fixture is stamped with.
const LEGACY_TOOL_VERSION: &str = "0.31.0";
/// The one cluster the legacy fixture carries.
const LEGACY_CLUSTER_ID: &str = "a5b1c68838489f44";
/// This test binary shares the workspace version with `deslop-mcp`.
const MCP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The whole-report IPC method every read tool issues.
const REPORT_GET: &str = "report/get";
/// The drill-in tool, which reads the same report.
const CLUSTER_BY_ID_TOOL: &str = "cluster-by-id";
/// The remedy the message must name.
const REMEDY: &str = "reinstall the Deslop VSIX";
/// The message the guard replaced.
const OPAQUE_PARSE_PREFIX: &str = "ipc report parse";
/// The prefix of every raw serde field error.
const FIELD_ERROR: &str = "missing field";
/// Clusters requested per page; the error path never reads them.
const PAGE_LIMIT: u64 = 5;

/// An LSP from the release that wrote the legacy fixture: it serves that
/// report and knows nothing else.
fn legacy_release_lsp(method: &str) -> Reply {
    if method == REPORT_GET {
        return Reply::Result(
            serde_json::from_str(LEGACY_REPORT).expect("the legacy fixture is JSON"),
        );
    }
    method_not_found(method)
}

/// The named condition, element by element.
fn ensure_named_mismatch(message: &str, socket: &str) -> Result<()> {
    let mcp = format!("deslop-mcp {MCP_VERSION}");
    let engine = format!("deslop-lsp {LEGACY_TOOL_VERSION}");
    ensure!(
        message.contains(&mcp),
        "must name this binary's version: {message}"
    );
    ensure!(
        message.contains(&engine),
        "must name the reply's producer: {message}"
    );
    ensure!(
        message.contains(socket),
        "must name the endpoint: {message}"
    );
    ensure!(message.contains(REMEDY), "must name the remedy: {message}");
    ensure!(
        message.contains(REPORT_GET),
        "must name the IPC method: {message}"
    );
    ensure!(
        !message.contains(FIELD_ERROR),
        "refused before decoding, so no raw field error: {message}"
    );
    ensure!(
        !message.contains(OPAQUE_PARSE_PREFIX),
        "the opaque message must never reach a caller: {message}"
    );
    Ok(())
}

/// The response must be an error frame, never a page: an empty page
/// reads as "no duplicates", which is the silent failure the guard exists
/// to prevent.
fn ensure_no_page(response: &Value) -> Result<()> {
    ensure!(
        response.pointer("/result").is_none(),
        "a mismatched reply must not produce a result page: {response}"
    );
    Ok(())
}

#[test]
fn duplicates_over_a_report_from_another_release_names_both_versions_and_the_remedy() -> Result<()>
{
    let workspace = TempDir::new()?;
    bind_stub_lsp(workspace.path(), Arc::new(legacy_release_lsp))?;
    let mut mcp = initialized_mcp(workspace.path())?;

    let response = request_duplicates_summary(&mut mcp, PAGE_LIMIT)?;

    ensure_no_page(&response)?;
    let (_error, message) = error_and_message(&response)?;
    ensure_named_mismatch(&message, &expected_socket_fragment(workspace.path())?)
}

#[test]
fn cluster_by_id_over_a_report_from_another_release_names_the_same_condition() -> Result<()> {
    let workspace = TempDir::new()?;
    bind_stub_lsp(workspace.path(), Arc::new(legacy_release_lsp))?;
    let mut mcp = initialized_mcp(workspace.path())?;

    let response = mcp.request(
        "tools/call",
        &json!({ "name": CLUSTER_BY_ID_TOOL, "arguments": { "id": LEGACY_CLUSTER_ID } }),
    )?;

    ensure_no_page(&response)?;
    let (_error, message) = error_and_message(&response)?;
    ensure_named_mismatch(&message, &expected_socket_fragment(workspace.path())?)
}
