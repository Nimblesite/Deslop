//! Cross-process freshness invariants for the LSP→MCP IPC architecture
//! ([MCP-IPC-CLIENT], [LIVE-IPC-SOCKET], [LIVE-SEED-CACHE]).
//!
//! These tests pin the showstopper behaviour from issue #137: the MCP
//! must serve whatever the LSP's in-memory `latest_report` says **right
//! now**, with no on-disk staleness window between the two processes.

#![cfg(unix)]

use std::{fs, time::Duration};

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    copied_fixture, initialized_mcp, spawn_lsp_and_initialize, structured_content, wait_for_path,
    ChildKillOnDrop, McpHandle, SOCKET_TIMEOUT,
};

/// [MCP-IPC-CLIENT] T1 — read freshness without on-disk staleness.
///
/// Spawns LSP+MCP, takes a baseline `report-get`, mutates `Beta.cs`,
/// asks the LSP to refresh via `rescan`, and immediately re-reads via
/// `report-get`. Asserts the second read reflects the mutation. The
/// implementation MUST NOT depend on the seed cache file mtime
/// changing — under the new architecture it doesn't, because incremental
/// updates no longer rewrite it ([LIVE-SEED-CACHE]).
#[test]
fn t1_report_get_reflects_lsp_state_immediately_after_rescan() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let mut mcp = initialized_mcp(workspace.path())?;

    let baseline = call_report_get(&mut mcp)?;
    let baseline_ids = cluster_ids(&baseline);
    ensure!(
        !baseline_ids.is_empty(),
        "fixture must produce at least one visible cluster: {baseline}",
    );

    // Mutate Beta.cs so the cluster shape is forced to change.
    let beta = workspace.path().join("Beta.cs");
    fs::write(
        &beta,
        b"namespace Beta { public class Differ { public int Run(int x) { return x + 1; } } }\n",
    )
    .context("mutate Beta.cs")?;
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

    // Immediate follow-up read must see the new state.
    let after = call_report_get(&mut mcp)?;
    let after_ids = cluster_ids(&after);
    ensure!(
        baseline_ids != after_ids,
        "MCP read must reflect the LSP's in-memory state immediately after rescan; baseline={baseline_ids:?}, after={after_ids:?}",
    );
    Ok(())
}

/// [MCP-IPC-CLIENT] T2 — issue #137 staleness regression.
///
/// Reproduces the original Basilisk symptom in miniature: a workspace
/// with all clusters under `benchmarks/fixtures/`, a `.deslop.toml`
/// that hides them, and an MCP `top-offenders` response that must
/// drop them after a `rescan`. Without the IPC architecture, the MCP
/// returned the pre-config snapshot from `live-report.json` and
/// continued to surface the hidden clusters as #1 offenders.
#[test]
fn t2_issue_137_report_hide_visible_via_mcp_after_lsp_reanalysis() -> Result<()> {
    use tempfile::TempDir;

    // Build a workspace where all clones live under
    // `benchmarks/fixtures/`. A repo-relative `report_hide` pattern
    // must drop them once the LSP has loaded the config.
    let workspace = TempDir::new()?;
    let hidden_dir = workspace.path().join("benchmarks").join("fixtures");
    fs::create_dir_all(&hidden_dir)?;
    let alpha_src = include_str!("fixtures/csharp-mcp/Alpha.cs");
    let beta_src = include_str!("fixtures/csharp-mcp/Beta.cs");
    fs::write(hidden_dir.join("Alpha.cs"), alpha_src)?;
    fs::write(hidden_dir.join("Beta.cs"), beta_src)?;
    fs::write(
        workspace.path().join(".deslop.toml"),
        "[defaults]\nreport_hide = [\"benchmarks/fixtures/**\"]\n",
    )?;

    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;

    let mut mcp = initialized_mcp(workspace.path())?;
    // Force a full refresh so the assertion does not race the cold
    // pass; without rescan the test would depend on cold-pass timing.
    let _rescan = mcp.request(
        "tools/call",
        &json!({"name": "rescan", "arguments": {"n": 5}}),
    )?;

    let response = mcp.request(
        "tools/call",
        &json!({"name": "top-offenders", "arguments": {"n": 5}}),
    )?;
    let structured = structured_content(&response, "top-offenders")?;
    let total_clusters = structured
        .get("total_clusters")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("total_clusters missing: {response}"))?;
    ensure!(
        total_clusters == 0,
        "issue #137: MCP top-offenders must honour scan-root-relative report_hide via the LSP IPC read; got {total_clusters} cluster(s) where the LSP has hidden them all: {response}",
    );
    Ok(())
}

/// [LIVE-SEED-CACHE] T6 — no per-keystroke seed-cache writes.
///
/// The seed cache is the LSP's private warm-start file. Per the new
/// architecture, it is written once after the initial pass / cold
/// pass, never on per-keystroke incremental updates. This test makes
/// 5 file edits over the LSP and asserts the cache mtime did not
/// advance after the initial write settles.
#[test]
fn t6_seed_cache_does_not_advance_on_incremental_edits() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    let state_file = workspace.path().join(".deslop-cache/live-report.json");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for seed cache")?;
    // Let the initial-pass write settle before sampling mtime.
    std::thread::sleep(Duration::from_millis(200));
    let baseline_mtime = fs::metadata(&state_file)?.modified()?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let beta = workspace.path().join("Beta.cs");
    let original = fs::read(&beta)?;

    // Make 5 distinct edits, each followed by a rescan so we know the
    // LSP definitely processed it. If the per-pass write was still on,
    // the mtime would advance 5 times.
    for index in 0..5_u32 {
        let body = format!(
            "namespace Beta {{ public class Iter{index} {{ public int Run(int x) {{ return x + {index}; }} }} }}\n",
        );
        fs::write(&beta, body.as_bytes())?;
        let _rescan = mcp.request(
            "tools/call",
            &json!({"name": "rescan", "arguments": {"n": 1}}),
        )?;
    }
    fs::write(&beta, original)?;

    let after_mtime = fs::metadata(&state_file)?.modified()?;
    ensure!(
        baseline_mtime == after_mtime,
        "[LIVE-SEED-CACHE] per-keystroke writes must not advance the seed cache mtime; baseline={baseline_mtime:?}, after={after_mtime:?}",
    );
    Ok(())
}

/// Helper that wraps the verbose `tools/call report-get` envelope.
fn call_report_get(mcp: &mut McpHandle) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "report-get",
            "arguments": {"offset": 0, "limit": 64},
        }),
    )?;
    structured_content(&response, "report-get")
}

/// Returns the cluster ids on a `report-get` page in stable order.
fn cluster_ids(page: &Value) -> Vec<String> {
    page.get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter_map(|cluster| cluster.get("id").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
