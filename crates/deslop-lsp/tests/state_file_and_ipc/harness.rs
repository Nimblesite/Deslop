//! Process, socket and state-file plumbing for the state-file and IPC
//! suite ([LIVE-STATE-FILE], [LSP-IPC]): spawning the LSP over a fixture,
//! waiting for the socket, the state file and cluster counts to settle,
//! issuing IPC requests, and the synthetic seed the seed tests stage.

use std::{
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    process::{ChildStdin, ChildStdout},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};

use super::{ANALYSIS_TIMEOUT, POLL_INTERVAL, SEED_KEY_FILE, STATE_FILE};
use crate::common::{
    call, cluster_count, copy_fixture, handshake, spawn_lsp_guarded, spawn_lsp_on_fixture_guarded,
    LspGuard,
};

pub(super) fn cluster_spans_files(report: &serde_json::Value, first: &str, second: &str) -> bool {
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

pub(super) fn socket_path_for(workspace: &Path) -> PathBuf {
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
pub(super) fn fixture_socket_ready(
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
pub(super) fn seeded_workspace_ready(
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
pub(super) fn stdio_report_get(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
) -> Result<serde_json::Value> {
    call(stdin, stdout, "deslop/reportGet", &serde_json::json!({}))
}

/// Calls `probe` every [`POLL_INTERVAL`] until it yields a value, or gives up
/// once `timeout` has elapsed. Every wait in this suite polls on this clock.
pub(super) fn poll_until<T>(timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
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
pub(super) fn wait_for_file(path: &Path, timeout: Duration) -> Result<()> {
    poll_until(timeout, || path.exists().then_some(()))
        .ok_or_else(|| anyhow!("timed out waiting for {}", path.display()))
}

/// Polls until `path` contains bytes different from `previous` or `timeout` elapses.
pub(super) fn wait_for_state_file_change(
    path: &Path,
    previous: &[u8],
    timeout: Duration,
) -> Result<()> {
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
pub(super) fn wait_for_cluster_count_change(
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
pub(super) fn ipc_request(
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
pub(super) fn ipc_command(socket_path: &Path, id: u64, method: &str) -> Result<serde_json::Value> {
    ipc_request(socket_path, id, method, &serde_json::json!({}))
}

/// Sends one JSON-RPC envelope over the Unix socket and returns the response line.
#[cfg(unix)]
pub(super) fn ipc_call(socket_path: &Path, req: &serde_json::Value) -> Result<serde_json::Value> {
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
pub(super) fn state_file_cluster_count(report: &serde_json::Value) -> usize {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

/// The synthetic startup cache the seed tests stage, serialized for staging.
pub(super) fn cached_report_bytes() -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&cached_report())?)
}

pub(super) fn cached_report() -> serde_json::Value {
    // The seed must satisfy the current generated wire contract
    // (`wire_generated::Report`/`ReportCluster`/`ReportOccurrence`) or the
    // loader rejects it and the cold pass silently replaces the staged
    // state. Clusters carry mass only ([MCP-TOOLS]): no signals, weight,
    // bucket, summary, interpretation, or action_hints. Mass is the
    // canonical extent times visible occurrences ([RANK-MASS-SUM]):
    // 6 canonical nodes x 2 visible occurrences = 12.
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
        "boilerplate_hints": [],
        "embedding_provenance": null,
        "clusters": [{
            "id": "cached-gh73",
            "rank": 1,
            "severity": "warning",
            "kind": "identical",
            "mass": 12,
            "canonical_node_count": 6,
            "occurrences": [
                {"path": "Alpha.cs", "start_byte": 0, "end_byte": 10, "start_line": 1, "end_line": 1, "hidden": false},
                {"path": "Beta.cs", "start_byte": 0, "end_byte": 10, "start_line": 1, "end_line": 1, "hidden": false}
            ],
            "occurrences_total": 2,
            "occurrence_count": 2,
            "occurrences_truncated": false
        }],
        "literal_findings": [],
        "literal_findings_total": 0,
        "literal_findings_hidden": 0,
        "literal_findings_capped": false,
        "literal_max_findings": 0
    })
}
