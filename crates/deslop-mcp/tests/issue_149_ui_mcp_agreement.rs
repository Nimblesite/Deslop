//! Regression test for issue #149: the VSIX top-offenders panel and the
//! MCP `top-offenders` tool must report the same clusters in the same
//! worst-first order, and `cluster-by-id` must accept the truncated
//! 7-hex slug the UI shows in every hover bubble / tree row.
//!
//! [VSIX-CLUSTER-ID-CONSISTENCY] / [Deslop#149].

#![cfg(unix)]

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::Path,
};

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    copied_fixture, initialized_mcp, spawn_lsp_and_initialize, structured_content, wait_for_path,
    ChildKillOnDrop, McpHandle, SOCKET_TIMEOUT,
};

/// Lower bound for the slug shared with `clusterSlug()` in the VSIX
/// (`clients/vscode/src/types/report.ts`). Hard-coded here so a drift
/// in the VSIX is caught by the parity test — the slug is the wire
/// contract between the panel and the agent.
const SLUG_LEN: usize = 7;

/// Issue #149 part 1: the canonical cluster list the VSIX consumes via
/// `deslop/reportGet` (a direct LSP IPC `report/get` here) and the
/// cluster list the MCP `top-offenders` tool returns must agree in
/// worst-first order. The test pulls the canonical IDs from the LSP
/// socket, then asks the MCP for the same top-N with a generous
/// `max_occurrences` budget so every cluster fits, and asserts the
/// prefix-by-prefix match.
#[test]
fn ui_and_mcp_top_offenders_agree_on_worst_first_ids() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;

    let canonical_ids = lsp_report_cluster_ids(&socket)?;
    ensure!(
        !canonical_ids.is_empty(),
        "fixture must produce at least one cluster; got an empty canonical list"
    );

    let mut mcp = initialized_mcp(workspace.path())?;
    let top_offenders_ids = mcp_top_offenders_ids(&mut mcp, canonical_ids.len())?;

    let limit = top_offenders_ids.len().min(canonical_ids.len());
    ensure!(
        limit > 0,
        "MCP top-offenders returned zero clusters but LSP report has {}",
        canonical_ids.len()
    );
    let canonical_prefix = canonical_ids.get(..limit).unwrap_or(&[]);
    let mcp_prefix = top_offenders_ids.get(..limit).unwrap_or(&[]);
    assert_eq!(
        canonical_prefix, mcp_prefix,
        "issue #149: VSIX canonical worst-first ids must equal MCP top-offenders ids; \
         canonical={canonical_ids:?} mcp={top_offenders_ids:?}",
    );
    Ok(())
}

/// Issue #149 part 2: `cluster-by-id` must accept the 7-hex slug the
/// VSIX surfaces in its hover bubbles and tree rows, not just the
/// 16-hex canonical id. An agent that quotes the slug from a panel
/// must be able to fetch the cluster without first expanding the id.
#[test]
fn cluster_by_id_accepts_seven_hex_slug() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;

    let canonical_ids = lsp_report_cluster_ids(&socket)?;
    let full_id = canonical_ids
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("fixture must produce at least one cluster"))?;
    ensure!(
        full_id.len() >= SLUG_LEN,
        "canonical cluster id must be at least {SLUG_LEN} hex chars; got {full_id:?}",
    );
    let slug = full_id[..SLUG_LEN].to_owned();

    let mut mcp = initialized_mcp(workspace.path())?;
    let full = mcp_cluster_by_id(&mut mcp, &full_id)?;
    let by_slug = mcp_cluster_by_id(&mut mcp, &slug)?;
    let full_resolved = full
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("cluster-by-id (full id) response missing id: {full}"))?;
    let slug_resolved = by_slug
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("cluster-by-id (slug) response missing id: {by_slug}"))?;
    assert_eq!(
        full_resolved, full_id,
        "cluster-by-id with full id must round-trip the same canonical id; got {full_resolved}",
    );
    assert_eq!(
        slug_resolved, full_id,
        "cluster-by-id with 7-hex slug {slug:?} must resolve to the same canonical id {full_id:?}; got {slug_resolved}",
    );
    Ok(())
}

/// Reads the LSP's full `report/get` over IPC and extracts the cluster
/// ids in their canonical worst-first order. This is the wire shape the
/// VSIX consumes via `deslop/reportGet` (modulo `truncate_for_wire`,
/// which never reorders or drops clusters — only caps occurrences).
fn lsp_report_cluster_ids(socket: &Path) -> Result<Vec<String>> {
    let result = lsp_ipc_call(socket, "report/get", &json!({}))?;
    let clusters = result
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("LSP report missing clusters array: {result}"))?;
    Ok(clusters
        .iter()
        .filter_map(|cluster| cluster.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect())
}

/// Calls MCP `top-offenders` with a generous occurrence budget so the
/// returned list is never short-truncated by [MCP-OCCURRENCE-BUDGET].
/// `n` matches the canonical list length so the response can never
/// claim a cluster the LSP did not also surface.
fn mcp_top_offenders_ids(mcp: &mut McpHandle, n: usize) -> Result<Vec<String>> {
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "top-offenders",
            "arguments": {
                "n": n.max(1),
                "max_occurrences": 100_000_usize,
            }
        }),
    )?;
    let payload = structured_content(&response, "top-offenders")?;
    let clusters = payload
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("top-offenders payload missing clusters array: {payload}"))?;
    Ok(clusters
        .iter()
        .filter_map(|cluster| cluster.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect())
}

/// Calls MCP `cluster-by-id` and returns the structured payload.
fn mcp_cluster_by_id(mcp: &mut McpHandle, id: &str) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "cluster-by-id",
            "arguments": { "id": id }
        }),
    )?;
    structured_content(&response, "cluster-by-id")
}

/// One-shot line-delimited JSON-RPC call against the LSP IPC socket.
/// Mirrors `deslop-mcp/src/backend/ipc.rs` so the test does not depend
/// on an internal crate (the MCP backend is the production consumer of
/// the same protocol).
fn lsp_ipc_call(socket: &Path, method: &str, params: &Value) -> Result<Value> {
    let mut stream = UnixStream::connect(socket).context("connect to LSP IPC socket")?;
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let mut payload = serde_json::to_vec(&request)?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    stream.flush()?;
    let mut line = String::new();
    let _bytes = BufReader::new(&stream).read_line(&mut line)?;
    let response: Value = serde_json::from_str(line.trim())
        .with_context(|| format!("invalid LSP IPC frame: {line}"))?;
    ensure!(
        response.get("error").is_none(),
        "LSP IPC {method} returned error: {response}"
    );
    response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!("LSP IPC response missing result: {response}"))
}
