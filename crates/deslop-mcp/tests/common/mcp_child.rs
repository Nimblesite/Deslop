//! The `deslop-mcp` child-process harness shared by the CLI suites.
//!
//! [`McpChild`] owns one live MCP conversation plus its companion LSP
//! and per-test workspace, so test authors work in request/response
//! pairs instead of raw process plumbing. Under [MCP-IPC-CLIENT] every
//! read tool call delegates to the LSP over its IPC socket — there is
//! no on-disk fallback — so the harness auto-spawns the LSP against
//! the same root when [`McpChild::spawn`] is invoked.

use std::{
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

use super::{
    copied_fixture, fixture_root, rpc::StdioRpc, spawn_lsp_and_wait_for_socket, value_get,
    ChildKillOnDrop,
};
use anyhow::{anyhow, Context, Result};
use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use tempfile::TempDir;

use super::mcp_wire::{PATHS_FIELD, ROOT_FLAG, TOTAL_CLUSTERS_POINTER};

/// Timeout for a child process to finish after its stdio closes.
const PROCESS_TIMEOUT_SECS: u64 = 30;
/// Interval the bounded waits poll at.
const POLL_INTERVAL_MILLIS: u64 = 50;
/// `min_nodes` the generated two-file fixture state file uses.
pub const FIXTURE_MIN_NODES: u32 = 15;
/// The mutated file of the two-file clone workspace.
pub const SECOND_FILE_NAME: &str = "Two.cs";

fn mcp_binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_deslop-mcp")
}

/// One live `deslop-mcp` child-process conversation. Holds stdio
/// handles + the buffered line reader so the test author works in
/// request/response pairs instead of raw bytes.
///
/// Under [MCP-IPC-CLIENT] every read tool call delegates to the LSP
/// over its IPC socket — there is no on-disk fallback. The harness
/// auto-spawns a companion `deslop-lsp` child against the same root
/// when [`McpChild::spawn`] is invoked. Both children are torn down
/// on drop. Tests that previously read a pre-committed
/// `live-report.json` fixture now exercise the full LSP→IPC→MCP chain.
pub struct McpChild {
    /// The spawned MCP process.
    child: Child,
    /// The JSON-RPC link over the child's stdio.
    rpc: StdioRpc,
    /// Companion LSP process. Reaped when this handle drops so the
    /// per-test workspace can be reclaimed without leaking sockets.
    lsp: Option<ChildKillOnDrop>,
    /// Owned per-test workspace clone. `Some` when [`McpChild::spawn`]
    /// copied a read-only fixture template; `None` when the caller
    /// passed an already-isolated path.
    workspace: Option<TempDir>,
}

impl Deref for McpChild {
    type Target = StdioRpc;

    fn deref(&self) -> &StdioRpc {
        &self.rpc
    }
}

impl DerefMut for McpChild {
    fn deref_mut(&mut self) -> &mut StdioRpc {
        &mut self.rpc
    }
}

impl McpChild {
    /// Spawns an LSP+MCP pair against `root` ([MCP-IPC-CLIENT]).
    ///
    /// Pass [`fixture_root()`] for read-only templates and the
    /// helper will copy the corpus into a per-test [`TempDir`] before
    /// starting the children — this avoids socket-bind contention
    /// when `cargo test` runs the suite in parallel and prevents the
    /// LSP from polluting the checked-in fixture tree. Pass any
    /// already-writable workspace directly; the helper will use it
    /// in place.
    pub fn spawn(root: &Path, extra_args: &[&str]) -> Result<Self> {
        let (workspace_root, workspace) = if root == fixture_root() {
            let temp = copied_fixture()?;
            (temp.path().to_path_buf(), Some(temp))
        } else {
            (root.to_path_buf(), None)
        };
        let lsp = spawn_lsp_and_wait_for_socket(&workspace_root)?;
        let mut cmd = Command::new(mcp_binary_path());
        let _ = cmd
            .arg(ROOT_FLAG)
            .arg(&workspace_root)
            .args(extra_args)
            .env("RUST_LOG", "info")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().context("spawn deslop-mcp binary")?;
        let rpc = StdioRpc::take(&mut child)?;
        Ok(Self {
            child,
            rpc,
            lsp: Some(lsp),
            workspace,
        })
    }

    /// The MCP process id, observable while the conversation is live.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Whether the MCP process has already exited
    /// ([`Child::try_wait`]).
    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }

    /// Closes the conversation and waits for the MCP to exit, killing
    /// it on timeout, and returns its exit status.
    pub fn finish(self) -> std::process::ExitStatus {
        // Destructure so we can move pieces independently. Dropping the
        // link closes the child's stdin; the companion LSP guard drops
        // at the end of this scope and reaps the LSP child without an
        // explicit `kill_companion_lsp` call ([MCP-IPC-CLIENT]).
        let Self {
            mut child,
            rpc,
            lsp: _lsp,
            workspace: _workspace,
        } = self;
        drop(rpc);
        child
            .wait_timeout(Duration::from_secs(PROCESS_TIMEOUT_SECS))
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                let _ = child.kill();
                child
                    .wait()
                    .unwrap_or_else(|_| std::process::ExitStatus::default())
            })
    }

    /// Returns the writable per-test workspace root the LSP+MCP pair
    /// is bound to. Tests that reference workspace files (e.g.
    /// `find-similar` with a `path` argument) must use this rather
    /// than [`fixture_root`] so the path lands inside the pinned
    /// workspace ([MCP-SAFETY]).
    pub fn workspace_root(&self) -> PathBuf {
        self.workspace.as_ref().map_or_else(
            || fixture_root().to_path_buf(),
            |temp| temp.path().to_path_buf(),
        )
    }

    /// Closes stdin and waits up to `duration` for the MCP to exit,
    /// killing it and erroring when it overstays.
    pub fn close_stdin_and_wait(self, duration: Duration) -> Result<std::process::ExitStatus> {
        let Self {
            mut child,
            rpc,
            lsp: _lsp,
            workspace: _workspace,
        } = self;
        drop(rpc);
        child.wait_timeout(duration)?.ok_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            anyhow!("deslop-mcp did not exit within {duration:?} after stdin closed")
        })
    }
}

/// Bounded [`Child::try_wait`] polling shared by the harness teardowns.
pub trait WaitTimeout {
    /// Waits up to `duration` for the child to exit, returning `None`
    /// when it is still running at the deadline.
    fn wait_timeout(
        &mut self,
        duration: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for Child {
    fn wait_timeout(
        &mut self,
        duration: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        let started = std::time::Instant::now();
        while started.elapsed() < duration {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MILLIS));
        }
        Ok(None)
    }
}

/// Polls `tool` (invoked with `args`) until its reported
/// `/total_clusters` drops below `target`, returning the final
/// structured result. A `filesChanged` reload is observable over IPC the
/// instant the analysis pass commits, but the LSP's watcher-driven
/// scheduler can debounce a second pass for the same edit; under heavy
/// CI load that pass can land in the window between the synchronous
/// refresh and the read. This bounded poll waits for the report to
/// settle to its post-reload state — returning on the first satisfying
/// read (the common case, no added latency) — so the caller's assertion
/// observes steady state instead of a transient. On deadline it returns
/// the last read so a genuine "cluster never removed" regression still
/// fails the caller's assertion.
pub fn poll_total_clusters_below(
    child: &mut McpChild,
    tool: &str,
    args: &Value,
    target: u64,
) -> Result<Value> {
    let started = std::time::Instant::now();
    let mut latest = Value::Null;
    while started.elapsed() < Duration::from_secs(PROCESS_TIMEOUT_SECS) {
        latest = super::mcp_calls::structured_tool_result(&super::mcp_calls::call_tool(
            child, tool, args,
        )?)?;
        if value_get(&latest, TOTAL_CLUSTERS_POINTER)?
            .as_u64()
            .unwrap_or(u64::MAX)
            < target
        {
            return Ok(latest);
        }
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MILLIS));
    }
    Ok(latest)
}

/// Runs the `deslop` CLI against `root` and writes the JSON report to
/// `{root}/.deslop/cache/live-report.json` so `StateFileBackend` can
/// read it without an LSP process.
pub fn generate_state_file(root: &Path, min_nodes: u32) -> Result<()> {
    let cache = root.join(".deslop/cache");
    fs::create_dir_all(&cache)?;
    let out_prefix = cache.join("report-gen");
    let status = Command::new(cargo_bin("deslop"))
        .arg(root)
        .arg("--min-nodes")
        .arg(min_nodes.to_string())
        .arg("--output")
        .arg(&out_prefix)
        .arg("--notext")
        .arg("--nohtml")
        .arg("--log-to-console")
        .status()?;
    anyhow::ensure!(status.success(), "deslop analysis failed with {status}");
    let json_src: PathBuf = out_prefix.with_extension("json");
    let state_dst = cache.join("live-report.json");
    fs::rename(&json_src, &state_dst)?;
    Ok(())
}

/// Drives the MCP `initialize` handshake for an already-spawned child.
pub fn init_session(child: &mut McpChild) -> Result<Value> {
    child.initialize("mcp-e2e-harness")
}

/// Spawns an LSP+MCP pair against the read-only fixture template and
/// drives the MCP `initialize` handshake, discarding the response —
/// the steady-state setup every read-tool test shares.
pub fn spawn_and_init() -> Result<McpChild> {
    let mut child = McpChild::spawn(fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    Ok(child)
}

/// Builds the two-file Alpha/Beta clone workspace, generates its state
/// file at `min_nodes = 15`, then spawns + initializes an MCP child
/// against it. Returns the owned workspace dir (caller MUST keep it
/// bound — dropping it deletes the workspace) and the live child.
pub fn two_file_workspace_with_state() -> Result<(TempDir, McpChild)> {
    let temp = TempDir::new()?;
    std::fs::write(
        temp.path().join("One.cs"),
        include_str!("../fixtures/csharp-mcp/Alpha.cs"),
    )?;
    std::fs::write(
        temp.path().join(SECOND_FILE_NAME),
        include_str!("../fixtures/csharp-mcp/Beta.cs"),
    )?;
    generate_state_file(temp.path(), FIXTURE_MIN_NODES)?;
    let mut child = McpChild::spawn(temp.path(), &[])?;
    let _ = init_session(&mut child)?;
    Ok((temp, child))
}

/// Rewrites `Two.cs` to a unique class so the planted clone disappears,
/// regenerates the state file, then fires the `filesChanged`
/// notification for it over the MCP socket.
pub fn mutate_two_and_notify(child: &mut McpChild, temp: &Path) -> Result<()> {
    std::fs::write(
        temp.join(SECOND_FILE_NAME),
        "namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;
    generate_state_file(temp, FIXTURE_MIN_NODES)?;
    child.notify(
        "notifications/deslop/filesChanged",
        &json!({ (PATHS_FIELD): [temp.join(SECOND_FILE_NAME).to_string_lossy().into_owned()] }),
    )
}

/// Copies the fixture into a writable temp dir, writes a single extra source
/// file (`leaf` ← `contents`) into it, then spawns + initializes an MCP child
/// against that workspace. Returns the owned workspace dir (caller MUST keep it
/// bound — dropping it deletes the workspace) and the live child. Used by the
/// "unique / unknown file yields no clusters" tests, which only differ in the
/// planted file and the tool they then call.
pub fn workspace_with_extra_file(leaf: &str, contents: &str) -> Result<(TempDir, McpChild)> {
    let workspace = copied_fixture()?;
    std::fs::write(workspace.path().join(leaf), contents)?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    Ok((workspace, child))
}
