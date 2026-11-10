//! Spawning a real `deslop-lsp` for the MCP suites, driving its LSP
//! handshake, and waiting for the IPC endpoint it publishes
//! ([MCP-IPC-CLIENT], [LIVE-IPC-SOCKET]).

use std::{
    fs,
    io::{self, BufReader},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use tempfile::TempDir;

use super::{copied_fixture, POLL_INTERVAL, SOCKET_TIMEOUT};

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
        .stderr(child_log_file(root, LSP_STDERR_LOG)?)
        .spawn()
        .context("spawn deslop-lsp")?;
    let mut stdin = child.stdin.take().context("lsp stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().context("lsp stdout")?);
    lsp_handshake(&mut stdin, &mut stdout).context("lsp handshake")?;
    child.stdin = Some(stdin);
    // Nothing in this suite reads the notifications the LSP keeps
    // publishing after the handshake; keep draining them so the pipe
    // can never fill and stall the server.
    drain_in_background(stdout);
    Ok(child)
}

/// The directory Deslop owns inside a workspace. Child logs are written
/// beside its cache, where file discovery never looks.
const DESLOP_DIR: &str = ".deslop";

/// Where the LSP's stderr lands for the lifetime of a test workspace.
const LSP_STDERR_LOG: &str = "lsp-stderr.log";

/// Opens `<root>/.deslop/<name>` (appending) as a child's stderr.
///
/// Never `Stdio::piped()` for a stream nothing reads: the LSP logs at
/// `info` on stderr — roughly 9 KiB per analysis pass — and a pipe with
/// no reader blocks its writer once the OS buffer (64 KiB) is full. On
/// Linux, where the file watcher adds a pass for every edit a test makes,
/// the server wedged mid-pass holding the session lock and every IPC
/// request behind it waited forever
/// (`t6_seed_cache_does_not_advance_on_incremental_edits`). A file cannot
/// fill, and the log stays readable for as long as the workspace lives.
fn child_log_file(root: &Path, name: &str) -> Result<Stdio> {
    let dir = root.join(DESLOP_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(name))
        .with_context(|| format!("open {name}"))?;
    Ok(Stdio::from(file))
}

/// Drains `reader` on a background thread until the child closes it, so a
/// stream the suite never reads cannot fill its pipe and block the child.
pub(super) fn drain_in_background<R: io::Read + Send + 'static>(mut reader: R) {
    let _drain = thread::spawn(move || io::copy(&mut reader, &mut io::sink()));
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

/// Owned `Child` that is killed and reaped when dropped.
pub struct ChildKillOnDrop(pub Child);

impl Drop for ChildKillOnDrop {
    fn drop(&mut self) {
        let _kill = self.0.kill();
        let _wait = self.0.wait();
    }
}
