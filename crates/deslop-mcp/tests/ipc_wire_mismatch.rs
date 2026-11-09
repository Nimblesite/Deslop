//! [MCP-IPC-WIRE-MISMATCH] A reply from another build is refused by name.
//! The stub LSP serves what another Deslop release serves — a report
//! carrying `weight` where this wire carries `mass`, or a page that fits
//! the current wire — and every tool that reads it answers with the one
//! named condition: both versions, the endpoint, the remedy. Never a raw
//! field error, never another build's page, never an empty page an agent
//! could read as "no duplicates" (gh #523, gh #558).

#![cfg(unix)]

use std::{path::Path, sync::Arc};

use anyhow::{ensure, Result};
use common::{
    error_and_message, expected_socket_fragment, initialized_mcp,
    stub_lsp::{bind_stub_lsp, method_not_found, Reply, Stamp},
};
use deslop_core::report_fixtures::{fixture_cluster, fixture_occurrence};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::common;

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
/// Why a reply envelope with no stamp is refused.
const NO_STAMP: &str = "carries no tool_version stamp";
/// A released build that stamps its envelopes but is not this one.
const OTHER_STAMPED_RELEASE: &str = "0.40.1";
/// The cluster a current-wire stub serves by id.
const CURRENT_WIRE_CLUSTER_ID: &str = "c0a74fd8fbb8e31f";
/// Its occurrences: two copies of one 359-byte function.
const CURRENT_WIRE_PATHS: [&str; 2] = ["a.py", "b.py"];
const CURRENT_WIRE_END_BYTE: usize = 359;
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
        Stamp::Unstamped,
        Arc::new(move |method| foreign_release_lsp(method, &report)),
    )
}

/// A cluster page that fits the current wire exactly.
fn current_wire_cluster() -> Result<Value> {
    let occurrences = CURRENT_WIRE_PATHS
        .iter()
        .map(|path| fixture_occurrence(path, 0, CURRENT_WIRE_END_BYTE))
        .collect();
    Ok(serde_json::to_value(fixture_cluster(
        CURRENT_WIRE_CLUSTER_ID,
        occurrences,
    ))?)
}

/// Binds a stub, stamped as `stamp`, that serves the current-wire cluster by id.
fn bind_current_wire_lsp(workspace: &Path, stamp: Stamp) -> Result<()> {
    let cluster = current_wire_cluster()?;
    bind_stub_lsp(
        workspace,
        stamp,
        Arc::new(move |method| {
            if method == CLUSTER_BY_ID {
                return Reply::Result(cluster.clone());
            }
            method_not_found(method)
        }),
    )
}

/// The `cluster-by-id` response from a stub stamped as `stamp`.
fn cluster_by_id_from(stamp: Stamp) -> Result<(Value, String)> {
    let workspace = TempDir::new()?;
    bind_current_wire_lsp(workspace.path(), stamp)?;
    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "cluster-by-id", "arguments": { "id": CURRENT_WIRE_CLUSTER_ID } }),
    )?;
    Ok((response, expected_socket_fragment(workspace.path())?))
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
        message.contains(NO_STAMP),
        "an envelope with no stamp is refused for that, by name: {message}"
    );
    ensure!(
        !message.contains(FIELD_ERROR),
        "refused before decoding, so no raw field error: {message}"
    );
    Ok(())
}

/// gh #558: a page from another build is refused even when it fits this
/// wire. Released 0.41.0 served a 0.40.1 engine's cluster as if it were
/// its own, because only whole reports carried a version stamp.
#[test]
fn cluster_by_id_refuses_another_builds_page_even_when_it_decodes() -> Result<()> {
    for (stamp, engine) in [
        (Stamp::Release(OTHER_STAMPED_RELEASE), OTHER_STAMPED_RELEASE),
        (Stamp::Unstamped, UNSTAMPED_ENGINE),
    ] {
        let (response, socket) = cluster_by_id_from(stamp)?;
        ensure!(
            response.pointer("/result").is_none(),
            "another build's page must not be served as this build's answer: {response}"
        );
        let (_error, message) = error_and_message(&response)?;
        ensure_named_mismatch(&message, &socket, CLUSTER_BY_ID, engine)?;
    }
    Ok(())
}

/// The control: this build's own stamped page is served whole.
#[test]
fn cluster_by_id_serves_this_builds_page() -> Result<()> {
    let (response, _socket) = cluster_by_id_from(Stamp::ThisBuild)?;
    ensure!(
        response.pointer("/error").is_none(),
        "this build's page is served: {response}"
    );
    let text = response
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let cluster: Value = serde_json::from_str(text)?;
    ensure!(
        cluster.pointer("/id").and_then(Value::as_str) == Some(CURRENT_WIRE_CLUSTER_ID),
        "the served cluster is the one asked for: {cluster}"
    );
    let paths: Vec<&str> = cluster
        .pointer("/occurrences")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("path").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    ensure!(
        paths == CURRENT_WIRE_PATHS,
        "both occurrences, in order: {paths:?}"
    );
    Ok(())
}
