//! Shared MCP+LSP integration test helpers.
//!
//! Each integration test file is a separate binary, so cross-file reuse
//! is wired via `mod common;` declarations rather than `pub use`. Keeps
//! the per-test files small and prevents duplicated handshake / spawn
//! plumbing across LSP-backed E2E suites.
//!
//! Platform-neutral on purpose: the TCP transport suite
//! (`tcp_transport.rs`, [LIVE-IPC-TCP]) runs on every platform,
//! including the Windows CI check leg.

#![allow(dead_code)]

/// The per-cluster `language` label contract over `duplicates`,
/// shared by every language whose label regressed.
pub mod language_label;
/// Newline-delimited JSON-RPC over a child's stdio.
pub mod rpc;

use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use assert_cmd::cargo::cargo_bin;
use rpc::StdioRpc;
use serde_json::{json, Value};
use tempfile::TempDir;

/// Maximum wait for the LSP to create its IPC socket / state file.
pub const SOCKET_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Copies the C# MCP fixture into a writable temp dir so the LSP can
/// write to `.deslop/cache/`.
pub fn copied_fixture() -> Result<TempDir> {
    copied_fixture_named("csharp-mcp")
}

/// Copies the named fixture directory into a writable temp dir so the
/// LSP can write to `.deslop/cache/`. Looks in this crate's
/// `tests/fixtures/<name>` first, then falls back to the workspace-shared
/// root under `crates/deslop/tests/fixtures/<name>` so MCP wire tests
/// reuse the refactor fixtures instead of duplicating them.
pub fn copied_fixture_named(name: &str) -> Result<TempDir> {
    let local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let src = if local.exists() {
        local
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../deslop/tests/fixtures")
            .join(name)
    };
    let dst = TempDir::new()?;
    copy_dir(&src, dst.path())?;
    let cache = dst.path().join(".deslop/cache");
    if cache.exists() {
        fs::remove_dir_all(&cache).context("clear cache dir")?;
    }
    Ok(dst)
}

/// Recursively copies `src` into `dst`, creating `dst` as needed.
pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            let _bytes = fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Spawns the LSP binary, drives the LSP `initialize`+`initialized`
/// handshake, and returns the running process.
pub fn spawn_lsp_and_initialize(root: &Path) -> Result<Child> {
    spawn_lsp_with_args(root, &[])
}

/// Spawns the LSP binary with extra startup flags (e.g.
/// `--ipc-transport tcp`, [LIVE-IPC-TCP]), drives the LSP
/// `initialize`+`initialized` handshake, and returns the running
/// process.
pub fn spawn_lsp_with_args(root: &Path, extra_args: &[&str]) -> Result<Child> {
    let bin = cargo_bin("deslop-lsp");
    let mut child = Command::new(bin)
        .arg(root)
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn deslop-lsp")?;
    let mut stdin = child.stdin.take().context("lsp stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().context("lsp stdout")?);
    lsp_handshake(&mut stdin, &mut stdout).context("lsp handshake")?;
    child.stdin = Some(stdin);
    child.stdout = Some(stdout.into_inner());
    Ok(child)
}

fn lsp_handshake(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) -> Result<()> {
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "processId": null, "rootUri": null, "capabilities": {} }
    });
    write_lsp_frame(stdin, &serde_json::to_string(&init)?)?;
    let _response = read_lsp_frame(stdout)?;
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_lsp_frame(stdin, &serde_json::to_string(&initialized)?)
}

fn write_lsp_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    deslop_test_support::write_lsp_frame(stdin, payload)
}

fn read_lsp_frame(reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    deslop_test_support::read_lsp_frame(reader)
}

/// Polls until `path` exists, failing after `timeout`.
pub fn wait_for_path(path: &Path, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    loop {
        if path.exists() {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            return Err(anyhow!(
                "timed out after {:?} waiting for {}",
                timeout,
                path.display()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Spawns an initialized LSP against `root` and blocks until it has
/// published its IPC endpoint, returning a kill-on-drop guard for the LSP
/// process. The caller MUST bind the returned `ChildKillOnDrop` to a live
/// (named) local — dropping it early kills the LSP mid-test.
///
/// The artefact waited on comes from the engine ([LIVE-IPC-SOCKET],
/// [LIVE-IPC-TCP]): Windows has no Unix-domain sockets and publishes a TCP
/// endpoint record instead, so a wait spelled `deslop.sock` there times out
/// against a server that came up seconds earlier.
pub fn spawn_lsp_and_wait_for_socket(root: &Path) -> Result<ChildKillOnDrop> {
    let lsp = spawn_lsp_and_initialize(root)?;
    let guard = ChildKillOnDrop(lsp);
    let endpoint = deslop_core::live::transport::endpoint_path(root);
    wait_for_path(&endpoint, SOCKET_TIMEOUT).context("wait for ipc endpoint")?;
    Ok(guard)
}

/// Spawns an initialized LSP over a fresh copied fixture and blocks until
/// its IPC socket exists, returning the workspace temp dir, a kill-on-drop
/// guard for the LSP process, and the socket path. The caller MUST bind the
/// returned `TempDir` and `ChildKillOnDrop` to live (named) locals — dropping
/// either early deletes the workspace or kills the LSP mid-test.
pub fn lsp_workspace_with_socket() -> Result<(TempDir, ChildKillOnDrop, PathBuf)> {
    let workspace = copied_fixture()?;
    let guard = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let endpoint = deslop_core::live::transport::endpoint_path(workspace.path());
    Ok((workspace, guard, endpoint))
}

/// Spawned MCP child + an id-tracked JSON-RPC request loop.
pub struct McpHandle {
    child: Child,
    rpc: StdioRpc,
}

impl McpHandle {
    /// Spawns the `deslop-mcp` binary against `root` over stdio.
    pub fn spawn(root: &Path) -> Result<Self> {
        let bin = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut command = Command::new(bin);
        let _configured = command.arg("--root").arg(root);
        Self::from_command(command)
    }

    /// Spawns the MCP binary from `working_directory` with `root_argument`.
    ///
    /// The explicit textual root is required by the wrong-root regression:
    /// `.` must resolve against the process working directory before the
    /// server binds its live LSP session ([MCP-ROOT-CANONICAL]).
    pub fn spawn_with_root_argument(working_directory: &Path, root_argument: &str) -> Result<Self> {
        let bin = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut command = Command::new(bin);
        let _configured = command
            .arg("--root")
            .arg(root_argument)
            .current_dir(working_directory);
        Self::from_command(command)
    }

    fn from_command(mut command: Command) -> Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn deslop-mcp")?;
        let rpc = StdioRpc::take(&mut child)?;
        Ok(Self { child, rpc })
    }

    /// Sends a JSON-RPC request and reads the matching response,
    /// skipping any pushed notifications in between.
    pub fn request(&mut self, method: &str, params: &Value) -> Result<Value> {
        self.rpc.request(method, params)
    }
}

impl Drop for McpHandle {
    fn drop(&mut self) {
        let _kill = self.child.kill();
        let _wait = self.child.wait();
    }
}

/// Owned `Child` that is killed and reaped when dropped.
pub struct ChildKillOnDrop(pub Child);

impl Drop for ChildKillOnDrop {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}

/// Workspace-relative path of the live-report state file the LSP publishes.
pub const LIVE_REPORT_STATE_FILE: &str = ".deslop/cache/live-report.json";

/// Waits for the LSP to publish its live-report state file under `workspace`,
/// then returns an initialized MCP handle — the standard preamble shared by
/// every MCP-over-live-LSP test.
pub fn wait_for_state_then_init_mcp(workspace: &Path) -> Result<McpHandle> {
    let state_file = workspace.join(LIVE_REPORT_STATE_FILE);
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for state file")?;
    initialized_mcp(workspace)
}

/// Spawns + initializes an MCP child against `root`.
pub fn initialized_mcp(root: &Path) -> Result<McpHandle> {
    let mut mcp = McpHandle::spawn(root)?;
    initialize_mcp(&mut mcp)?;
    Ok(mcp)
}

/// Performs the MCP `initialize` request for an already-spawned client.
pub fn initialize_mcp(mcp: &mut McpHandle) -> Result<()> {
    let response = mcp.rpc.initialize("phase5-e2e")?;
    ensure!(
        response.get("error").is_none(),
        "MCP initialize failed: {response}"
    );
    Ok(())
}

/// Drives `tools/call` for `tool` with `arguments` and returns its
/// `structuredContent` envelope, erroring if the call failed.
pub fn call_tool(mcp: &mut McpHandle, tool: &str, arguments: &Value) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({ "name": tool, "arguments": arguments }),
    )?;
    structured_content(&response, tool)
}

/// Requests one `duplicates` summary page of `limit` clusters, returning
/// the raw JSON-RPC response so an error frame stays inspectable.
pub fn request_duplicates_summary(mcp: &mut McpHandle, limit: u64) -> Result<Value> {
    mcp.request(
        "tools/call",
        &json!({
            "name": "duplicates",
            "arguments": { "offset": 0, "limit": limit, "detail": "summary" }
        }),
    )
}

/// Extracts the `structuredContent` envelope from a successful tools/call.
pub fn structured_content(response: &Value, tool: &str) -> Result<Value> {
    ensure!(
        response.get("error").is_none(),
        "{tool} must succeed when the LSP is live, got: {response}"
    );
    response
        .pointer("/result/structuredContent")
        .cloned()
        .ok_or_else(|| anyhow!("response missing structuredContent: {response}"))
}

/// Returns the cluster ids on a `duplicates` page in stable order.
pub fn cluster_ids(page: &Value) -> Vec<String> {
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

/// Calls the `rescan` MCP tool and returns the structured payload.
pub fn rescan_call(mcp: &mut McpHandle, paths: &[String]) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "rescan",
            "arguments": {
                "paths": paths,
                "limit": 8,
            }
        }),
    )?;
    structured_content(&response, "rescan")
}

/// Resolves a JSON pointer in `value`, erroring when the path is absent.
pub fn value_get(value: &Value, pointer: &str) -> Result<Value> {
    value
        .pointer(pointer)
        .cloned()
        .ok_or_else(|| anyhow!("pointer {pointer} not found in {value}"))
}

/// Resolves a JSON pointer in `value` and clones the array it points at,
/// erroring when the pointer is absent or does not name an array.
pub fn value_array(value: &Value, pointer: &str) -> Result<Vec<Value>> {
    value_get(value, pointer)?
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("pointer {pointer} is not an array in {value}"))
}

/// Extracts the `/error` frame and its string `message` from a JSON-RPC
/// error response, erroring if either is absent.
pub fn error_and_message(response: &Value) -> Result<(Value, String)> {
    let error = response
        .pointer("/error")
        .ok_or_else(|| anyhow!("expected error frame, got: {response}"))?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("error missing string message: {error}"))?;
    Ok((error.clone(), message.to_owned()))
}

/// Canonicalises `workspace` and returns the absolute
/// `.deslop/cache/deslop.sock` path string the MCP backend connects to.
pub fn expected_socket_fragment(workspace: &Path) -> Result<String> {
    let canonical = std::fs::canonicalize(workspace)?;
    Ok(canonical
        .join(".deslop/cache")
        .join("deslop.sock")
        .display()
        .to_string())
}

/// Read-only path to the bundled `csharp-mcp` MCP fixture template.
pub fn fixture_root() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/csharp-mcp"
    ))
}

/// Shell wrapper that launches `deslop-mcp` as a grandchild and prints its
/// pid to stderr, so the orphan-exit suites can kill the launching parent
/// while keeping the MCP child stdio open.
#[cfg(unix)]
pub const KILLABLE_PARENT_SCRIPT: &str = r#"exec 3<&0; "$1" --root "$2" <&3 2>/dev/null & mcp_pid=$!; printf '%s\n' "$mcp_pid" >&2; wait "$mcp_pid""#;

/// Reads the MCP pid the killable-parent shell prints on its stderr line.
#[cfg(unix)]
pub fn read_mcp_pid(child: &mut Child) -> Result<u32> {
    let stderr = child.stderr.take().context("parent stderr")?;
    let mut stderr = BufReader::new(stderr);
    let mut pid_line = String::new();
    let bytes = stderr.read_line(&mut pid_line)?;
    ensure!(bytes > 0, "parent shell did not report mcp pid");
    pid_line
        .trim()
        .parse::<u32>()
        .context("parse mcp pid from parent shell")
}

/// Polls until `pid` no longer exists, returning `false` on timeout.
#[cfg(unix)]
pub fn wait_for_pid_exit(pid: u32, duration: Duration) -> Result<bool> {
    let started = Instant::now();
    while started.elapsed() < duration {
        if !pid_exists(pid)? {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}

/// Probes whether `pid` is alive via `kill -0`.
#[cfg(unix)]
pub fn pid_exists(pid: u32) -> Result<bool> {
    let status = Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("probe process existence with kill -0")?;
    Ok(status.success())
}

/// Best-effort terminate of an orphaned MCP pid: `SIGTERM`, then `SIGKILL`
/// if it does not exit within a second.
#[cfg(unix)]
pub fn terminate_pid(pid: u32) -> Result<()> {
    if !pid_exists(pid)? {
        return Ok(());
    }
    let _term_status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("terminate orphaned mcp pid")?;
    if wait_for_pid_exit(pid, Duration::from_secs(1))? {
        return Ok(());
    }
    let _kill_status = Command::new("kill")
        .arg("-KILL")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("kill orphaned mcp pid")?;
    let _exited = wait_for_pid_exit(pid, Duration::from_secs(1))?;
    Ok(())
}
