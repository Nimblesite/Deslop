//! E2E tests for the state-file and IPC surfaces added by the
//! MCP-architecture fix.
//!
//! [LIVE-STATE-FILE] The LSP writes `.deslop/cache/live-report.json` on
//! initialize and on each cold-pass install — never on incremental passes
//! ([LIVE-SEED-CACHE]) — so the MCP can warm-start without running its own
//! pipeline.
//!
//! [LSP-IPC] The LSP exposes `.deslop/cache/deslop.sock` (Unix only)
//! so the MCP can delegate `duplicates/findSimilar` and
//! `embedding/listModels` without duplicating compute.
//!
//! Whole suite is Unix-gated: it drives the Unix-socket transport
//! end-to-end. The TCP twin that Windows production uses lives in
//! `crates/deslop-mcp/tests/tcp_transport.rs` ([LIVE-IPC-TCP]) and
//! runs on every platform.

#![cfg(unix)]

use std::{
    fs,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Result};
use common::{
    cluster_count, handshake, spawn_lsp_on_fixture_guarded, watched_file_changed, write_frame,
};
use harness::{
    cached_report_bytes, cluster_spans_files, fixture_socket_ready, ipc_command, ipc_request,
    seeded_workspace_ready, socket_path_for, state_file_cluster_count, stdio_report_get,
    wait_for_cluster_count_change, wait_for_file, wait_for_state_file_change,
};

use crate::common;

#[path = "state_file_and_ipc/harness.rs"]
mod harness;

const STATE_FILE: &str = ".deslop/cache/live-report.json";

/// The run-identity key written beside the state file
/// ([LIVE-CACHE-SEED-KEY]). A seed is served as an answer, so the
/// loader refuses a report whose recorded key is absent or does not
/// describe the run asking for it.
const SEED_KEY_FILE: &str = ".deslop/cache/live-report.key";
const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// The `Beta.cs` body the incremental tests write to break the
/// `csharp-small` fixture's only clone pair with `Alpha.cs`.
const UNIQUE_BETA_SOURCE: &[u8] =
    b"public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n";

/// [LIVE-STATE-FILE] The LSP must write the state file during
/// initialization so the MCP has something to read immediately.
#[test]
fn state_file_exists_after_initialize() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded("csharp-small")?;

    let _init = handshake(&mut stdin, &mut stdout)?;

    let state_path = workspace.path().join(STATE_FILE);
    wait_for_file(&state_path, ANALYSIS_TIMEOUT)?;

    let bytes = fs::read(&state_path)?;
    let report: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| anyhow!("state file is not valid JSON: {error}"))?;
    ensure!(
        report.get("tool_version").is_some(),
        "state file must contain the current report shape: {report}"
    );
    let count = state_file_cluster_count(&report);
    ensure!(
        count > 0,
        "csharp-small fixture must produce at least one cluster in the state file"
    );

    // Verify the live API cluster count matches the state file content.
    let live = stdio_report_get(&mut stdin, &mut stdout)?;
    let live_count = cluster_count(&live);
    ensure!(
        live_count == count,
        "state file cluster count ({count}) must match live reportGet count ({live_count})"
    );
    Ok(())
}

/// [LIVE-CACHE-SEED] GH #73: when a valid state file already exists,
/// the LSP must answer `reportGet` from that cache instead of blocking
/// startup on a cold full pass.
#[test]
fn issue_73_lsp_report_get_uses_prestaged_live_report_cache() -> Result<()> {
    let (_workspace, _state_path, _guard, mut stdin, mut stdout) =
        seeded_workspace_ready(&cached_report_bytes()?)?;
    let start = Instant::now();
    let live = stdio_report_get(&mut stdin, &mut stdout)?;
    let elapsed = start.elapsed();

    ensure!(
        elapsed < Duration::from_millis(500),
        "cached startup reportGet must complete under 500ms, took {elapsed:?}"
    );
    let clusters = live
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("reportGet must return clusters: {live}"))?;
    ensure!(
        clusters.len() == 1,
        "cached report must have one cluster: {live}"
    );
    let first = clusters
        .first()
        .ok_or_else(|| anyhow!("cached report must contain at least one cluster: {live}"))?;
    ensure!(
        first.pointer("/id") == Some(&serde_json::json!("cached-gh73")),
        "reportGet must return the staged cached cluster before a cold pass: {live}"
    );
    ensure!(
        live.pointer("/result/files_analysed") == Some(&serde_json::json!(73)),
        "reportGet must preserve cached report metadata: {live}"
    );
    ensure!(
        live.pointer("/result/cache_stats/hits") == Some(&serde_json::json!(7)),
        "reportGet must preserve cached cache stats: {live}"
    );
    Ok(())
}

/// [LIVE-CACHE-SEED], [LIVE-SEED-CACHE], [MCP-IPC-CLIENT] A valid seed
/// cache is a real startup snapshot, not a dead end: the LSP must serve
/// it immediately, then install the background pipeline and keep
/// applying incremental file updates. Under the IPC-truth architecture,
/// freshness is observed via the live `report/get` over IPC — the seed
/// cache is only rewritten on cold-pass install, never on incremental
/// edits.
#[cfg(unix)]
#[test]
fn current_state_file_loads_and_incremental_updates_continue() -> Result<()> {
    let cached_bytes = cached_report_bytes()?;
    let (workspace, state_path, _guard, mut stdin, mut stdout) =
        seeded_workspace_ready(&cached_bytes)?;
    let seeded = stdio_report_get(&mut stdin, &mut stdout)?;
    ensure!(
        seeded.pointer("/result/clusters/0/id") == Some(&serde_json::json!("cached-gh73")),
        "valid cached state must be served before the background scan lands: {seeded}"
    );
    ensure!(
        seeded.pointer("/result/files_analysed") == Some(&serde_json::json!(73)),
        "valid cached state metadata must be preserved at startup: {seeded}"
    );

    // Cold-pass install rewrites the seed cache exactly once.
    wait_for_state_file_change(&state_path, &cached_bytes, ANALYSIS_TIMEOUT)?;
    let socket_path = socket_path_for(workspace.path());
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;
    let post_install = ipc_command(&socket_path, 1, "report/get")?;
    let refreshed_count = cluster_count(&post_install);
    ensure!(
        refreshed_count > 0,
        "background full scan must produce real clusters in the live IPC report: {post_install}"
    );
    ensure!(
        post_install
            .pointer("/result/clusters")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|clusters| clusters
                .iter()
                .all(|cluster| cluster.get("id") != Some(&serde_json::json!("cached-gh73")))),
        "background full scan must replace the staged cache marker in the live report: {post_install}"
    );

    let beta = workspace.path().join("Beta.cs");
    fs::write(&beta, UNIQUE_BETA_SOURCE)?;
    write_frame(&mut stdin, &watched_file_changed(&beta)?)?;

    let updated_count =
        wait_for_cluster_count_change(&socket_path, refreshed_count, ANALYSIS_TIMEOUT)?;
    ensure!(
        updated_count < refreshed_count,
        "incremental update after cache load must reduce cluster count: {refreshed_count} -> {updated_count}"
    );

    let live = stdio_report_get(&mut stdin, &mut stdout)?;
    let live_count = cluster_count(&live);
    ensure!(
        live_count == updated_count,
        "live reportGet over stdio must match the IPC view of the incremental update: {live}"
    );
    Ok(())
}

/// [LIVE-CACHE-SEED] If the persisted state cannot be loaded as the
/// current report shape, startup must ignore it and write a fresh scan.
#[test]
fn incompatible_state_file_is_wiped_and_startup_scans_from_scratch() -> Result<()> {
    let bad_bytes: &[u8] = br#"{"tool_version":"stale","files_analysed":999,"clusters":[]}"#;
    let (_workspace, state_path, _guard, mut stdin, mut stdout) =
        seeded_workspace_ready(bad_bytes)?;
    wait_for_file(&state_path, ANALYSIS_TIMEOUT)?;

    let fresh_bytes = fs::read(&state_path)?;
    ensure!(
        fresh_bytes != bad_bytes,
        "startup must replace incompatible persisted state"
    );
    let fresh: serde_json::Value = serde_json::from_slice(&fresh_bytes)?;
    ensure!(
        fresh.pointer("/tool_version") != Some(&serde_json::json!("stale")),
        "fresh scan must not preserve the incompatible tool marker: {fresh}"
    );
    ensure!(
        fresh.pointer("/files_analysed") != Some(&serde_json::json!(999)),
        "fresh scan must not preserve incompatible cached metadata: {fresh}"
    );
    ensure!(
        state_file_cluster_count(&fresh) > 0,
        "fresh scan must analyse the workspace and write real clusters: {fresh}"
    );

    let live = stdio_report_get(&mut stdin, &mut stdout)?;
    let live_count = cluster_count(&live);
    ensure!(
        live_count == state_file_cluster_count(&fresh),
        "live reportGet must match the fresh scan written to state: {live}"
    );
    Ok(())
}

/// [LIVE-SEED-CACHE], [MCP-IPC-CLIENT] After a file edit triggers
/// re-analysis, the live IPC report must reflect the change. Under the
/// IPC-truth architecture the seed cache is not rewritten on every
/// incremental edit; freshness lives in memory and is exposed over the
/// `report/get` IPC method.
#[cfg(unix)]
#[test]
fn state_file_updated_after_file_change() -> Result<()> {
    let (workspace, _guard, mut stdin, _stdout, socket_path) =
        fixture_socket_ready("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");

    let initial = ipc_command(&socket_path, 1, "report/get")?;
    let initial_count = cluster_count(&initial);
    ensure!(
        initial_count > 0,
        "initial live IPC report must have clusters: {initial}"
    );

    fs::write(&beta, UNIQUE_BETA_SOURCE)?;
    write_frame(&mut stdin, &watched_file_changed(&beta)?)?;

    let updated_count =
        wait_for_cluster_count_change(&socket_path, initial_count, ANALYSIS_TIMEOUT)?;
    ensure!(
        updated_count < initial_count,
        "removing Beta.cs duplicates must reduce cluster count: \
         {initial_count} → {updated_count}"
    );
    Ok(())
}

/// [LSP-IPC] The LSP must bind the Unix socket and respond to a
/// `duplicates/findSimilar` JSON-RPC request with a valid result.
#[cfg(unix)]
#[test]
fn ipc_socket_handles_find_similar_request() -> Result<()> {
    let (_workspace, _guard, _stdin, _stdout, socket_path) = fixture_socket_ready("csharp-small")?;

    let response = ipc_request(
        &socket_path,
        1,
        "duplicates/findSimilar",
        &serde_json::json!({
            "input": {
                "kind": "snippet",
                "snippet": "namespace N { class C { void M(int x) { return; } } }",
                "language": "csharp"
            },
            "max_results": 5
        }),
    )?;
    ensure!(
        response.get("error").is_none(),
        "findSimilar IPC request must not return a JSON-RPC error: {response}"
    );
    ensure!(
        response.get("result").is_some(),
        "findSimilar IPC response must contain a result field: {response}"
    );
    Ok(())
}

/// [LSP-IPC] The IPC socket must respond to `embedding/listModels`
/// with a JSON array. Under the stub-removal plan, production lists
/// only the models reachable from the registered providers (Ollama).
/// The array is empty when Ollama is unreachable (CI default) and
/// non-empty otherwise. Every entry, when present, must carry the
/// `provider_id`, `model_id`, and `dimensions` keys the picker reads.
#[cfg(unix)]
#[test]
fn ipc_socket_handles_list_models_request() -> Result<()> {
    let (_workspace, _guard, _stdin, _stdout, socket_path) = fixture_socket_ready("csharp-small")?;

    let response = ipc_command(&socket_path, 2, "embedding/listModels")?;
    ensure!(
        response.get("error").is_none(),
        "listModels IPC request must not return a JSON-RPC error: {response}"
    );
    let models = response
        .pointer("/result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("listModels result must be an array: {response}"))?;
    for entry in models {
        ensure!(
            entry.get("provider_id").and_then(serde_json::Value::as_str) == Some("ollama"),
            "every listed model must carry provider_id=ollama (stub removed): {entry}"
        );
        ensure!(
            entry
                .get("model_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| !id.is_empty()),
            "every listed model must carry a non-empty model_id: {entry}"
        );
        ensure!(
            entry.get("dimensions").is_some(),
            "every listed model must expose a dimensions field (may be null until probed): {entry}"
        );
    }
    Ok(())
}

/// [LSP-IPC], [MCP-IPC-CLIENT], [LIVE-RESCAN-FRESHNESS] MCP uses
/// `deslop.lsp.refreshReport` over IPC to force a full re-analysis after
/// an agent edit. The spec's contract is end-state freshness: after the
/// call returns, the live report reflects the on-disk sources and
/// eliminated clusters are gone.
///
/// The assertions are deliberately on the post-refresh state, never on the
/// refresh pass's own delta. The filesystem watcher ingests the external
/// `Beta.cs` write concurrently ([LIVE-SCHEDULER]); whichever pass runs first
/// legitimately carries the removal, so `clustersRemoved` on the refresh
/// response is schedule-dependent — asserting it `>= 1` failed on loaded CI
/// runners whenever the watcher won the race (delta 0/0/0 at generation 3).
/// The end-state pins hold under both schedules, and the watcher half of
/// the contract is pinned by `notifications.rs`.
#[cfg(unix)]
#[test]
fn ipc_socket_handles_refresh_report_request() -> Result<()> {
    let (workspace, _guard, _stdin, _stdout, socket_path) = fixture_socket_ready("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");

    let initial = ipc_command(&socket_path, 1, "report/get")?;
    let initial_count = cluster_count(&initial);
    ensure!(
        initial_count > 0,
        "initial live IPC report must have clusters: {initial}"
    );
    ensure!(
        cluster_spans_files(&initial, "Alpha.cs", "Beta.cs"),
        "the csharp-small fixture must open with a cluster spanning \
         Alpha.cs and Beta.cs: {initial}"
    );

    fs::write(&beta, UNIQUE_BETA_SOURCE)?;

    let response = ipc_command(&socket_path, 3, "deslop.lsp.refreshReport")?;
    ensure!(
        response.get("error").is_none(),
        "refreshReport IPC request must not return a JSON-RPC error: {response}"
    );
    ensure!(
        response.pointer("/result/command") == Some(&serde_json::json!("deslop.lsp.refreshReport")),
        "refreshReport result must echo the LSP command id: {response}"
    );
    ensure!(
        response
            .pointer("/result/generation")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|generation| generation >= 2),
        "refreshReport result must advance or expose a live generation: {response}"
    );
    for delta_field in [
        "clustersAdded",
        "clustersRemoved",
        "clustersUpdated",
        "literalFindingsAdded",
        "literalFindingsRemoved",
        "literalFindingsUpdated",
        "worstMass",
    ] {
        ensure!(
            response
                .pointer(&format!("/result/{delta_field}"))
                .and_then(serde_json::Value::as_u64)
                .is_some(),
            "refreshReport must expose {delta_field} as a count: {response}"
        );
    }
    // The Beta.cs rewrite only breaks the one clone pair; no schedule can
    // honestly create or reshape a cluster from it.
    for untouched_field in ["clustersAdded", "clustersUpdated"] {
        ensure!(
            response.pointer(&format!("/result/{untouched_field}")) == Some(&serde_json::json!(0)),
            "a pure-removal edit must not report {untouched_field}: {response}"
        );
    }

    // [LIVE-RESCAN-FRESHNESS] After refreshReport returns, the report must
    // reflect disk no matter which pass carried the removal: the fixture's
    // only clone pair is broken, so nothing may remain. A refresh that
    // skipped the rescan would still show the stale Alpha/Beta cluster and
    // fail here.
    let updated = ipc_command(&socket_path, 4, "report/get")?;
    let updated_count = cluster_count(&updated);
    ensure!(
        updated_count == 0,
        "breaking the fixture's only clone pair must empty the live report: \
         {initial_count} -> {updated_count}: {updated}"
    );
    Ok(())
}

/// [LSP-IPC] Unrecognised IPC methods must return a JSON-RPC method-not-found
/// error rather than silently dropping the request.
#[cfg(unix)]
#[test]
fn ipc_socket_returns_method_not_found_for_unknown_method() -> Result<()> {
    let (_workspace, _guard, _stdin, _stdout, socket_path) = fixture_socket_ready("csharp-small")?;

    let response = ipc_command(&socket_path, 3, "nonexistent/method")?;
    let code = response
        .pointer("/error/code")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| anyhow!("unknown method must return error.code: {response}"))?;
    ensure!(
        code == -32601,
        "unknown method must return JSON-RPC -32601 method-not-found, got {code}: {response}"
    );
    Ok(())
}

/// [LIVE-CACHE-SEED] A seeded startup is not a dead end: after serving the
/// synthetic cache instantly, the background cold pass must re-analyse the
/// real corpus and commit, replacing the seed with the genuine report.
/// This drives the deferred-refresh path end-to-end (the prior seed tests
/// only assert the fast cached serve, then exit before the cold pass runs).
#[test]
fn issue_73_cold_pass_commits_and_replaces_the_seed_after_seeded_startup() -> Result<()> {
    let (_workspace, _state_path, _guard, mut stdin, mut stdout) =
        seeded_workspace_ready(&cached_report_bytes()?)?;

    // The first read is served straight from the synthetic seed cache.
    let seeded = stdio_report_get(&mut stdin, &mut stdout)?;
    ensure!(
        seeded.pointer("/result/clusters/0/id") == Some(&serde_json::json!("cached-gh73")),
        "seeded startup must serve the cached cluster first: {seeded}"
    );

    // Poll until the background cold pass commits the real report — the
    // synthetic `cached-gh73` cluster is replaced by genuine cluster ids.
    let deadline = Instant::now() + ANALYSIS_TIMEOUT;
    let mut replaced = false;
    while Instant::now() < deadline {
        let live = stdio_report_get(&mut stdin, &mut stdout)?;
        let first_id = live
            .pointer("/result/clusters/0/id")
            .and_then(serde_json::Value::as_str);
        if matches!(first_id, Some(id) if id != "cached-gh73") {
            replaced = true;
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    ensure!(
        replaced,
        "the deferred cold pass must commit the real report, replacing the seed cache"
    );
    Ok(())
}
