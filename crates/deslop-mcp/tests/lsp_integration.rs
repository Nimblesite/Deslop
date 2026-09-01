//! End-to-end test for the LSP+MCP side-by-side architecture
//! ([MCP-WHY-LIVE], [MCP-IPC-CLIENT]).
//!
//! Spawns the real `deslop-lsp` binary, waits for its IPC socket at
//! `.deslop/cache/deslop.sock`, then spawns `deslop-mcp` against the
//! same workspace and calls `find-similar` over the MCP wire. The
//! call traverses MCP → IPC socket → LSP → live analysis → IPC reply
//! → MCP response — the same chain agents will hit in production.
//!
//! Without this test the MCP `find-similar` path is only exercised in
//! the `LspNotRunning` error case, leaving every success branch in
//! `tools/handlers.rs` uncovered.

#![cfg(unix)]

use std::fs;

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};

use crate::common;
use common::{
    copied_fixture, initialized_mcp, lsp_workspace_with_socket, spawn_lsp_and_wait_for_socket,
    structured_content, wait_for_state_then_init_mcp, McpHandle,
};

const TOOLS_CALL_METHOD: &str = "tools/call";
const NAME_FIELD: &str = "name";
const ARGUMENTS_FIELD: &str = "arguments";
const RESCAN_TOOL: &str = "rescan";
const REPORT_GET_TOOL: &str = "report-get";

/// [MCP-IPC-CLIENT] When the LSP is running, MCP must delegate
/// `find-similar` to the LSP IPC socket and return real cluster data
/// — never `LspNotRunning`. This is the success path that lives
/// behind the IPC chain in production.
#[test]
fn find_similar_via_mcp_delegates_to_running_lsp() -> Result<()> {
    let (workspace, _lsp_guard, _socket) = lsp_workspace_with_socket()?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    let response = mcp.request(
        TOOLS_CALL_METHOD,
        &json!({
            (NAME_FIELD): "find-similar",
            (ARGUMENTS_FIELD): {
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

/// [MCP-IPC-CLIENT] [REMOVE-STUB] `list-embedding-models` is another
/// compute tool: MCP must delegate it to the live LSP IPC socket. With
/// the stub provider removed from production payloads, the live LSP
/// returns whatever Ollama reports — empty when Ollama is unreachable.
/// CI never has Ollama running, so the wire payload must be an empty
/// array (no stub fallback row, no legacy keys).
#[test]
fn list_embedding_models_via_mcp_delegates_to_running_lsp() -> Result<()> {
    let (workspace, _lsp_guard, _socket) = lsp_workspace_with_socket()?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): "list-embedding-models", (ARGUMENTS_FIELD): {} }),
    )?;
    let structured = structured_content(&response, "list-embedding-models")?;
    let models = structured
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("models must be an array: {response}"))?;
    let has_stub = models
        .iter()
        .any(|model| model.get("provider_id") == Some(&json!("stub")));
    ensure!(
        !has_stub,
        "list-embedding-models must never include the stub provider in production: {response}"
    );
    for model in models {
        for legacy_key in [
            NAME_FIELD,
            "bare_id",
            "digest",
            "size_bytes",
            "is_embedding_model",
        ] {
            ensure!(
                model.get(legacy_key).is_none(),
                "issue #87: generated model row must not expose legacy key {legacy_key}: {model}"
            );
        }
    }
    Ok(())
}

/// [MCP-IPC-CLIENT] Issue #286: `set-embedding-model` must travel the
/// same IPC chain as every other compute tool. It used to ignore its
/// arguments and answer `LspNotRunning` unconditionally — on every
/// platform, while the LSP was up and serving `find-similar` from this
/// very socket — so the tool could never succeed and its error blamed a
/// server that was running.
///
/// An unregistered provider id keeps the assertion independent of
/// whether an Ollama daemon happens to be installed: the only component
/// that can name the registered providers is the live LSP's registry,
/// so an error naming the request proves the argument crossed the IPC
/// boundary rather than being discarded by the MCP backend.
#[test]
fn issue_286_set_embedding_model_reaches_the_running_lsp() -> Result<()> {
    let (workspace, _lsp_guard, _socket) = lsp_workspace_with_socket()?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    let response = mcp.request(
        TOOLS_CALL_METHOD,
        &json!({
            (NAME_FIELD): "set-embedding-model",
            (ARGUMENTS_FIELD): {
                "user_initiated": true,
                "provider_id": "definitely-not-a-registered-provider",
                "model_id": "nomic-embed-text"
            }
        }),
    )?;

    let rendered = response.to_string();
    ensure!(
        !rendered.contains("LSP is not running"),
        "issue #286: set-embedding-model reported the LSP as down while the LSP was serving this very socket: {response}"
    );
    ensure!(
        !rendered.contains("method-not-found"),
        "issue #286: the LSP IPC table must route embedding/setModel: {response}"
    );
    ensure!(
        rendered.contains("definitely-not-a-registered-provider"),
        "issue #286: the live provider registry must reject the requested provider by name, proving the argument reached the LSP: {response}"
    );
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
    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;

    let mut mcp = initialized_mcp(workspace.path())?;
    // Flush any pending cold-pass install so the post-mutation rescan
    // does not race a delayed background commit.
    let _flush = mcp.request(
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): RESCAN_TOOL, (ARGUMENTS_FIELD): { "n": 1 } }),
    )?;
    let before = mcp.request(
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): "duplicates", (ARGUMENTS_FIELD): { "offset": 0, "limit": 100, "detail": "summary" } }),
    )?;
    let before_structured = structured_content(&before, "duplicates")?;
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
        TOOLS_CALL_METHOD,
        &json!({
            (NAME_FIELD): RESCAN_TOOL,
            (ARGUMENTS_FIELD): {
                "paths": [beta.to_string_lossy().into_owned()],
                "n": 100
            }
        }),
    )?;
    let after = structured_content(&response, RESCAN_TOOL)?;
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
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): REPORT_GET_TOOL, (ARGUMENTS_FIELD): { "offset": 0, "limit": 100 } }),
    )?;
    let cross_structured = structured_content(&cross, REPORT_GET_TOOL)?;
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
    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;

    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    fs::write(
        &beta,
        b"namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;
    let response = mcp.request(
        TOOLS_CALL_METHOD,
        &json!({
            (NAME_FIELD): RESCAN_TOOL,
            (ARGUMENTS_FIELD): { "paths": [beta.to_string_lossy().into_owned()], "n": 1 }
        }),
    )?;
    let after = structured_content(&response, RESCAN_TOOL)?;
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
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): REPORT_GET_TOOL, (ARGUMENTS_FIELD): { "offset": 0, "limit": 0 } }),
    )?;
    let report_page = structured_content(&report, REPORT_GET_TOOL)?;
    let session = mcp.request(
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): "session-config", (ARGUMENTS_FIELD): {} }),
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
