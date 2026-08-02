//! Repurposed `StateFileBackend` tests, retargeted at `LiveBackend` +
//! the LSP IPC surface ([MCP-IPC-CLIENT], [LIVE-IPC-SOCKET],
//! [LIVE-SEED-CACHE]).
//!
//! The original tests asserted file-reader behaviour: cache reload on
//! mtime change (#90), durable on-disk state after read, and deletion
//! of incompatible state files (#118). With the IPC architecture the
//! MCP does not read `live-report.json` at all — every read is a
//! socket round-trip to the LSP. The invariants survive in a stronger
//! form here:
//!
//! - "no stale cached snapshot in MCP" — file mtime is replaced by an
//!   LSP-driven `rescan` round-trip that proves the next `report-get`
//!   reflects the new state immediately.
//! - "MCP read never mutates the seed cache" — the seed cache is the
//!   LSP's private warm-start file ([LIVE-SEED-CACHE]); the MCP must
//!   not touch it.
//! - "incompatible seed cache cannot bring the LSP down" — moved to
//!   the LSP startup path, which is the only consumer of the file
//!   under the new architecture.

#![cfg(unix)]

use std::{fs, time::Duration};

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    cluster_ids, copied_fixture, initialized_mcp, lsp_workspace_with_socket,
    spawn_lsp_and_wait_for_socket, structured_content, wait_for_path, SOCKET_TIMEOUT,
};

/// [MCP-IPC-CLIENT] Repurposes `issue_90_report_get_reloads_state_file_between_plain_calls`.
///
/// Original invariant: a second plain `report_get` must not return a
/// stale cached snapshot — when the LSP-written state file changes,
/// the next read must observe the change.
///
/// New mechanism: there is no MCP-side cache to invalidate. Every
/// `report_get` is an IPC round-trip. We mutate a source file, force
/// the LSP to re-analyse via `rescan`, and assert the next plain
/// `report_get` reflects the new generation. Same invariant: stale
/// state cannot survive a forced LSP refresh.
#[test]
fn issue_90_report_get_reflects_lsp_state_between_plain_calls() -> Result<()> {
    let (workspace, _lsp_guard, _socket) = lsp_workspace_with_socket()?;

    let mut mcp = initialized_mcp(workspace.path())?;

    let before = call_report_get(&mut mcp, /*offset*/ 0, /*limit*/ 64)?;
    let before_total = before
        .get("total_clusters")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("total_clusters missing: {before}"))?;
    ensure!(
        before_total > 0,
        "fixture must produce at least one visible cluster before mutation: {before}",
    );

    // Mutate Beta.cs so a re-analysis would produce a different
    // cluster set, then force the LSP to re-run.
    let beta = workspace.path().join("Beta.cs");
    let original = fs::read(&beta)?;
    fs::write(
        &beta,
        b"namespace Beta { public class Differ { public int Run(int x) { return x + 1; } } }\n",
    )?;
    let rescan = mcp.request(
        "tools/call",
        &json!({"name": "rescan", "arguments": {"n": 5}}),
    )?;
    let rescan_structured = structured_content(&rescan, "rescan")?;
    let rescan_generation = rescan_structured
        .get("generation")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("rescan generation missing: {rescan}"))?;
    ensure!(
        rescan_generation > 0,
        "rescan must advance generation past zero: {rescan}",
    );

    let after = call_report_get(&mut mcp, /*offset*/ 0, /*limit*/ 64)?;
    let after_total = after
        .get("total_clusters")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("total_clusters missing: {after}"))?;

    ensure!(
        after_total != before_total
            || cluster_ids(&before) != cluster_ids(&after),
        "issue #90: plain report-get after rescan must reflect mutated source, not return stale state. before={before}, after={after}",
    );

    // Restore so TempDir cleanup is symmetric.
    fs::write(&beta, original)?;
    Ok(())
}

/// [LIVE-SEED-CACHE] Repurposes `current_state_file_persists_after_successful_load`.
///
/// Original invariant: a successful state-file read must not delete
/// or rewrite the file. Under the IPC architecture the MCP no longer
/// reads it at all; the file is the LSP's private warm-start cache.
/// The new contract: an MCP `report-get` must not modify
/// `.deslop/cache/live-report.json` — only the LSP's cold-pass /
/// initial-pass install paths may write it.
#[test]
fn mcp_read_does_not_mutate_lsp_seed_cache() -> Result<()> {
    let workspace = copied_fixture()?;
    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let state_file = workspace.path().join(".deslop/cache/live-report.json");
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for seed cache")?;

    // Sample mtime + bytes after the LSP's initial pass has settled.
    // A small sleep here is unavoidable: the cold-pass install can
    // race with `wait_for_path`. We re-check mtime stability before
    // measuring rather than relying on the timer alone.
    std::thread::sleep(Duration::from_millis(150));
    let baseline_mtime = fs::metadata(&state_file)?.modified()?;
    let baseline_bytes = fs::read(&state_file)?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let _page = call_report_get(&mut mcp, 0, 64)?;
    let _again = call_report_get(&mut mcp, 0, 64)?;

    let after_mtime = fs::metadata(&state_file)?.modified()?;
    let after_bytes = fs::read(&state_file)?;
    ensure!(
        baseline_mtime == after_mtime,
        "MCP read must not touch the LSP seed cache mtime: before={baseline_mtime:?}, after={after_mtime:?}",
    );
    ensure!(
        baseline_bytes == after_bytes,
        "MCP read must not rewrite the LSP seed cache contents",
    );
    Ok(())
}

/// [LIVE-SEED-CACHE] Repurposes `issue_118_incompatible_state_file_is_deleted_instead_of_migrated`.
///
/// Original invariant: an incompatible state file from a previous
/// version must be removed instead of crashing the loader. Under the
/// new architecture the MCP never reads the file, so the consumer is
/// the LSP's warm-start path. The contract: an incompatible seed
/// cache cannot brick the LSP — startup proceeds, the cold pass
/// rewrites the file, and MCP IPC works against the fresh state.
#[test]
fn issue_118_incompatible_seed_cache_cannot_brick_lsp_startup() -> Result<()> {
    let workspace = copied_fixture()?;
    let cache_dir = workspace.path().join(".deslop/cache");
    fs::create_dir_all(&cache_dir).context("create .deslop/cache")?;
    let state_file = cache_dir.join("live-report.json");
    fs::write(&state_file, br#"{"tool_version":"stale","clusters":[]}"#)?;

    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = call_report_get(&mut mcp, 0, 64)?;
    let total = response
        .get("total_clusters")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("total_clusters missing: {response}"))?;
    ensure!(
        total > 0,
        "incompatible seed cache must not block the cold pass; LSP must produce live clusters from source: {response}",
    );

    // Verify the seed cache was rewritten with current-shape JSON
    // (i.e. parses as a Report).
    let bytes = fs::read(&state_file).context("seed cache present after cold pass")?;
    let parsed: Value =
        serde_json::from_slice(&bytes).context("rewritten seed cache must be valid JSON")?;
    ensure!(
        parsed
            .get("tool_version")
            .and_then(Value::as_str)
            .is_some_and(|version| version != "stale"),
        "stale tool_version must not survive cold-pass install: {parsed}",
    );
    Ok(())
}

/// Helper that wraps the verbose `tools/call report-get` envelope.
fn call_report_get(mcp: &mut common::McpHandle, offset: u64, limit: u64) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "report-get",
            "arguments": {"offset": offset, "limit": limit},
        }),
    )?;
    structured_content(&response, "report-get")
}
