//! E2E tests for the state-file and IPC surfaces added by the
//! MCP-architecture fix.
//!
//! [LIVE-STATE-FILE] The LSP writes `.deslop-cache/live-report.json`
//! after every analysis pass so the MCP can read it without running its
//! own pipeline.
//!
//! [LSP-IPC] The LSP exposes `.deslop-cache/deslop.sock` (Unix only)
//! so the MCP can delegate `duplicates/findSimilar` and
//! `embedding/listModels` without duplicating compute.

mod common;

use std::{
    fs,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Result};
use common::{call, copy_fixture, handshake, notification, spawn_lsp, take_io, write_frame};

const STATE_FILE: &str = ".deslop-cache/live-report.json";
const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// [LIVE-STATE-FILE] The LSP must write the state file during
/// initialization so the MCP has something to read immediately.
#[test]
fn state_file_exists_after_initialize() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

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
    let count = cluster_count(&report);
    ensure!(
        count > 0,
        "csharp-small fixture must produce at least one cluster in the state file"
    );

    // Verify the live API cluster count matches the state file content.
    let live = call(
        &mut stdin,
        &mut stdout,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
    let live_count = live
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
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
    let workspace = copy_fixture("csharp-small")?;
    let state_path = workspace.path().join(STATE_FILE);
    seed_cached_report(&state_path)?;

    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;
    let start = Instant::now();
    let live = call(
        &mut stdin,
        &mut stdout,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
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
    let workspace = copy_fixture("csharp-small")?;
    let state_path = workspace.path().join(STATE_FILE);
    seed_cached_report(&state_path)?;
    let cached_bytes = fs::read(&state_path)?;

    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;
    let seeded = call(
        &mut stdin,
        &mut stdout,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
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
    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;
    let post_install = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "report/get",
            "params": {}
        }),
    )?;
    let refreshed_count = post_install
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
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
    fs::write(
        &beta,
        b"public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n",
    )?;
    write_frame(&mut stdin, &watched_file_changed(&beta)?)?;

    let updated_count =
        wait_for_cluster_count_change(&socket_path, refreshed_count, ANALYSIS_TIMEOUT)?;
    ensure!(
        updated_count < refreshed_count,
        "incremental update after cache load must reduce cluster count: {refreshed_count} -> {updated_count}"
    );

    let live = call(
        &mut stdin,
        &mut stdout,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
    let live_count = live
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
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
    let workspace = copy_fixture("csharp-small")?;
    let state_path = workspace.path().join(STATE_FILE);
    let parent = state_path
        .parent()
        .ok_or_else(|| anyhow!("state path must have parent: {}", state_path.display()))?;
    fs::create_dir_all(parent)?;
    fs::write(
        &state_path,
        br#"{"tool_version":"stale","files_analysed":999,"clusters":[]}"#,
    )?;
    let bad_bytes = fs::read(&state_path)?;

    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;
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
        cluster_count(&fresh) > 0,
        "fresh scan must analyse the workspace and write real clusters: {fresh}"
    );

    let live = call(
        &mut stdin,
        &mut stdout,
        "deslop/reportGet",
        &serde_json::json!({}),
    )?;
    let live_count = live
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    ensure!(
        live_count == cluster_count(&fresh),
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
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let initial = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "report/get",
            "params": {}
        }),
    )?;
    let initial_count = initial
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    ensure!(
        initial_count > 0,
        "initial live IPC report must have clusters: {initial}"
    );

    fs::write(
        &beta,
        b"public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n",
    )?;
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
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "duplicates/findSimilar",
            "params": {
                "input": {
                    "kind": "snippet",
                    "snippet": "namespace N { class C { void M(int x) { return; } } }",
                    "language": "csharp"
                },
                "max_results": 5
            }
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
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "embedding/listModels",
            "params": {}
        }),
    )?;
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

/// [LSP-IPC], [MCP-IPC-CLIENT] MCP uses `deslop.lsp.refreshReport` over
/// IPC to force a full re-analysis after an agent edit. Under the
/// IPC-truth architecture, freshness is observed via the live
/// `report/get` IPC reply — the seed cache is only rewritten on
/// cold-pass install.
#[cfg(unix)]
#[test]
fn ipc_socket_handles_refresh_report_request() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let beta = workspace.path().join("Beta.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let initial = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "report/get",
            "params": {}
        }),
    )?;
    let initial_count = initial
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    ensure!(
        initial_count > 0,
        "initial live IPC report must have clusters: {initial}"
    );

    fs::write(
        &beta,
        b"public class Beta {\n    public string Name() {\n        return \"unique\";\n    }\n}\n",
    )?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "deslop.lsp.refreshReport",
            "params": {}
        }),
    )?;
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
    ensure!(
        response
            .pointer("/result/clustersRemoved")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|removed| removed >= 1),
        "refreshReport must report removed clusters after the Beta.cs edit: {response}"
    );

    let updated = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "report/get",
            "params": {}
        }),
    )?;
    let updated_count = updated
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    ensure!(
        updated_count < initial_count,
        "refreshReport must rescan edited files and reduce cluster count: {initial_count} -> {updated_count}"
    );
    Ok(())
}

/// [LSP-IPC] Unrecognised IPC methods must return a JSON-RPC method-not-found
/// error rather than silently dropping the request.
#[cfg(unix)]
#[test]
fn ipc_socket_returns_method_not_found_for_unknown_method() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _guard = KillOnDrop(&mut child);

    let _init = handshake(&mut stdin, &mut stdout)?;

    let socket_path = workspace.path().join(".deslop-cache").join("deslop.sock");
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;

    let response = ipc_call(
        &socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "nonexistent/method",
            "params": {}
        }),
    )?;
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

struct KillOnDrop<'a>(&'a mut std::process::Child);

impl Drop for KillOnDrop<'_> {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// Polls until `path` exists or `timeout` elapses.
fn wait_for_file(path: &Path, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if path.exists() {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    Err(anyhow!("timed out waiting for {}", path.display()))
}

/// Polls until `path` contains bytes different from `previous` or `timeout` elapses.
fn wait_for_state_file_change(path: &Path, previous: &[u8], timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if let Ok(bytes) = fs::read(path) {
            if bytes != previous {
                return Ok(());
            }
        }
        if start.elapsed() >= timeout {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    Err(anyhow!("timed out waiting for state file to change"))
}

/// Polls the LSP's IPC socket for `report/get` until the visible cluster
/// count differs from `previous_count` or `timeout` elapses. Used to
/// observe live incremental updates without depending on `live-report.json`
/// mtimes — under the IPC-truth architecture ([LIVE-SEED-CACHE],
/// [MCP-IPC-CLIENT]) the seed cache is only written on cold-pass install.
#[cfg(unix)]
fn wait_for_cluster_count_change(
    socket_path: &Path,
    previous_count: usize,
    timeout: Duration,
) -> Result<usize> {
    let start = Instant::now();
    loop {
        if let Ok(report) = ipc_call(
            socket_path,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "report/get",
                "params": {}
            }),
        ) {
            let count = report
                .pointer("/result/clusters")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len);
            if count != previous_count {
                return Ok(count);
            }
        }
        if start.elapsed() >= timeout {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    Err(anyhow!(
        "timed out waiting for live cluster count to differ from {previous_count}"
    ))
}

/// Sends one JSON-RPC envelope over the Unix socket and returns the response line.
#[cfg(unix)]
fn ipc_call(socket_path: &Path, req: &serde_json::Value) -> Result<serde_json::Value> {
    use std::{io::BufRead, os::unix::net::UnixStream};

    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| anyhow!("failed to connect to IPC socket: {error}"))?;
    let mut payload = serde_json::to_vec(req)?;
    payload.push(b'\n');
    stream
        .write_all(&payload)
        .map_err(|error| anyhow!("IPC write failed: {error}"))?;
    stream
        .flush()
        .map_err(|error| anyhow!("IPC flush failed: {error}"))?;
    let mut line = String::new();
    let _bytes_read = std::io::BufReader::new(&stream)
        .read_line(&mut line)
        .map_err(|error| anyhow!("IPC read failed: {error}"))?;
    serde_json::from_str(line.trim())
        .map_err(|error| anyhow!("IPC response is not valid JSON: {error} — raw: {line}"))
}

fn cluster_count(report: &serde_json::Value) -> usize {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

fn watched_file_changed(path: &Path) -> Result<String> {
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path must be absolute: {}", path.display()))?;
    notification(
        "workspace/didChangeWatchedFiles",
        &serde_json::json!({"changes": [{"uri": uri.as_str(), "type": 2}]}),
    )
}

fn seed_cached_report(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("state path must have parent: {}", path.display()))?;
    fs::create_dir_all(parent)?;
    fs::write(path, serde_json::to_vec(&cached_report())?)?;
    Ok(())
}

fn cached_report() -> serde_json::Value {
    serde_json::json!({
        "tool_version": "test-cache",
        "min_nodes": 4,
        "files_analysed": 73,
        "clusters_hidden": 0,
        "cache_stats": {"hits": 7, "misses": 0},
        "metrics": {
            "analysed_loc": 10,
            "duplicated_loc": 2,
            "duplication_percent": 20.0,
            "clusters_total": 1,
            "duplicated_files": 2,
            "threshold": {"percent": 0.0, "breached": false, "source": "none"}
        },
        "schema_doc": "",
        "action_hints": [],
        "boilerplate_hints": [],
        "embedding_provenance": null,
        "clusters": [{
            "id": "cached-gh73",
            "weight": 9.0,
            "size": 2,
            "canonical_node_count": 6,
            "signals": {"structural": 1.0, "token_jaccard": 1.0, "embedding_cos": 0.0, "fused": 1.0},
            "bucket": "identical",
            "occurrences": [
                {"path": "Alpha.cs", "start_byte": 0, "end_byte": 10, "hidden": false},
                {"path": "Beta.cs", "start_byte": 0, "end_byte": 10, "hidden": false}
            ],
            "occurrences_total": 2,
            "occurrences_truncated": false,
            "summary": "",
            "interpretation": ""
        }]
    })
}
