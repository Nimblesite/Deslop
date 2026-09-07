//! [MCP-IPC-WIRE-MISMATCH] A report the MCP cannot read is refused by
//! name. The stub LSP serves what another Deslop release serves — a
//! report carrying `weight` where this wire carries `mass` — and every
//! tool that reads it answers with the one named condition: both
//! versions, the endpoint, the remedy. Never a raw field error, never an
//! empty page an agent could read as "no duplicates" (gh #523).

#![cfg(unix)]

use std::{path::Path, sync::Arc};

use anyhow::{ensure, Result};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::common;
use common::stub_lsp::{bind_stub_lsp, method_not_found, Reply};
use common::{error_and_message, expected_socket_fragment, initialized_mcp};

/// A report another Deslop release wrote: stamped with that release's
/// version, carrying `weight` where this wire carries `mass`. It is the
/// input the guard must refuse — nothing in the product decodes it.
const FOREIGN_RELEASE_REPORT: &str = include_str!("fixtures/report-from-another-release.json");
/// The version the foreign report is stamped with.
const FOREIGN_RELEASE_VERSION: &str = "0.31.0";
/// The one cluster the foreign report carries.
const FOREIGN_RELEASE_CLUSTER_ID: &str = "a5b1c68838489f44";
/// This test binary shares the workspace version with `deslop-mcp`.
const MCP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The whole-report IPC method `duplicates` issues.
const REPORT_GET: &str = "report/get";
/// The one-cluster IPC method `cluster-by-id` issues.
const CLUSTER_BY_ID: &str = "cluster/byId";
/// What the guard names as the engine version of a reply with no stamp.
const UNSTAMPED_ENGINE: &str = "unknown (reply carries no tool_version)";
/// The field this wire expects where the foreign cluster carries `weight`.
const RENAMED_FIELD: &str = "missing field `mass`";
/// The remedy the message must name.
const REMEDY: &str = "reinstall the Deslop VSIX";
/// The message the guard replaced, on every site: `ipc <thing> parse:`.
const OPAQUE_PARSE_PREFIX: &str = " parse:";
/// The prefix of every raw serde field error.
const FIELD_ERROR: &str = "missing field";
/// Clusters requested per page; the error path never reads them.
const PAGE_LIMIT: u64 = 5;

/// An LSP from the release that wrote the foreign report: it serves that
/// report, and its one cluster by id, and knows nothing else.
fn foreign_release_lsp(method: &str, report: &Value) -> Reply {
    if method == REPORT_GET {
        return Reply::Result(report.clone());
    }
    if method == CLUSTER_BY_ID {
        return Reply::Result(report.pointer("/clusters/0").cloned().unwrap_or_default());
    }
    method_not_found(method)
}

/// Binds a stub serving the foreign report under `workspace`.
fn bind_foreign_release_lsp(workspace: &Path) -> Result<()> {
    let report: Value = serde_json::from_str(FOREIGN_RELEASE_REPORT)?;
    bind_stub_lsp(
        workspace,
        Arc::new(move |method| foreign_release_lsp(method, &report)),
    )
}

/// Calls `tool` with `arguments` against the foreign-release stub and
/// returns the refusal message beside the endpoint it must name. The
/// response must be an error frame, never a page: an empty page reads as
/// "no duplicates", the silent failure the guard exists to prevent.
fn refusal_for(tool: &str, arguments: &Value) -> Result<(String, String)> {
    let workspace = TempDir::new()?;
    bind_foreign_release_lsp(workspace.path())?;
    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": tool, "arguments": arguments }),
    )?;
    ensure!(
        response.pointer("/result").is_none(),
        "a mismatched reply must not produce a result page: {response}"
    );
    let (_error, message) = error_and_message(&response)?;
    Ok((message, expected_socket_fragment(workspace.path())?))
}

/// The named condition, element by element.
fn ensure_named_mismatch(
    message: &str,
    socket: &str,
    method: &str,
    engine_version: &str,
) -> Result<()> {
    let mcp = format!("deslop-mcp {MCP_VERSION}");
    let engine = format!("deslop-lsp {engine_version}");
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
        message.contains(method),
        "must name the IPC method: {message}"
    );
    ensure!(
        !message.contains(OPAQUE_PARSE_PREFIX),
        "the opaque message must never reach a caller: {message}"
    );
    Ok(())
}

#[test]
fn duplicates_over_a_report_from_another_release_names_both_versions_and_the_remedy() -> Result<()>
{
    let (message, socket) = refusal_for(
        "duplicates",
        &json!({ "offset": 0, "limit": PAGE_LIMIT, "detail": "summary" }),
    )?;
    ensure_named_mismatch(&message, &socket, REPORT_GET, FOREIGN_RELEASE_VERSION)?;
    ensure!(
        !message.contains(FIELD_ERROR),
        "a stamped foreign report is refused before decoding, so no raw field error: {message}"
    );
    Ok(())
}

#[test]
fn cluster_by_id_over_a_report_from_another_release_names_the_same_condition() -> Result<()> {
    let (message, socket) = refusal_for(
        "cluster-by-id",
        &json!({ "id": FOREIGN_RELEASE_CLUSTER_ID }),
    )?;
    ensure_named_mismatch(&message, &socket, CLUSTER_BY_ID, UNSTAMPED_ENGINE)?;
    ensure!(
        message.contains(RENAMED_FIELD),
        "an unstamped cluster page names the field the wire moved: {message}"
    );
    Ok(())
}
