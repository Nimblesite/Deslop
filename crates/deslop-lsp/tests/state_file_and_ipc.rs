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

use crate::common;

use std::{
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    process::{ChildStdin, ChildStdout},
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Result};
use common::{
    call, cluster_count, copy_fixture, handshake, spawn_lsp_guarded, spawn_lsp_on_fixture_guarded,
    watched_file_changed, write_frame, LspGuard,
};

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
/// `Beta.cs` write concurrently ([DESLOP-LIVE]); whichever pass runs first
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
    for delta_field in ["clustersAdded", "clustersRemoved", "clustersUpdated"] {
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

/// True when any cluster in the live IPC report has occurrences in both
/// `first` and `second` (matched on path suffix).
fn cluster_spans_files(report: &serde_json::Value, first: &str, second: &str) -> bool {
    let clusters = report
        .pointer("/result/clusters")
        .and_then(serde_json::Value::as_array);
    clusters.into_iter().flatten().any(|cluster| {
        let paths: Vec<&str> = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|occurrence| occurrence.get("path").and_then(serde_json::Value::as_str))
            .collect();
        paths.iter().any(|path| path.ends_with(first))
            && paths.iter().any(|path| path.ends_with(second))
    })
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

/// Returns the LSP IPC socket path under `workspace`'s `.deslop/cache`.
fn socket_path_for(workspace: &Path) -> PathBuf {
    workspace.join(".deslop/cache").join("deslop.sock")
}

/// Spawns the LSP over a fresh copy of `fixture`, drives the
/// `initialize`+`initialized` handshake, and blocks until the IPC socket binds.
/// Returns the owned workspace and process guard (both MUST stay bound — dropping
/// either tears the workspace or the LSP down mid-test), the child's stdin/stdout
/// (kept alive so the LSP's stdio transport stays open), and the bound socket path
/// the test issues `ipc_call`s against. The workspace comes back before any
/// source edit, so a test can also mutate watched files and notify the LSP over
/// the returned stdin.
fn fixture_socket_ready(
    fixture: &str,
) -> Result<(
    tempfile::TempDir,
    LspGuard,
    ChildStdin,
    BufReader<ChildStdout>,
    PathBuf,
)> {
    let (workspace, guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(fixture)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let socket_path = socket_path_for(workspace.path());
    wait_for_file(&socket_path, ANALYSIS_TIMEOUT)?;
    Ok((workspace, guard, stdin, stdout, socket_path))
}

/// Copies `csharp-small`, stages `state` at its state-file path, spawns the
/// guarded LSP, and drives the handshake. Returns the owned workspace and state
/// path (keep the workspace bound), the process guard, and the child's
/// stdin/stdout for follow-up `call`s. Every startup-with-persisted-state test
/// shares this setup; what varies is the staged bytes and which post-handshake
/// reads they assert.
/// Stages `state` as the workspace's cached live report and starts the
/// LSP against it, returning once the handshake completes.
///
/// [LIVE-CACHE-SEED-KEY] made the seed loader refuse a report whose
/// sibling run-identity key is absent or foreign, so a bare
/// hand-written state file no longer models a cache the contract
/// serves — it models the stale-seed defect the key exists to close.
/// The staging therefore prewarms first: one real LSP run against this
/// exact workspace writes the state file *and* its key, that server is
/// shut down, and only the report bytes are replaced with `state`. The
/// key still describes the run identity — root, settings, tool
/// version — which is all it records; a seed is an ordinary earlier
/// generation, and the sentinel cluster it now carries is how the
/// tests observe it being served.
fn seeded_workspace_ready(
    state: &[u8],
) -> Result<(
    tempfile::TempDir,
    PathBuf,
    LspGuard,
    ChildStdin,
    BufReader<ChildStdout>,
)> {
    let workspace = copy_fixture("csharp-small")?;
    let state_path = workspace.path().join(STATE_FILE);
    {
        let (prewarm, mut stdin, mut stdout) = spawn_lsp_guarded(workspace.path())?;
        let _init = handshake(&mut stdin, &mut stdout)?;
        wait_for_file(&workspace.path().join(SEED_KEY_FILE), ANALYSIS_TIMEOUT)?;
        wait_for_file(&state_path, ANALYSIS_TIMEOUT)?;
        drop(prewarm);
    }
    fs::write(&state_path, state)?;
    let (guard, mut stdin, mut stdout) = spawn_lsp_guarded(workspace.path())?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    Ok((workspace, state_path, guard, stdin, stdout))
}

/// Reads the live report over the LSP's stdio transport.
fn stdio_report_get(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
) -> Result<serde_json::Value> {
    call(stdin, stdout, "deslop/reportGet", &serde_json::json!({}))
}

/// Calls `probe` every [`POLL_INTERVAL`] until it yields a value, or gives up
/// once `timeout` has elapsed. Every wait in this suite polls on this clock.
fn poll_until<T>(timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    let start = Instant::now();
    loop {
        if let Some(observed) = probe() {
            return Some(observed);
        }
        if start.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Polls until `path` exists or `timeout` elapses.
fn wait_for_file(path: &Path, timeout: Duration) -> Result<()> {
    poll_until(timeout, || path.exists().then_some(()))
        .ok_or_else(|| anyhow!("timed out waiting for {}", path.display()))
}

/// Polls until `path` contains bytes different from `previous` or `timeout` elapses.
fn wait_for_state_file_change(path: &Path, previous: &[u8], timeout: Duration) -> Result<()> {
    poll_until(timeout, || {
        let bytes = fs::read(path).ok()?;
        (bytes.as_slice() != previous).then_some(())
    })
    .ok_or_else(|| anyhow!("timed out waiting for state file to change"))
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
    poll_until(timeout, || {
        ipc_command(socket_path, 1, "report/get")
            .ok()
            .map(|report| cluster_count(&report))
            .filter(|count| *count != previous_count)
    })
    .ok_or_else(|| {
        anyhow!("timed out waiting for live cluster count to differ from {previous_count}")
    })
}

/// Sends `method` with `params` over the Unix socket and returns the response.
#[cfg(unix)]
fn ipc_request(
    socket_path: &Path,
    id: u64,
    method: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    ipc_call(
        socket_path,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }),
    )
}

/// Sends a parameterless `method` over the Unix socket and returns the response.
#[cfg(unix)]
fn ipc_command(socket_path: &Path, id: u64, method: &str) -> Result<serde_json::Value> {
    ipc_request(socket_path, id, method, &serde_json::json!({}))
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

/// Counts clusters in a persisted state-file report, where the cluster
/// array sits at the document root rather than under a JSON-RPC `result`.
fn state_file_cluster_count(report: &serde_json::Value) -> usize {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

/// The synthetic startup cache the seed tests stage, serialized for staging.
fn cached_report_bytes() -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&cached_report())?)
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
            "signals": {"structural": 1.0, "token_jaccard": 1.0, "embedding_cos": 0.0, "fused": 1.0, "agreement": 1.0, "rename_consistency": 0.0, "literal_fraction": 0.0},
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
