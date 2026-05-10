//! End-to-end test for the LSP+MCP side-by-side architecture
//! ([MCP-WHY-LIVE], [MCP-IPC-CLIENT]).
//!
//! Spawns the real `deslop-lsp` binary, waits for its IPC socket at
//! `.deslop-cache/deslop.sock`, then spawns `deslop-mcp` against the
//! same workspace and calls `find-similar` over the MCP wire. The
//! call traverses MCP → IPC socket → LSP → live analysis → IPC reply
//! → MCP response — the same chain agents will hit in production.
//!
//! Without this test the MCP `find-similar` path is only exercised in
//! the `LspNotRunning` error case, leaving every success branch in
//! `tools/handlers.rs` uncovered.

#![cfg(unix)]

use std::fs;

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    copied_fixture, initialized_mcp, spawn_lsp_and_initialize, structured_content, wait_for_path,
    ChildKillOnDrop, McpHandle, SOCKET_TIMEOUT,
};

/// [MCP-IPC-CLIENT] When the LSP is running, MCP must delegate
/// `find-similar` to the LSP IPC socket and return real cluster data
/// — never `LspNotRunning`. This is the success path that lives
/// behind the IPC chain in production.
#[test]
fn find_similar_via_mcp_delegates_to_running_lsp() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let state_file = workspace.path().join(".deslop-cache/live-report.json");
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for state file")?;

    let mut mcp = initialized_mcp(workspace.path())?;

    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "find-similar",
            "arguments": {
                "snippet": include_str!("fixtures/csharp-mcp/Alpha.cs"),
                "language": "csharp",
                "top_n": 5
            }
        }),
    )?;

    let structured = structured_content(&response, "find-similar")?;
    let clusters = structured
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("clusters must be an array: {response}"))?;
    ensure!(
        !clusters.is_empty(),
        "find-similar must return live LSP clusters: {response}"
    );
    ensure!(
        structured.get("below_min_nodes") == Some(&Value::Bool(false)),
        "fixture snippet must be large enough to fingerprint: {response}"
    );
    Ok(())
}

/// [MCP-IPC-CLIENT] `list-embedding-models` is another compute tool:
/// MCP must delegate it to the live LSP IPC socket and expose model
/// metadata through the normal MCP tool result envelope.
#[test]
fn list_embedding_models_via_mcp_delegates_to_running_lsp() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "list-embedding-models", "arguments": {} }),
    )?;
    let structured = structured_content(&response, "list-embedding-models")?;
    let models = structured
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("models must be an array: {response}"))?;
    let stub = models
        .iter()
        .find(|model| model.get("provider_id") == Some(&json!("stub")))
        .ok_or_else(|| anyhow!("list-embedding-models must include stub provider: {response}"))?;
    ensure!(
        stub.get("model_id") == Some(&json!("blake3-stub")),
        "issue #87: stub row must use generated model_id field: {stub}"
    );
    ensure!(
        stub.get("model_version") == Some(&json!("v1")),
        "issue #87: stub row must carry generated model_version field: {stub}"
    );
    ensure!(
        stub.get("dimensions").and_then(Value::as_u64).is_some(),
        "issue #87: stub row must carry generated dimensions field: {stub}"
    );
    ensure!(
        stub.get("recommended").and_then(Value::as_bool) == Some(false),
        "issue #87: stub row must carry generated recommended field: {stub}"
    );
    ensure!(
        stub.get("reachable").and_then(Value::as_bool) == Some(true),
        "issue #87: stub row must carry generated reachable field: {stub}"
    );
    for legacy_key in [
        "name",
        "bare_id",
        "digest",
        "size_bytes",
        "is_embedding_model",
    ] {
        ensure!(
            stub.get(legacy_key).is_none(),
            "issue #87: generated model row must not expose legacy key {legacy_key}: {stub}"
        );
    }
    Ok(())
}

/// [MCP-IPC-CLIENT] Agent `rescan` must ask the running LSP to execute
/// `deslop.lsp.refreshReport`, then return top offenders from the
/// refreshed state file.
#[test]
fn rescan_via_mcp_triggers_lsp_reanalysis() -> Result<()> {
    // [MCP-IPC-CLIENT] / [LIVE-IPC-SOCKET] rescan triggers a full LSP
    // re-analysis over IPC. The MCP response reflects the new state
    // immediately — without round-tripping through `live-report.json`,
    // which is now an LSP-private warm-start cache only ([LIVE-SEED-CACHE]).
    let workspace = copied_fixture()?;
    let beta = workspace.path().join("Beta.cs");
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;

    let mut mcp = initialized_mcp(workspace.path())?;
    // Flush any pending cold-pass install so the post-mutation rescan
    // does not race a delayed background commit.
    let _flush = mcp.request(
        "tools/call",
        &json!({ "name": "rescan", "arguments": { "n": 1 } }),
    )?;
    let before = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 100 } }),
    )?;
    let before_structured = structured_content(&before, "top-offenders")?;
    let before_count = before_structured
        .get("total_clusters")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stale_cluster_id = beta_cluster_id(&before, &before_structured)?;
    ensure!(
        before_count > 0,
        "fixture must start with at least one cluster: {before}"
    );

    fs::write(
        &beta,
        b"namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;

    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "rescan",
            "arguments": {
                "paths": [beta.to_string_lossy().into_owned()],
                "n": 100
            }
        }),
    )?;
    let after = structured_content(&response, "rescan")?;
    let after_count = after
        .get("total_clusters")
        .and_then(Value::as_u64)
        .unwrap_or(before_count);
    ensure!(
        after_count < before_count,
        "rescan must trigger LSP re-analysis and drop the edited Beta.cs clone: {before_count} -> {after_count}; response {response}"
    );
    assert_rescan_progress(&after, &response)?;
    let clusters = rescan_clusters(&after, &response)?;
    ensure!(
        clusters.len() as u64 == after_count,
        "with n=100, rescan clusters must match total_clusters: {response}"
    );
    assert_stale_cluster_absent(clusters, &stale_cluster_id, &response)?;

    // Cross-check: a follow-up plain `report-get` over IPC sees the
    // same fresh state — proving the read path doesn't leak the
    // pre-edit cluster from any cache.
    let cross = mcp.request(
        "tools/call",
        &json!({ "name": "report-get", "arguments": { "offset": 0, "limit": 100 } }),
    )?;
    let cross_structured = structured_content(&cross, "report-get")?;
    let cross_count = cross_structured
        .get("total_clusters")
        .and_then(Value::as_u64)
        .unwrap_or(before_count);
    ensure!(
        cross_count == after_count,
        "MCP report-get after rescan must match the rescan response: rescan={after_count}, report-get={cross_count}"
    );
    Ok(())
}

#[test]
fn issue_135_rescan_generation_matches_report_get_and_session_config() -> Result<()> {
    let workspace = copied_fixture()?;
    let beta = workspace.path().join("Beta.cs");
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);

    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let state_file = workspace.path().join(".deslop-cache/live-report.json");
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for state file")?;

    let mut mcp = initialized_mcp(workspace.path())?;
    fs::write(
        &beta,
        b"namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "rescan",
            "arguments": { "paths": [beta.to_string_lossy().into_owned()], "n": 1 }
        }),
    )?;
    let after = structured_content(&response, "rescan")?;
    assert_rescan_generation_matches_visible_state(&mut mcp, &after)?;
    Ok(())
}

fn beta_cluster_id(response: &Value, structured: &Value) -> Result<String> {
    structured
        .get("clusters")
        .and_then(Value::as_array)
        .and_then(|clusters| {
            clusters
                .iter()
                .find(|cluster| cluster_touches_beta(cluster))
        })
        .and_then(|cluster| cluster.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("fixture must report a Beta.cs cluster before edit: {response}"))
}

fn assert_rescan_progress(after: &Value, response: &Value) -> Result<()> {
    let summary = after
        .get("summary")
        .ok_or_else(|| anyhow!("rescan must expose refresh progress summary: {response}"))?;
    ensure!(
        summary
            .get("clusters_removed")
            .and_then(Value::as_u64)
            .is_some_and(|removed| removed >= 1),
        "rescan summary must show removed stale clusters: {response}"
    );
    ensure!(
        after.get("generation").and_then(Value::as_u64).is_some(),
        "rescan must expose the refreshed generation: {response}"
    );
    ensure!(
        after.get("n").and_then(Value::as_u64) == Some(100),
        "rescan must echo the requested top-offenders count: {response}"
    );
    Ok(())
}

fn assert_rescan_generation_matches_visible_state(
    mcp: &mut McpHandle,
    after: &Value,
) -> Result<()> {
    let rescan_generation = after
        .get("generation")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("rescan must expose a numeric generation: {after}"))?;
    let report = mcp.request(
        "tools/call",
        &json!({ "name": "report-get", "arguments": { "offset": 0, "limit": 0 } }),
    )?;
    let report_page = structured_content(&report, "report-get")?;
    let session = mcp.request(
        "tools/call",
        &json!({ "name": "session-config", "arguments": {} }),
    )?;
    let session_config = structured_content(&session, "session-config")?;
    ensure!(
        report_page.get("generation").and_then(Value::as_u64) == Some(rescan_generation),
        "issue #135: rescan generation must match report-get generation: rescan {rescan_generation}, report {report_page}"
    );
    ensure!(
        session_config.get("generation").and_then(Value::as_u64) == Some(rescan_generation),
        "issue #135: rescan generation must match session-config generation: rescan {rescan_generation}, session {session_config}"
    );
    Ok(())
}

fn rescan_clusters<'a>(after: &'a Value, response: &Value) -> Result<&'a [Value]> {
    after
        .get("clusters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| anyhow!("rescan clusters must be an array: {response}"))
}

fn assert_stale_cluster_absent(
    clusters: &[Value],
    stale_cluster_id: &str,
    response: &Value,
) -> Result<()> {
    ensure!(
        clusters.iter().all(|cluster| {
            cluster.get("id").and_then(Value::as_str) != Some(stale_cluster_id)
        }),
        "rescan(paths) must not return the stale edited-path cluster id {stale_cluster_id}: {response}"
    );
    Ok(())
}

fn cluster_touches_beta(cluster: &Value) -> bool {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .is_some_and(|occurrences| {
            occurrences.iter().any(|occurrence| {
                occurrence
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with("Beta.cs"))
            })
        })
}
