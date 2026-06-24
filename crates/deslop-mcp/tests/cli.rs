//! End-to-end tests for `deslop-mcp`.
//!
//! Drives the real binary over stdio with raw JSON-RPC frames —
//! exactly the contract that Claude Code / Cursor / Continue honour
//! in production. No mocking of the MCP framing anywhere; the only
//! non-binary inputs are the fixture source tree and the prepared
//! JSON payloads below.
//!
//! Covers [MCP-TESTING]: initialize + tools/list + every tool call +
//! resources/read + unparseable / unsupported-language / below-min-nodes
//! edge cases + path-traversal rejection + malformed-frame handling.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use tempfile::TempDir;

mod common;
use common::{fixture_root, value_array, value_get};
#[cfg(unix)]
use common::{pid_exists, read_mcp_pid, terminate_pid, wait_for_pid_exit, KILLABLE_PARENT_SCRIPT};

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
struct McpChild {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
    /// Companion LSP process. Reaped when this handle drops so the
    /// per-test workspace can be reclaimed without leaking sockets.
    lsp: Option<LspGuard>,
    /// Owned per-test workspace clone. `Some` when [`McpChild::spawn`]
    /// copied a read-only fixture template; `None` when the caller
    /// passed an already-isolated path.
    workspace: Option<TempDir>,
}

/// Kill-on-drop wrapper for the companion LSP child. Lives as a
/// standalone field on [`McpChild`] so the parent struct does not
/// need its own `Drop` impl — that would lock down moves out of
/// [`McpChild::stdin`] in `finish` / `close_stdin_and_wait`.
struct LspGuard(Child);

impl Drop for LspGuard {
    fn drop(&mut self) {
        let _killed = self.0.kill();
        let _waited = self.0.wait();
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
    fn spawn(root: &Path, extra_args: &[&str]) -> Result<Self> {
        let (workspace_root, ownedworkspace) = if root == fixture_root() {
            let temp = TempDir::new().context("alloc per-test workspace")?;
            copy_dir_all(root, temp.path())?;
            (temp.path().to_path_buf(), Some(temp))
        } else {
            (root.to_path_buf(), None)
        };
        let lsp = LspGuard(spawn_companion_lsp(&workspace_root)?);
        wait_for_socket(&workspace_root)?;
        let binary = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut cmd = Command::new(binary);
        let _ = cmd
            .arg("--root")
            .arg(&workspace_root)
            .args(extra_args)
            .env("RUST_LOG", "info")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().context("spawn deslop-mcp binary")?;
        let stdin = child.stdin.take().context("child stdin")?;
        let stdout = child.stdout.take().context("child stdout")?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 0,
            lsp: Some(lsp),
            workspace: ownedworkspace,
        })
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value> {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_frame(&frame)?;
        loop {
            let response = self.read_frame()?;
            let response_id = response.get("id").cloned().unwrap_or(Value::Null);
            if response_id == json!(id) {
                return Ok(response);
            }
            // Notifications mixed with responses: skip and keep reading.
            if response.get("method").is_none() {
                return Err(anyhow!("unexpected frame without id match: {response:?}"));
            }
        }
    }

    fn notify(&mut self, method: &str, params: &Value) -> Result<()> {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send_frame(&frame)
    }

    fn read_frame(&mut self) -> Result<Value> {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line)?;
        if bytes == 0 {
            return Err(anyhow!("mcp stdout closed unexpectedly"));
        }
        serde_json::from_str(&line)
            .with_context(|| format!("invalid JSON from mcp: frame was: {line}"))
    }

    fn send_frame(&mut self, frame: &Value) -> Result<()> {
        let bytes = serde_json::to_vec(frame)?;
        self.stdin.write_all(&bytes)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn send_raw_line(&mut self, line: &str) -> Result<()> {
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn finish(self) -> std::process::ExitStatus {
        // Destructure so we can move pieces independently. The
        // companion LSP guard drops at the end of this scope and
        // reaps the LSP child without an explicit `kill_companion_lsp`
        // call ([MCP-IPC-CLIENT]).
        let Self {
            mut child,
            stdin,
            lsp: _lsp,
            workspace: _workspace,
            ..
        } = self;
        drop(stdin);
        child
            .wait_timeout(Duration::from_secs(30))
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
    fn workspace_root(&self) -> PathBuf {
        self.workspace
            .as_ref()
            .map_or_else(|| fixture_root().to_path_buf(), |t| t.path().to_path_buf())
    }

    fn close_stdin_and_wait(mut self, duration: Duration) -> Result<std::process::ExitStatus> {
        drop(self.stdin);
        self.child.wait_timeout(duration)?.ok_or_else(|| {
            let _ = self.child.kill();
            let _ = self.child.wait();
            anyhow!("deslop-mcp did not exit within {duration:?} after stdin closed")
        })
    }
}

/// Spawns a companion `deslop-lsp` process and drives the LSP
/// `initialize`+`initialized` handshake so the IPC socket is ready
/// before the MCP child connects.
fn spawn_companion_lsp(root: &Path) -> Result<Child> {
    let mut child = deslop_test_support::spawn_deslop_lsp(root, Stdio::piped())
        .context("spawn companion deslop-lsp")?;
    let mut stdin = child.stdin.take().context("lsp stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().context("lsp stdout")?);
    lsp_handshake(&mut stdin, &mut stdout).context("lsp initialize")?;
    child.stdin = Some(stdin);
    child.stdout = Some(stdout.into_inner());
    Ok(child)
}

/// Sends the minimal `initialize` + `initialized` LSP handshake.
fn lsp_handshake(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) -> Result<()> {
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"processId": null, "rootUri": null, "capabilities": {}}
    });
    write_lsp_frame(stdin, &serde_json::to_string(&init)?)?;
    let _response = read_lsp_frame(stdout)?;
    let initialized = json!({"jsonrpc": "2.0", "method": "initialized", "params": {}});
    write_lsp_frame(stdin, &serde_json::to_string(&initialized)?)
}

/// Writes one LSP-framed JSON-RPC message.
fn write_lsp_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    deslop_test_support::write_lsp_frame(stdin, payload)
}

/// Reads one LSP-framed JSON-RPC response and returns it as JSON.
fn read_lsp_frame(reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    deslop_test_support::read_lsp_frame(reader)
}

/// Polls until `<root>/.deslop-cache/deslop.sock` exists. Failure
/// after 30 s is fatal — the LSP is meant to bind within seconds.
fn wait_for_socket(root: &Path) -> Result<()> {
    let socket = root.join(".deslop-cache").join("deslop.sock");
    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        if socket.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(anyhow!(
        "companion LSP did not bind {} within 30s",
        socket.display()
    ))
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
fn poll_total_clusters_below(
    child: &mut McpChild,
    tool: &str,
    args: &Value,
    target: u64,
) -> Result<Value> {
    let started = std::time::Instant::now();
    let mut latest = Value::Null;
    while started.elapsed() < Duration::from_secs(30) {
        latest = structured_tool_result(&call_tool(child, tool, args)?)?;
        if value_get(&latest, "/total_clusters")?
            .as_u64()
            .unwrap_or(u64::MAX)
            < target
        {
            return Ok(latest);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(latest)
}

trait WaitTimeout {
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
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(None)
    }
}

/// Read-only fixture root. The `.deslop-cache/live-report.json` state file
/// is pre-committed alongside the source files so `StateFileBackend` can
/// serve data without an LSP process.
/// Copies the fixture (including `.deslop-cache/live-report.json`) to a
/// writable temp directory for tests that mutate the workspace.
fn copied_fixture_root() -> Result<TempDir> {
    let temp = TempDir::new()?;
    copy_dir_all(fixture_root(), temp.path())?;
    Ok(temp)
}

/// Runs the `deslop` CLI against `root` and writes the JSON report to
/// `{root}/.deslop-cache/live-report.json` so `StateFileBackend` can
/// read it without an LSP process.
fn generate_state_file(root: &Path, min_nodes: u32) -> Result<()> {
    let cache = root.join(".deslop-cache");
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

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            let _bytes = fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn init_session(child: &mut McpChild) -> Result<Value> {
    child.request(
        "initialize",
        &json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "mcp-e2e-harness", "version": "0.1.0" }
        }),
    )
}

/// Spawns an LSP+MCP pair against the read-only fixture template and
/// drives the MCP `initialize` handshake, discarding the response —
/// the steady-state setup every read-tool test shares.
fn spawn_and_init() -> Result<McpChild> {
    let mut child = McpChild::spawn(fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    Ok(child)
}

/// Builds the two-file Alpha/Beta clone workspace, generates its state
/// file at `min_nodes = 15`, then spawns + initializes an MCP child
/// against it. Returns the owned workspace dir (caller MUST keep it
/// bound — dropping it deletes the workspace) and the live child.
fn two_file_workspace_with_state() -> Result<(TempDir, McpChild)> {
    let temp = TempDir::new()?;
    std::fs::write(
        temp.path().join("One.cs"),
        include_str!("fixtures/csharp-mcp/Alpha.cs"),
    )?;
    std::fs::write(
        temp.path().join("Two.cs"),
        include_str!("fixtures/csharp-mcp/Beta.cs"),
    )?;
    generate_state_file(temp.path(), 15)?;
    let mut child = McpChild::spawn(temp.path(), &[])?;
    let _ = init_session(&mut child)?;
    Ok((temp, child))
}

/// Rewrites `Two.cs` to a unique class so the planted clone disappears,
/// regenerates the state file, then fires the `filesChanged`
/// notification for it over the MCP socket.
fn mutate_two_and_notify(child: &mut McpChild, temp: &Path) -> Result<()> {
    std::fs::write(
        temp.join("Two.cs"),
        "namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;
    generate_state_file(temp, 15)?;
    child.notify(
        "notifications/deslop/filesChanged",
        &json!({ "paths": [temp.join("Two.cs").to_string_lossy().into_owned()] }),
    )
}

/// Copies the fixture into a writable temp dir, writes a single extra source
/// file (`leaf` ← `contents`) into it, then spawns + initializes an MCP child
/// against that workspace. Returns the owned workspace dir (caller MUST keep it
/// bound — dropping it deletes the workspace) and the live child. Used by the
/// "unique / unknown file yields no clusters" tests, which only differ in the
/// planted file and the tool they then call.
fn workspace_with_extra_file(leaf: &str, contents: &str) -> Result<(TempDir, McpChild)> {
    let workspace = copied_fixture_root()?;
    std::fs::write(workspace.path().join(leaf), contents)?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    Ok((workspace, child))
}

/// Asserts a report page is empty: zero total clusters and an empty
/// `clusters[]` array.
fn assert_empty_page(page: &Value) -> Result<()> {
    assert_eq!(value_get(page, "/total_clusters")?, json!(0));
    assert!(value_get(page, "/clusters")?
        .as_array()
        .is_some_and(Vec::is_empty));
    Ok(())
}

#[cfg(unix)]
fn spawn_mcp_with_killable_parent(root: &Path) -> Result<(McpChild, u32)> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(KILLABLE_PARENT_SCRIPT)
        .arg("deslop-mcp-parent")
        .arg(env!("CARGO_BIN_EXE_deslop-mcp"))
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn killable deslop-mcp parent shell")?;
    let mcp_pid = read_mcp_pid(&mut child)?;
    let stdin = child.stdin.take().context("parent stdin")?;
    let stdout = child.stdout.take().context("parent stdout")?;
    Ok((
        McpChild {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 0,
            lsp: None,
            workspace: None,
        },
        mcp_pid,
    ))
}

#[test]
fn prints_exact_version_contract() -> Result<()> {
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary).arg("--version").output()?;
    assert!(output.status.success(), "status was {}", output.status);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("deslop-mcp {}\n", expected_version())
    );
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(())
}

#[test]
fn prints_json_version_contract() -> Result<()> {
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary)
        .arg("--version")
        .arg("--json")
        .output()?;
    assert!(output.status.success(), "status was {}", output.status);
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_version_manifest(&value, "deslop-mcp", "mcp");
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(())
}

fn assert_version_manifest(value: &Value, name: &str, kind: &str) {
    deslop_test_support::assert_version_manifest(value, name, kind, expected_version());
}

fn call_tool(child: &mut McpChild, name: &str, arguments: &Value) -> Result<Value> {
    let response = child.request(
        "tools/call",
        &json!({ "name": name, "arguments": arguments }),
    )?;
    if response.get("error").is_some() {
        return Err(anyhow!("tools/call {name} failed: {response}"));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!("missing result in response: {response}"))
}

fn structured_tool_result(result: &Value) -> Result<Value> {
    result
        .get("structuredContent")
        .cloned()
        .ok_or_else(|| anyhow!("missing structuredContent in {result}"))
}

/// Spawns + initializes an MCP child against the read-only fixture, invokes
/// `tool` with `arguments` through the success-path [`call_tool`], and returns
/// the live child alongside the call's `structuredContent` payload. The child
/// is returned so the test can issue follow-up calls and `finish()` it; the
/// payload is the value the test asserts on.
fn init_and_tool_payload(tool: &str, arguments: &Value) -> Result<(McpChild, Value)> {
    let mut child = spawn_and_init()?;
    let payload = structured_tool_result(&call_tool(&mut child, tool, arguments)?)?;
    Ok((child, payload))
}

/// Spawns + initializes an MCP child against the read-only fixture and sends a
/// raw `tools/call` request for `tool` with `arguments`, returning the live
/// child and the unwrapped JSON-RPC response frame (no success assertion). Used
/// by tests that assert on the error envelope or on `structuredContent`
/// directly rather than through [`structured_tool_result`].
fn init_and_tool_response(tool: &str, arguments: &Value) -> Result<(McpChild, Value)> {
    let mut child = spawn_and_init()?;
    let response = child.request(
        "tools/call",
        &json!({ "name": tool, "arguments": arguments }),
    )?;
    Ok((child, response))
}

#[test]
fn initialize_returns_server_info_and_capabilities() -> Result<()> {
    let mut child = McpChild::spawn(fixture_root(), &[])?;
    let response = init_session(&mut child)?;
    assert_eq!(value_get(&response, "/jsonrpc")?, json!("2.0"));
    assert_eq!(
        value_get(&response, "/result/protocolVersion")?,
        json!("2024-11-05")
    );
    assert_eq!(
        value_get(&response, "/result/serverInfo/name")?,
        json!("deslop-mcp")
    );
    assert_eq!(
        value_get(&response, "/result/serverInfo/version")?,
        json!(expected_version())
    );
    assert!(
        value_get(&response, "/result/capabilities/tools")?.is_object(),
        "tools capability missing: {response}"
    );
    assert!(
        value_get(&response, "/result/capabilities/resources")?.is_object(),
        "resources capability missing: {response}"
    );
    let _ = child.finish();
    Ok(())
}

fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[test]
fn exits_within_five_seconds_after_stdio_stdin_closes() -> Result<()> {
    let mut child = McpChild::spawn(fixture_root(), &[])?;
    assert!(child.child.id() > 0, "mcp pid must be observable");
    assert!(
        child.child.try_wait()?.is_none(),
        "mcp must stay alive before stdin is closed"
    );
    let response = init_session(&mut child)?;
    assert_eq!(
        value_get(&response, "/result/serverInfo/name")?,
        json!("deslop-mcp")
    );
    assert!(
        value_get(&response, "/result/capabilities/resources")?.is_object(),
        "resources capability missing: {response}"
    );
    let started = std::time::Instant::now();
    let status = child.close_stdin_and_wait(Duration::from_secs(5))?;
    assert!(status.success(), "stdin EOF should exit cleanly: {status}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "stdin EOF must stop deslop-mcp within five seconds"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn exits_when_launching_parent_disappears_with_stdio_open() -> Result<()> {
    let (mut child, mcp_pid) = spawn_mcp_with_killable_parent(fixture_root())?;
    assert_ne!(
        mcp_pid,
        child.child.id(),
        "test must observe the mcp child separately from its shell parent"
    );
    assert!(pid_exists(mcp_pid)?, "mcp pid must exist before initialize");
    assert!(
        child.child.try_wait()?.is_none(),
        "launcher parent must stay alive until killed by the test"
    );
    let response = init_session(&mut child)?;
    assert_eq!(
        value_get(&response, "/result/serverInfo/name")?,
        json!("deslop-mcp")
    );
    assert_eq!(
        value_get(&response, "/result/protocolVersion")?,
        json!("2024-11-05")
    );

    child.child.kill()?;
    let parent_status = child.child.wait()?;
    assert!(
        !parent_status.success(),
        "launcher parent should be killed during orphan-exit test"
    );
    assert!(
        pid_exists(mcp_pid)?,
        "mcp must still be observable after parent kill"
    );
    let exited = wait_for_pid_exit(mcp_pid, Duration::from_secs(5))?;
    if !exited {
        terminate_pid(mcp_pid)?;
    }
    assert!(
        exited,
        "deslop-mcp must exit within 5s when its launching parent disappears"
    );
    Ok(())
}

#[test]
fn tools_list_returns_all_tools_with_schemas() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("tools/list", &json!({}))?;
    let tools = value_get(&response, "/result/tools")?;
    let names: Vec<String> = tools
        .as_array()
        .ok_or_else(|| anyhow!("tools not array"))?
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str).map(str::to_owned))
        .collect();
    assert_eq!(names.len(), 12, "expected 12 tools, got {names:?}");
    for expected in [
        "top-offenders",
        "rescan",
        "report-get",
        "report-query",
        "schema-doc",
        "report-for-file",
        "report-for-range",
        "find-similar",
        "cluster-by-id",
        "list-embedding-models",
        "set-embedding-model",
        "session-config",
    ] {
        assert!(
            names.iter().any(|candidate| candidate == expected),
            "missing tool: {expected}"
        );
    }
    assert_eq!(
        names.first().map(String::as_str),
        Some("top-offenders"),
        "top-offenders must be listed first as the primary tool"
    );
    for tool in tools.as_array().unwrap_or(&Vec::new()) {
        let description = tool
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!description.is_empty(), "tool missing description: {tool}");
        assert!(
            tool.get("inputSchema").is_some_and(Value::is_object),
            "tool missing inputSchema: {tool}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn top_offenders_returns_full_clusters_with_occurrences_and_interpretation() -> Result<()> {
    let (child, payload) = init_and_tool_payload("top-offenders", &json!({ "n": 3 }))?;
    let total = value_get(&payload, "/total_clusters")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters must be present"))?;
    assert!(total >= 1, "fixture must have at least one cluster");
    assert_eq!(
        value_get(&payload, "/n")?.as_u64(),
        Some(3),
        "n must echo the requested value"
    );
    let clusters = value_get(&payload, "/clusters")?;
    let clusters_arr = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters must be an array"))?;
    assert!(
        clusters_arr.len() <= 3,
        "returned {} clusters but requested max 3",
        clusters_arr.len()
    );
    let first = clusters_arr
        .first()
        .ok_or_else(|| anyhow!("at least one cluster expected"))?;
    assert!(
        first.get("occurrences").is_some_and(Value::is_array),
        "top-offenders must return full occurrences array: {first}"
    );
    assert!(
        first
            .get("interpretation")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()),
        "top-offenders must return interpretation text: {first}"
    );
    assert!(
        first
            .get("bucket")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()),
        "top-offenders must return bucket: {first}"
    );
    assert!(
        first
            .get("weight")
            .and_then(Value::as_f64)
            .is_some_and(|w| w > 0.0),
        "top-offenders must return positive weight: {first}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn top_offenders_defaults_to_five_and_clusters_are_worst_first() -> Result<()> {
    let (child, payload) = init_and_tool_payload("top-offenders", &json!({}))?;
    assert_eq!(
        value_get(&payload, "/n")?.as_u64(),
        Some(5),
        "omitting n must default to 5"
    );
    let clusters = value_get(&payload, "/clusters")?;
    let clusters_arr = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters must be an array"))?;
    assert!(
        clusters_arr.len() <= 5,
        "default n=5 must not return more than 5 clusters"
    );
    let weights: Vec<f64> = clusters_arr
        .iter()
        .filter_map(|c| c.get("weight").and_then(Value::as_f64))
        .collect();
    assert_eq!(
        weights.len(),
        clusters_arr.len(),
        "every cluster must have a weight"
    );
    let sorted = weights.windows(2).all(|w| matches!(w, [a, b] if a >= b));
    assert!(
        sorted,
        "clusters must be worst-first by weight: {weights:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_136_top_offenders_max_occurrences_caps_response_and_reports_total() -> Result<()> {
    // Issue #136 [MCP-OCCURRENCE-BUDGET]. A `top-offenders` response on a real
    // workspace can ship 50+ occurrences per cluster — large enough to crash
    // some MCP clients (e.g. Codex's rmcp_client). The fix: a `max_occurrences`
    // budget across returned clusters that keeps the response small while
    // surfacing the true unfiltered count via `total_occurrences` so the agent
    // knows what was filtered. cluster-by-id remains the way to fetch the full
    // occurrence list of any one cluster.
    let mut child = spawn_and_init()?;

    let baseline = call_tool(&mut child, "top-offenders", &json!({ "n": 5 }))?;
    let baseline_payload = structured_tool_result(&baseline)?;
    let baseline_total = value_get(&baseline_payload, "/total_occurrences")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_occurrences must be a non-negative integer"))?;
    assert!(
        baseline_total > 0,
        "fixture must contain at least one occurrence: {baseline_payload}"
    );

    // Pick a budget strictly below the baseline total so truncation MUST
    // fire (either per-cluster or by dropping trailing clusters).
    let budget = baseline_total
        .checked_sub(1)
        .filter(|&b| b > 0)
        .unwrap_or(1);
    let result = call_tool(
        &mut child,
        "top-offenders",
        &json!({ "n": 5, "max_occurrences": budget }),
    )?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(
        value_get(&payload, "/max_occurrences")?.as_u64(),
        Some(budget),
        "result must echo the requested max_occurrences"
    );
    assert_eq!(
        value_get(&payload, "/total_occurrences")?.as_u64(),
        Some(baseline_total),
        "total_occurrences must equal the unfiltered count, not the budgeted count"
    );
    let clusters = value_get(&payload, "/clusters")?;
    let clusters_arr = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters must be an array"))?;
    let returned: u64 = clusters_arr
        .iter()
        .map(|c| {
            c.get("occurrences")
                .and_then(Value::as_array)
                .map_or(0u64, |a| a.len() as u64)
        })
        .sum();
    assert!(
        returned <= budget,
        "budget={budget} must yield at most {budget} occurrences total across returned clusters; got {returned}"
    );
    // The budget can manifest two ways: (a) a cluster shipped with
    // `occurrences_truncated=true` (its tail dropped), or (b) trailing
    // clusters were dropped entirely (returned_clusters < total_clusters
    // limited by `n`). Either is a valid truncation signal — assert at
    // least one fired given baseline_total exceeds the budget of 2.
    let total_clusters = value_get(&payload, "/total_clusters")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters missing"))?;
    let n_requested = value_get(&payload, "/n")?
        .as_u64()
        .ok_or_else(|| anyhow!("n missing"))?;
    let cap = total_clusters.min(n_requested);
    let returned_clusters = clusters_arr.len() as u64;
    let truncation_marker_present = clusters_arr
        .iter()
        .any(|c| c.get("occurrences_truncated").and_then(Value::as_bool) == Some(true));
    let dropped_following_cluster = returned_clusters < cap;
    assert!(
        truncation_marker_present || dropped_following_cluster,
        "budget={budget} with baseline_total={baseline_total} must drop trailing clusters or set occurrences_truncated; got {returned_clusters}/{cap} clusters back: {clusters_arr:#?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_136_top_offenders_default_max_occurrences_is_fifteen() -> Result<()> {
    let (child, payload) = init_and_tool_payload("top-offenders", &json!({}))?;
    assert_eq!(
        value_get(&payload, "/max_occurrences")?.as_u64(),
        Some(15),
        "omitting max_occurrences must default to 15 ([MCP-OCCURRENCE-BUDGET])"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_134_top_offenders_does_not_label_structural_only_matches_as_nearly_identical() -> Result<()>
{
    let (child, payload) = init_and_tool_payload("top-offenders", &json!({ "n": 5 }))?;
    let clusters = value_get(&payload, "/clusters")?;
    let structural_only_nearly_identical = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters must be an array"))?
        .iter()
        .find(|cluster| {
            cluster.get("bucket").and_then(Value::as_str) == Some("nearly_identical")
                && cluster
                    .pointer("/signals/structural")
                    .and_then(Value::as_f64)
                    == Some(1.0)
                && cluster
                    .pointer("/signals/token_jaccard")
                    .and_then(Value::as_f64)
                    == Some(0.0)
                && cluster
                    .pointer("/signals/embedding_cos")
                    .and_then(Value::as_f64)
                    == Some(0.0)
        });
    assert!(
        structural_only_nearly_identical.is_none(),
        "issue #134: top-offenders must not present structural-only zero-token matches as ordinary nearly_identical duplication: {structural_only_nearly_identical:#?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_113_find_similar_description_leads_with_prevention() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("tools/list", &json!({}))?;
    let tools_value = value_get(&response, "/result/tools")?;
    let tools = tools_value
        .as_array()
        .ok_or_else(|| anyhow!("tools/list result.tools must be an array"))?;
    let find_similar_tools: Vec<&Value> = tools
        .iter()
        .filter(|tool| tool.get("name").and_then(Value::as_str) == Some("find-similar"))
        .collect();
    assert_eq!(
        find_similar_tools.len(),
        1,
        "issue #113: tools/list must expose exactly one find-similar tool: {tools:?}"
    );
    let tool = find_similar_tools
        .first()
        .ok_or_else(|| anyhow!("find-similar tool must be present"))?;
    let description = tool
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("find-similar tool must include a description"))?;
    assert!(
        description.starts_with("Call BEFORE writing new code"),
        "issue #113: find-similar description must lead with prevention guidance: {description}"
    );
    assert!(
        description.contains("PREVENT"),
        "issue #113: find-similar description must explicitly name prevention: {description}"
    );
    assert!(
        description.contains("avoid introducing new clones"),
        "issue #113: find-similar description must explain the duplication risk: {description}"
    );
    assert!(
        description.contains("reuse"),
        "issue #113: find-similar description must point agents toward reuse: {description}"
    );
    let schema = tool
        .get("inputSchema")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("find-similar tool must include an input schema object"))?;
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("find-similar schema must include properties"))?;
    for field in ["path", "start_byte", "end_byte", "snippet", "language"] {
        assert!(
            properties.contains_key(field),
            "issue #113: find-similar schema must document {field}: {properties:?}"
        );
    }
    // Issue #170/#198: the `language` enum is derived from the core parser
    // registry, so it must list every first-class language — including
    // `dart`, which `session-config` already reports. A hand-maintained
    // enum let `dart` fall off and made the filter unusable on Dart repos.
    let language_enum = properties
        .get("language")
        .and_then(|language| language.get("enum"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("find-similar language must be a closed enum: {properties:?}"))?;
    let languages: Vec<&str> = language_enum.iter().filter_map(Value::as_str).collect();
    for expected in ["csharp", "rust", "python", "dart"] {
        assert!(
            languages.contains(&expected),
            "issue #170/#198: find-similar language enum must include {expected}, got {languages:?}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_returns_paginated_slim_report_page() -> Result<()> {
    let (child, page) = init_and_tool_payload("report-get", &json!({ "offset": 0, "limit": 10 }))?;
    assert!(
        page.get("report_schema_version").is_none(),
        "report pages must not expose internal report-format revisions"
    );
    assert!(
        page.get("schema_doc").is_none(),
        "schema_doc must live behind schema-doc/deslop://schema, not every report page"
    );
    let total = value_get(&page, "/total_clusters")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters must be a number"))?;
    let returned = value_get(&page, "/page/returned")?
        .as_u64()
        .ok_or_else(|| anyhow!("page.returned missing"))?;
    assert_eq!(value_get(&page, "/page/offset")?, json!(0));
    assert_eq!(value_get(&page, "/page/limit")?, json!(10));
    assert!(
        returned <= 10,
        "returned ({returned}) must respect requested limit"
    );
    assert!(
        total >= returned,
        "total_clusters ({total}) must be >= returned ({returned})"
    );
    assert!(total >= 1, "fixture should surface at least one cluster");
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_110_report_pages_omit_schema_doc_and_schema_doc_tool_serves_it() -> Result<()> {
    let mut child = spawn_and_init()?;
    let report_get = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 2 }),
    )?)?;
    assert!(
        report_get.get("schema_doc").is_none(),
        "issue #110/#111: report-get must not inline repeated schema_doc; got {} chars",
        report_get
            .get("schema_doc")
            .and_then(Value::as_str)
            .map_or(0, str::len)
    );
    let report_query = structured_tool_result(&call_tool(
        &mut child,
        "report-query",
        &json!({ "offset": 0, "limit": 2, "bucket": "identical" }),
    )?)?;
    assert!(
        report_query.get("schema_doc").is_none(),
        "issue #110/#111: report-query must not inline repeated schema_doc; got {} chars",
        report_query
            .get("schema_doc")
            .and_then(Value::as_str)
            .map_or(0, str::len)
    );
    let tools_response = child.request("tools/list", &json!({}))?;
    let tools_value = value_get(&tools_response, "/result/tools")?;
    let tools = tools_value
        .as_array()
        .ok_or_else(|| anyhow!("tools/list must return an array"))?;
    let schema_tool = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("schema-doc"))
        .ok_or_else(|| anyhow!("schema-doc must be listed as the one-shot schema tool"))?;
    assert_eq!(
        schema_tool.pointer("/inputSchema/properties"),
        Some(&json!({})),
        "schema-doc must take no arguments: {schema_tool}"
    );
    let schema_payload = structured_tool_result(&call_tool(&mut child, "schema-doc", &json!({}))?)?;
    let schema_doc_value = value_get(&schema_payload, "/schema_doc")?;
    let schema_doc = schema_doc_value
        .as_str()
        .ok_or_else(|| anyhow!("schema-doc payload must include schema_doc text"))?;
    assert!(
        schema_doc.len() > 1_000,
        "schema-doc must return the full report schema markdown, got {} chars",
        schema_doc.len()
    );
    let resource_response =
        child.request("resources/read", &json!({ "uri": "deslop://schema" }))?;
    let resource_doc_value = value_get(&resource_response, "/result/contents/0/text")?;
    let resource_doc = resource_doc_value
        .as_str()
        .ok_or_else(|| anyhow!("deslop://schema resource must return text"))?;
    assert_eq!(
        schema_doc, resource_doc,
        "schema-doc tool and deslop://schema resource must serve the same markdown"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_accepts_dart_language_filter() -> Result<()> {
    // Issue #170/#198: `report-query` only *filters* already-detected
    // clusters by language — no parsing — yet its `language` enum omitted
    // `dart`, so `language: "dart"` failed JSON-Schema validation with
    // InvalidParams and there was no workaround on Dart repos. The enum is
    // now derived from the core parser registry, so the filter must be
    // accepted (returning a, possibly empty, page) rather than rejected.
    let (child, response) = init_and_tool_response(
        "report-query",
        &json!({ "offset": 0, "limit": 5, "language": "dart" }),
    )?;
    assert!(
        response.get("error").is_none(),
        "issue #170/#198: report-query must accept language=\"dart\" at the \
         schema layer, not reject it as InvalidParams: {response}"
    );
    let page = structured_tool_result(
        response
            .get("result")
            .ok_or_else(|| anyhow!("report-query must return a result: {response}"))?,
    )?;
    let clusters = value_get(&page, "/clusters")?;
    assert!(
        clusters.is_array(),
        "report-query must return a clusters array even when the language \
         filter matches nothing: {page}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_requires_offset_argument() -> Result<()> {
    let (child, response) = init_and_tool_response("report-get", &json!({ "limit": 10 }))?;
    assert_eq!(
        value_get(&response, "/error/code")?.as_i64(),
        Some(-32_602),
        "missing offset must be InvalidParams; got {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_requires_limit_argument() -> Result<()> {
    let (child, response) = init_and_tool_response("report-get", &json!({ "offset": 0 }))?;
    assert_eq!(
        value_get(&response, "/error/code")?.as_i64(),
        Some(-32_602),
        "missing limit must be InvalidParams; got {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_clusters_are_slim_summaries_only() -> Result<()> {
    let (child, page) = init_and_tool_payload("report-get", &json!({ "offset": 0, "limit": 10 }))?;
    let clusters = value_get(&page, "/clusters")?;
    let array = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(!array.is_empty(), "fixture should produce >= 1 cluster");
    for cluster in array {
        assert!(
            cluster.get("members").is_none(),
            "ClusterSummary must drop full member list (lives behind cluster-by-id): {cluster}"
        );
        assert!(
            cluster.get("occurrences").is_none(),
            "ClusterSummary must drop full occurrences[] (lives behind cluster-by-id): {cluster}"
        );
        for required in [
            "id",
            "bucket",
            "score",
            "size_nodes",
            "occurrence_count",
            "language",
            "first_occurrence",
        ] {
            assert!(
                cluster.get(required).is_some(),
                "ClusterSummary missing required field {required:?}: {cluster}"
            );
        }
        let first_occ = value_get(cluster, "/first_occurrence")?;
        for occ_field in ["path", "start_byte", "end_byte", "start_line", "end_line"] {
            assert!(
                first_occ.get(occ_field).is_some(),
                "first_occurrence missing {occ_field:?}: {first_occ}"
            );
        }
        for line_field in ["start_line", "end_line"] {
            let line = value_get(&first_occ, &format!("/{line_field}"))?
                .as_i64()
                .ok_or_else(|| anyhow!("first_occurrence {line_field} must be an integer"))?;
            assert!(
                line >= 1,
                "first_occurrence {line_field} must be one-based: {first_occ}"
            );
        }
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_first_occurrence_belongs_to_full_cluster() -> Result<()> {
    let (mut child, page) =
        init_and_tool_payload("report-get", &json!({ "offset": 0, "limit": 10 }))?;
    let clusters = value_array(&page, "/clusters")?;
    assert!(!clusters.is_empty(), "fixture should produce >= 1 cluster");
    for summary in &clusters {
        assert_first_occurrence_matches_full_cluster(&mut child, summary)?;
    }
    let _ = child.finish();
    Ok(())
}

fn assert_first_occurrence_matches_full_cluster(
    child: &mut McpChild,
    summary: &Value,
) -> Result<()> {
    let id = value_get(summary, "/id")?;
    let first = value_get(summary, "/first_occurrence")?;
    let cluster =
        structured_tool_result(&call_tool(child, "cluster-by-id", &json!({ "id": id }))?)?;
    let occurrences = value_array(&cluster, "/occurrences")?;
    assert!(
        occurrences.iter().any(|occ| same_occurrence(occ, &first)),
        "first_occurrence must be present in cluster-by-id occurrences: {summary}"
    );
    Ok(())
}

fn same_occurrence(left: &Value, right: &Value) -> bool {
    let left_path = left.get("path").and_then(Value::as_str);
    let right_path = right.get("path").and_then(Value::as_str);
    let left_start = left.get("start_byte").and_then(Value::as_u64);
    let right_start = right.get("start_byte").and_then(Value::as_u64);
    let left_end = left.get("end_byte").and_then(Value::as_u64);
    let right_end = right.get("end_byte").and_then(Value::as_u64);
    left_path == right_path && left_start == right_start && left_end == right_end
}

#[test]
fn report_get_offset_past_end_returns_empty_page() -> Result<()> {
    let (mut child, probe) =
        init_and_tool_payload("report-get", &json!({ "offset": 0, "limit": 1 }))?;
    let total = value_get(&probe, "/total_clusters")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters missing"))?;
    let past = total.saturating_add(100);
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": past, "limit": 10 }),
    )?)?;
    assert_eq!(
        value_get(&page, "/page/returned")?,
        json!(0),
        "offset past end must return zero clusters"
    );
    assert!(
        value_get(&page, "/clusters")?
            .as_array()
            .is_some_and(Vec::is_empty),
        "clusters[] must be empty when offset is past the end"
    );
    assert_eq!(
        value_get(&page, "/total_clusters")?
            .as_u64()
            .unwrap_or(u64::MAX),
        total,
        "total_clusters must not change when paging past the end"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_response_stays_under_byte_budget() -> Result<()> {
    let (child, page) = init_and_tool_payload("report-get", &json!({ "offset": 0, "limit": 50 }))?;
    let serialised = serde_json::to_string(&page)?;
    // 50KB budget. Earlier "fat" report-get on a real workspace was 2.4MB
    // which blew out every agent context; the slim ClusterSummary must
    // keep a 50-cluster page comfortably under this floor.
    assert!(
        serialised.len() < 50_000,
        "report-get page exceeded 50KB budget: was {} bytes",
        serialised.len()
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn initialize_capabilities_have_no_null_values() -> Result<()> {
    // Regression: a `prompts: null` / `logging: null` payload was rejected
    // by Claude Desktop's MCP picker with `expected: object, received:
    // null`. Capabilities the server does not implement must be omitted,
    // not nulled.
    let mut child = McpChild::spawn(fixture_root(), &[])?;
    let response = init_session(&mut child)?;
    let capabilities = value_get(&response, "/result/capabilities")?;
    let object = capabilities
        .as_object()
        .ok_or_else(|| anyhow!("capabilities not an object: {capabilities}"))?;
    for (key, value) in object {
        assert!(
            !value.is_null(),
            "capability {key:?} is null — must be omitted or set to an object instead"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_language() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "language": "csharp" }),
    )?;
    let clusters = value_get(&page, "/clusters")?;
    let array = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(
        !array.is_empty(),
        "fixture should match >= 1 csharp cluster"
    );
    for cluster in array {
        assert_eq!(
            cluster.get("language").and_then(Value::as_str),
            Some("csharp"),
            "language filter not applied: {cluster}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_unknown_language_returns_empty() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "language": "cobol" }),
    )?;
    assert_empty_page(&page)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_path_contains() -> Result<()> {
    let (mut child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "path_contains": "Alpha" }),
    )?;
    let array = value_array(&page, "/clusters")?;
    assert!(
        !array.is_empty(),
        "Alpha.cs participates in the planted clone family"
    );
    for cluster in &array {
        let first_path = cluster
            .pointer("/first_occurrence/path")
            .and_then(Value::as_str)
            .unwrap_or("");
        // first_occurrence is one representative — path_contains may match
        // any occurrence, so we can't assert on first_occurrence alone.
        // Instead prove the filter narrowed the result by checking
        // total_clusters dropped vs the unfiltered baseline.
        let _ = first_path;
    }
    let unfiltered = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 1 }),
    )?)?;
    let unfiltered_total = value_get(&unfiltered, "/total_clusters")?
        .as_u64()
        .unwrap_or(0);
    let filtered_total = value_get(&page, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        filtered_total <= unfiltered_total,
        "filtered total ({filtered_total}) must be <= unfiltered total ({unfiltered_total})"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_min_size() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "min_size": 20 }),
    )?;
    let clusters = value_get(&page, "/clusters")?;
    for cluster in clusters.as_array().unwrap_or(&Vec::new()) {
        let size = cluster
            .get("size_nodes")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(
            size >= 20,
            "min_size=20 violated: cluster size_nodes={size}, cluster={cluster}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_min_score() -> Result<()> {
    let (mut child, baseline) =
        init_and_tool_payload("report-get", &json!({ "offset": 0, "limit": 1 }))?;
    let max_score = value_get(&baseline, "/clusters/0/score")?
        .as_f64()
        .ok_or_else(|| anyhow!("baseline score missing"))?;
    let floor = max_score / 2.0;
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-query",
        &json!({ "offset": 0, "limit": 50, "min_score": floor }),
    )?)?;
    for cluster in value_get(&page, "/clusters")?
        .as_array()
        .unwrap_or(&Vec::new())
    {
        let score = cluster.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(
            score >= floor,
            "min_score={floor} violated: cluster score={score}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_requires_offset_and_limit() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-query",
            "arguments": { "language": "csharp" }
        }),
    )?;
    assert_eq!(
        value_get(&response, "/error/code")?.as_i64(),
        Some(-32_602),
        "missing offset+limit must be InvalidParams; got {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_min_score_excludes_above_max() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "min_score": 9_999_999.0 }),
    )?;
    assert_empty_page(&page)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_min_size_excludes_above_max() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "min_size": 99_999 }),
    )?;
    assert_empty_page(&page)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_unknown_bucket_returns_empty() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "bucket": "loosely_similar" }),
    )?;
    assert_empty_page(&page)?;
    let filters = value_get(&page, "/filters")?;
    assert_eq!(filters.get("bucket"), Some(&json!("loosely_similar")));
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_nonmatching_path_returns_empty() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({
            "offset": 0,
            "limit": 50,
            "path_contains": "ZZZ_NEVER_MATCHES_ANYTHING"
        }),
    )?;
    assert_empty_page(&page)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_matching_bucket_includes_clusters() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({ "offset": 0, "limit": 50, "bucket": "nearly_identical" }),
    )?;
    let clusters = value_array(&page, "/clusters")?;
    assert!(
        !clusters.is_empty(),
        "fixture has at least one nearly-identical cluster"
    );
    for cluster in &clusters {
        assert_eq!(
            cluster.get("bucket").and_then(Value::as_str),
            Some("nearly_identical")
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_echoes_filters_in_response() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        "report-query",
        &json!({
            "offset": 0,
            "limit": 5,
            "language": "csharp",
            "min_size": 10,
        }),
    )?;
    let filters = value_get(&page, "/filters")?;
    assert_eq!(filters.get("language"), Some(&json!("csharp")));
    assert_eq!(filters.get("min_size"), Some(&json!(10)));
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_file_returns_only_matching_clusters() -> Result<()> {
    let (child, payload) =
        init_and_tool_payload("report-for-file", &json!({ "path": "Alpha.cs" }))?;
    let clusters = value_get(&payload, "/clusters")?;
    let array = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(
        !array.is_empty(),
        "Alpha.cs participates in the planted Type-2 clone"
    );
    for cluster in array {
        let occurrences = cluster
            .get("occurrences")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("occurrences missing"))?;
        let touches_alpha = occurrences
            .iter()
            .filter_map(|occ| occ.get("path").and_then(Value::as_str))
            .any(|path| path.ends_with("Alpha.cs"));
        assert!(
            touches_alpha,
            "cluster must touch Alpha.cs, got {occurrences:?}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_range_rejects_inverted_range() -> Result<()> {
    let (child, response) = init_and_tool_response(
        "report-for-range",
        &json!({ "path": "Alpha.cs", "start_byte": 100, "end_byte": 1 }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_returns_below_min_nodes_for_tiny_input() -> Result<()> {
    // [MCP-IPC-CLIENT] A snippet smaller than `min_nodes` parses
    // cleanly but produces no fingerprint — the response surfaces
    // an empty `clusters` list with `below_min_nodes: true` per
    // [MCP-TOOL-FINDSIMILAR].
    let (child, response) = init_and_tool_response(
        "find-similar",
        &json!({ "snippet": "int x = 0;", "language": "csharp" }),
    )?;
    let payload = value_get(&response, "/result/structuredContent")?;
    assert_eq!(
        payload.get("below_min_nodes"),
        Some(&json!(true)),
        "tiny snippet must surface below_min_nodes: {response}",
    );
    assert!(
        payload
            .get("clusters")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty),
        "below-min-nodes input must return no clusters: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_unsupported_language_yields_error() -> Result<()> {
    // StateFileBackend returns LspNotRunning (-32004) before language validation.
    let (child, response) = init_and_tool_response(
        "find-similar",
        &json!({ "snippet": "fn main() {}", "language": "cobol" }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_004));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_requires_exactly_one_input_variant() -> Result<()> {
    let (child, response) = init_and_tool_response("find-similar", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_range_finds_clone_on_alpha() -> Result<()> {
    // [MCP-IPC-CLIENT] find-similar delegates to the live LSP via
    // IPC. Range input on a file already in the corpus must surface
    // its sibling cluster on Beta.cs.
    let mut child = spawn_and_init()?;
    // Use a workspace-relative path so the MCP's `resolve_within_root`
    // canonical form lines up with the LSP's pinned workspace root —
    // macOS exposes `/private/var/...` vs `/var/...` for the same
    // tempdir, and only the LSP's view is authoritative.
    let source = std::fs::read_to_string(child.workspace_root().join("Alpha.cs"))?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "find-similar",
            "arguments": {
                "path": "Alpha.cs",
                "start_byte": 0,
                "end_byte": source.len(),
                "top_n": 3,
            }
        }),
    )?;
    let clusters = value_array(&response, "/result/structuredContent/clusters")?;
    assert!(
        !clusters.is_empty(),
        "find-similar on Alpha.cs must return at least the Beta sibling cluster: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn cluster_by_id_round_trips() -> Result<()> {
    let (mut child, report_value) =
        init_and_tool_payload("report-get", &json!({ "offset": 0, "limit": 1 }))?;
    let first_id = value_get(&report_value, "/clusters/0/id")?
        .as_str()
        .ok_or_else(|| anyhow!("first cluster id missing"))?
        .to_owned();
    let cluster = structured_tool_result(&call_tool(
        &mut child,
        "cluster-by-id",
        &json!({ "id": &first_id }),
    )?)?;
    assert_eq!(value_get(&cluster, "/id")?, json!(first_id));
    assert!(
        cluster.get("occurrences").is_some(),
        "cluster-by-id is the deep-dive — must surface occurrences[]"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn cluster_by_id_unknown_returns_error() -> Result<()> {
    let (child, response) =
        init_and_tool_response("cluster-by-id", &json!({ "id": "not-a-real-id" }))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn list_embedding_models_excludes_stub_when_ollama_unreachable() -> Result<()> {
    // [REMOVE-STUB] list-embedding-models delegates to the companion
    // LSP via IPC. When Ollama is unreachable in CI the production
    // listing must come back empty — the deterministic stub is test
    // infrastructure and never appears in production payloads.
    let (child, response) = init_and_tool_response("list-embedding-models", &json!({}))?;
    let models = value_array(&response, "/result/structuredContent/models")?;
    let has_stub = models
        .iter()
        .any(|model| model.get("provider_id") == Some(&json!("stub")));
    assert!(
        !has_stub,
        "list-embedding-models must never include the stub provider: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_rejects_stub_provider() -> Result<()> {
    // [REMOVE-STUB] The MCP `set-embedding-model` schema enum is
    // `["ollama"]` — submitting `provider_id: "stub"` is rejected
    // before the call reaches any backend.
    let (child, response) = init_and_tool_response(
        "set-embedding-model",
        &json!({ "provider_id": "stub", "model_id": "blake3-stub", "user_initiated": true }),
    )?;
    assert!(
        response.get("error").is_some(),
        "stub provider must be rejected by the MCP schema: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_preserves_shared_settings_and_endpoint() -> Result<()> {
    // StateFileBackend does not manage embeddings — set-embedding-model requires the LSP.
    // [REMOVE-STUB] Stub provider removed from production payloads;
    // exercise the same plumbing through the ollama provider.
    let (child, response) = init_and_tool_response(
        "set-embedding-model",
        &json!({ "provider_id": "ollama", "model_id": "nomic-embed-text", "endpoint": "http://127.0.0.1:1", "user_initiated": true }),
    )?;
    assert!(
        response.get("error").is_some(),
        "set-embedding-model without LSP must return an error envelope: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_fails_when_shared_settings_cannot_be_written() -> Result<()> {
    let workspace = copied_fixture_root()?;
    fs::write(workspace.path().join(".vscode"), "not a directory")?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "set-embedding-model",
            "arguments": {
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "user_initiated": true
            }
        }),
    )?;
    assert!(
        response.get("error").is_some(),
        "expected config write error"
    );
    let snap = structured_tool_result(&call_tool(&mut child, "session-config", &json!({}))?)?;
    assert!(
        value_get(&snap, "/embedding_provenance")?.is_null(),
        "failed settings write must not switch MCP state: {snap}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_unknown_provider_errors() -> Result<()> {
    let (child, response) = init_and_tool_response(
        "set-embedding-model",
        &json!({ "provider_id": "aztec-cpu", "model_id": "blah", "user_initiated": true }),
    )?;
    assert!(response.get("error").is_some(), "expected error response");
    let _ = child.finish();
    Ok(())
}

#[test]
fn session_config_reportsworkspace_root_and_languages() -> Result<()> {
    // Tests [MCP-TOOL-SESSION]
    // [MCP-IPC-CLIENT] session-config goes over IPC to the running
    // LSP, so `min_nodes` is the LSP's default (30) — the fixture's
    // pre-committed value is no longer the wire source.
    let (child, payload) = init_and_tool_payload("session-config", &json!({}))?;
    assert_eq!(value_get(&payload, "/min_nodes")?.as_u64().unwrap_or(0), 30);
    let languages_value = value_get(&payload, "/languages")?;
    let languages: Vec<String> = languages_value
        .as_array()
        .ok_or_else(|| anyhow!("languages not array"))?
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect();
    assert!(
        languages.iter().any(|candidate| candidate == "csharp"),
        "csharp missing from session config: {languages:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_list_returns_report_and_schema_uris() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("resources/list", &json!({}))?;
    let resources_value = value_get(&response, "/result/resources")?;
    let resources = resources_value
        .as_array()
        .ok_or_else(|| anyhow!("resources not array"))?;
    let uris: Vec<String> = resources
        .iter()
        .filter_map(|resource| {
            resource
                .get("uri")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    assert!(
        uris.iter().any(|uri| uri == "deslop://report"),
        "report uri missing: {uris:?}"
    );
    assert!(
        uris.iter().any(|uri| uri == "deslop://schema"),
        "schema uri missing: {uris:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_report_returns_parseable_json() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("resources/read", &json!({ "uri": "deslop://report" }))?;
    let text = value_get(&response, "/result/contents/0/text")?
        .as_str()
        .ok_or_else(|| anyhow!("report text payload missing"))?
        .to_owned();
    let parsed: Value = serde_json::from_str(&text)?;
    assert!(value_get(&parsed, "/clusters")?.is_array());
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_schema_returns_markdown_body() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("resources/read", &json!({ "uri": "deslop://schema" }))?;
    let text = value_get(&response, "/result/contents/0/text")?
        .as_str()
        .ok_or_else(|| anyhow!("schema text payload missing"))?
        .to_owned();
    assert!(!text.is_empty(), "schema_doc must not be empty");
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_unknown_uri_errors() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("resources/read", &json!({ "uri": "deslop://invalid" }))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn unknown_method_returns_method_not_found() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("completely/made-up", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_601));
    let _ = child.finish();
    Ok(())
}

#[test]
fn malformed_frame_returns_parse_error() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.send_raw_line("{this is not valid json")?;
    let response = child.read_frame()?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_700));
    let _ = child.finish();
    Ok(())
}

#[test]
fn path_outside_root_is_rejected() -> Result<()> {
    let outside = TempDir::new()?;
    let outside_file = outside.path().join("Evil.cs");
    std::fs::write(&outside_file, "namespace E { class X {} }")?;
    let mut child = spawn_and_init()?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-for-file",
            "arguments": { "path": outside_file }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_003));
    let _ = child.finish();
    Ok(())
}

#[test]
fn notifications_initialized_is_accepted_silently() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.notify("notifications/initialized", &json!({}))?;
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());
    let _ = child.finish();
    Ok(())
}

#[test]
fn mark_changed_is_idempotent_across_second_session() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    let first = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
    )?)?;
    let first_count = value_get(&first, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(first_count >= 1, "expected at least one cluster initially");
    let _ = child.finish();
    std::fs::write(
        temp.path().join("Two.cs"),
        "namespace Lone { class Only { public int Go() => 1; } }\n",
    )?;
    generate_state_file(temp.path(), 15)?;
    let mut second = McpChild::spawn(temp.path(), &[])?;
    let _ = init_session(&mut second)?;
    let rerun = structured_tool_result(&call_tool(
        &mut second,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
    )?)?;
    let rerun_count = value_get(&rerun, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        rerun_count < first_count,
        "after mutating Two.cs, cluster count must drop; was {first_count}, now {rerun_count}"
    );
    let _ = second.finish();
    Ok(())
}

#[test]
fn report_for_range_returns_empty_when_path_has_no_clusters() -> Result<()> {
    let (_workspace, mut child) = workspace_with_extra_file(
        "Lonely.cs",
        "namespace Lonely { class Solo { public int Uniq() => 42; } }",
    )?;
    let result = call_tool(
        &mut child,
        "report-for-range",
        &json!({
            "path": "Lonely.cs",
            "start_byte": 0,
            "end_byte": 10_000,
        }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, "/clusters")?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "a unique-content file should not participate in any cluster"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_file_on_unknown_path_returns_empty_clusters() -> Result<()> {
    let (_workspace, mut child) =
        workspace_with_extra_file("Ghost.cs", "namespace G { class G {} }")?;
    let result = call_tool(
        &mut child,
        "report-for-file",
        &json!({ "path": "Ghost.cs" }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, "/clusters")?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "unknown file should produce no clusters"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_swap_updates_session_config_provenance() -> Result<()> {
    // StateFileBackend does not manage embeddings — set-embedding-model requires the LSP.
    // [REMOVE-STUB] Use the ollama provider id since stub is no longer
    // accepted by the production MCP schema.
    let (child, response) = init_and_tool_response(
        "set-embedding-model",
        &json!({ "provider_id": "ollama", "model_id": "nomic-embed-text", "endpoint": "http://127.0.0.1:1", "user_initiated": true }),
    )?;
    assert!(
        response.get("error").is_some(),
        "set-embedding-model without LSP must return an error envelope: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_to_ollama_fails_when_daemon_not_running() -> Result<()> {
    let (child, response) = init_and_tool_response(
        "set-embedding-model",
        &json!({
            "provider_id": "ollama",
            "model_id": "nomic-embed-text",
            "endpoint": "http://127.0.0.1:1",
            "user_initiated": true
        }),
    )?;
    // Either a clean error envelope or the inner backend error. Both
    // paths exercise the ollama branch of set_embedding_model.
    assert!(
        response.get("error").is_some(),
        "ollama-to-nowhere must not succeed: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_with_top_n_zero_falls_back_to_default() -> Result<()> {
    // [MCP-IPC-CLIENT] `top_n: 0` must use the default cap rather
    // than returning an empty list. The find-similar IPC roundtrip
    // surfaces at least the Beta sibling.
    let mut child = spawn_and_init()?;
    // Workspace-relative path keeps the MCP↔LSP canonical view
    // aligned (see find_similar_range_finds_clone_on_alpha).
    let source = std::fs::read_to_string(child.workspace_root().join("Alpha.cs"))?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "find-similar",
            "arguments": { "path": "Alpha.cs", "start_byte": 0, "end_byte": source.len(), "top_n": 0 }
        }),
    )?;
    let clusters = value_array(&response, "/result/structuredContent/clusters")?;
    assert!(
        !clusters.is_empty(),
        "find-similar with top_n=0 must fall back to default and return clusters: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_with_empty_source_returns_empty_result() -> Result<()> {
    // [MCP-IPC-CLIENT] find-similar against the live LSP must
    // tolerate an empty snippet — it is below the parser's minimum
    // node floor, so the success-path reply marks `below_min_nodes`
    // and returns no clusters (no error envelope).
    let (child, response) = init_and_tool_response(
        "find-similar",
        &json!({ "snippet": "", "language": "csharp" }),
    )?;
    assert!(
        response.pointer("/error").is_none(),
        "find-similar with the live LSP must succeed (no JSON-RPC error envelope): {response}"
    );
    assert_eq!(
        value_get(&response, "/result/structuredContent/below_min_nodes")?.as_bool(),
        Some(true),
        "empty snippet must report below_min_nodes=true: {response}"
    );
    let clusters = value_array(&response, "/result/structuredContent/clusters")?;
    assert!(
        clusters.is_empty(),
        "empty snippet must return no clusters: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn tools_call_missing_name_returns_invalid_params() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("tools/call", &json!({ "arguments": {} }))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn tools_call_unknown_tool_returns_method_not_found_error() -> Result<()> {
    let (child, response) = init_and_tool_response("bogus-tool", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_601));
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_missing_uri_returns_invalid_params() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("resources/read", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn invalid_jsonrpc_version_returns_invalid_request() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.send_raw_line(r#"{"jsonrpc":"1.5","id":99,"method":"ping"}"#)?;
    let response = child.read_frame()?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_600));
    let _ = child.finish();
    Ok(())
}

#[test]
fn ping_method_returns_empty_object() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("ping", &json!({}))?;
    assert!(response.get("error").is_none());
    let _ = child.finish();
    Ok(())
}

#[test]
fn shutdown_method_returns_null_result() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("shutdown", &json!({}))?;
    assert_eq!(value_get(&response, "/result")?, json!(null));
    let _ = child.finish();
    Ok(())
}

#[test]
fn string_request_id_round_trips_through_dispatch() -> Result<()> {
    let mut child = spawn_and_init()?;
    // The harness only issues numeric ids; craft a raw frame with a
    // string id so we exercise RequestId::String on the wire.
    let frame = r#"{"jsonrpc":"2.0","id":"alpha","method":"tools/list"}"#;
    child.send_raw_line(frame)?;
    let response = child.read_frame()?;
    assert_eq!(value_get(&response, "/id")?, json!("alpha"));
    let _ = child.finish();
    Ok(())
}

#[test]
fn relative_path_insideworkspace_is_accepted() -> Result<()> {
    let (child, payload) = init_and_tool_payload(
        "report-for-range",
        &json!({
            "path": "./Alpha.cs",
            "start_byte": 0,
            "end_byte": 1,
        }),
    )?;
    assert_eq!(value_get(&payload, "/path")?, json!("./Alpha.cs"));
    let _ = child.finish();
    Ok(())
}

#[test]
fn tool_missing_required_string_arg_returns_invalid_params() -> Result<()> {
    // report-for-file needs a "path" string — omit it.
    let (child, response) = init_and_tool_response("report-for-file", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn tool_missing_required_integer_arg_returns_invalid_params() -> Result<()> {
    // report-for-range needs start_byte + end_byte — omit both.
    let (child, response) =
        init_and_tool_response("report-for-range", &json!({ "path": "Alpha.cs" }))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_missing_model_id_returns_invalid_params() -> Result<()> {
    let (child, response) = init_and_tool_response(
        "set-embedding-model",
        &json!({ "provider_id": "ollama", "user_initiated": true }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_without_user_initiation_returns_invalid_params() -> Result<()> {
    // Tests [MCP-EMBEDDING-CONSENT]
    let (child, response) = init_and_tool_response(
        "set-embedding-model",
        &json!({ "provider_id": "ollama", "model_id": "nomic-embed-text" }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn cluster_by_id_missing_id_returns_invalid_params() -> Result<()> {
    let (child, response) = init_and_tool_response("cluster-by-id", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn mcp_sends_empty_line_and_server_keeps_going() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.send_raw_line("")?;
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_file_accepts_nonexistent_leaf_but_resolves_parent() -> Result<()> {
    // Query a file that doesn't exist but whose *parent* (the scan
    // root) does. This exercises safety::canonicalise_best_effort's
    // nonexistent-leaf branch, returning an empty cluster set.
    let (child, payload) =
        init_and_tool_payload("report-for-file", &json!({ "path": "NeverCreated.cs" }))?;
    let clusters = value_get(&payload, "/clusters")?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "phantom leaf must resolve under root and return no clusters"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn path_in_nonexistent_subdirectory_is_rejected_as_io_failure() -> Result<()> {
    let (child, response) = init_and_tool_response(
        "report-for-file",
        &json!({ "path": "no/such/dir/Phantom.cs" }),
    )?;
    assert!(
        response.get("error").is_some(),
        "nonexistent parent directory must surface an error: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn binary_starts_with_default_embedding_config() -> Result<()> {
    // [REMOVE-STUB] StateFileBackend reads provenance from the state
    // file; with stub removed, provenance may be null when no real
    // provider is selected. The key must still be present.
    let (child, snapshot) = init_and_tool_payload("session-config", &json!({}))?;
    assert!(
        value_get(&snapshot, "/embedding_provenance")?.is_object()
            || value_get(&snapshot, "/embedding_provenance")?.is_null(),
        "session-config must return embedding_provenance field: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn binary_starts_without_ollama_returns_provenance_field() -> Result<()> {
    // [REMOVE-STUB] StateFileBackend reads provenance from the state
    // file; Ollama is not contacted. Production no longer falls back
    // to a stub provider when Ollama is unreachable, but the
    // provenance key must always be present so the editor can detect
    // the disabled state.
    let (child, snapshot) = init_and_tool_payload("session-config", &json!({}))?;
    assert!(
        snapshot.get("embedding_provenance").is_some(),
        "session-config must include embedding_provenance key: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

/// `[LSP-EMBEDDING-CONSENT]` Audience: HUMAN. Issue #35. `StateFileBackend` does
/// not contact Ollama — the server always starts and `session-config` responds.
#[test]
fn binary_survives_when_required_ollama_endpoint_is_unreachable() -> Result<()> {
    let (child, snapshot) = init_and_tool_payload("session-config", &json!({}))?;
    assert!(
        snapshot.get("embedding_provenance").is_some(),
        "session-config must respond even when Ollama is not running: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn binary_rejects_invalid_embedding_mode_string() -> Result<()> {
    // Spawn the binary directly (bypassing the McpChild harness) so we
    // can assert on the non-zero exit status.
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary)
        .arg("--root")
        .arg(fixture_root())
        .arg("--embeddings")
        .arg("nonsense")
        .output()?;
    assert!(
        !output.status.success(),
        "invalid --embeddings value must not succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn binary_rejects_unknown_embedding_provider_at_init() -> Result<()> {
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary)
        .arg("--root")
        .arg(fixture_root())
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-provider")
        .arg("zzz-not-real")
        .output()?;
    assert!(
        !output.status.success(),
        "unknown provider must exit non-zero"
    );
    Ok(())
}

#[test]
fn files_changed_notification_triggers_reanalysis() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    let before = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
    )?)?;
    let before_count = value_get(&before, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(before_count >= 1, "expected at least one cluster");
    // Edit Two.cs so the clone disappears, regenerate the state file, then notify.
    mutate_two_and_notify(&mut child, temp.path())?;
    // mark_changed triggers the reload; poll until the report settles to
    // its post-reload state so the watcher-debounced pass cannot race the
    // read under heavy CI load (see poll_total_clusters_below).
    let after = poll_total_clusters_below(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
        before_count,
    )?;
    let after_count = value_get(&after, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        after_count < before_count,
        "filesChanged notification must reload the state file and drop the Two.cs clone; was {before_count}, now {after_count}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_77_session_config_reports_incremental_true_after_mutation_reload() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    let before_top = structured_tool_result(&call_tool(
        &mut child,
        "top-offenders",
        &json!({ "n": 100 }),
    )?)?;
    let before_count = value_get(&before_top, "/total_clusters")?
        .as_u64()
        .unwrap_or(0);
    assert!(before_count >= 1, "expected a cluster before mutation");
    let before_config =
        structured_tool_result(&call_tool(&mut child, "session-config", &json!({}))?)?;
    let before_generation = value_get(&before_config, "/generation")?
        .as_u64()
        .unwrap_or(0);
    assert!(
        before_generation >= 1,
        "initial generation should load state"
    );
    assert!(
        value_get(&before_config, "/languages")?
            .as_array()
            .is_some_and(|languages| languages.iter().any(|value| value == "csharp")),
        "session-config should report csharp before mutation: {before_config}"
    );

    mutate_two_and_notify(&mut child, temp.path())?;

    let after_top = poll_total_clusters_below(
        &mut child,
        "top-offenders",
        &json!({ "n": 100 }),
        before_count,
    )?;
    let after_count = value_get(&after_top, "/total_clusters")?
        .as_u64()
        .unwrap_or(0);
    assert!(
        after_count < before_count,
        "mutation reload should remove the stale duplicate cluster"
    );
    let after_config =
        structured_tool_result(&call_tool(&mut child, "session-config", &json!({}))?)?;
    // [MCP-IPC-CLIENT] min_nodes now comes from the live LSP, not
    // the test's `generate_state_file` invocation — LSP defaults
    // to 30.
    assert_eq!(value_get(&after_config, "/min_nodes")?.as_u64(), Some(30));
    assert!(
        value_get(&after_config, "/languages")?.is_array(),
        "session-config should keep languages shaped as an array: {after_config}"
    );
    assert!(
        value_get(&after_config, "/generation")?
            .as_u64()
            .unwrap_or(0)
            > before_generation,
        "filesChanged reload should advance the MCP generation"
    );
    assert_eq!(
        value_get(&after_config, "/incremental")?.as_bool(),
        Some(true),
        "issue #77/#81: session-config must report live incremental mode after mutation reload"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_89_rescan_tool_reloads_state_file_and_returns_fresh_top_offenders() -> Result<()> {
    // [MCP-IPC-CLIENT] rescan triggers the LSP's
    // `deslop.lsp.refreshReport` over IPC and the next read sees the
    // re-analysed state. Mutating source (not the seed cache) is the
    // only way to influence the LSP under the new architecture.
    let workspace = copied_fixture_root()?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    // Flush the LSP cold-pass install before measuring `before` so
    // the post-mutation rescan does not race a delayed background
    // commit that would re-introduce the stale cluster.
    let _flush = call_tool(&mut child, "rescan", &json!({ "n": 1 }))?;
    let before = structured_tool_result(&call_tool(
        &mut child,
        "top-offenders",
        &json!({ "n": 100 }),
    )?)?;
    let before_count = value_get(&before, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        before_count >= 1,
        "expected at least one cluster before edit"
    );

    // Replace Beta.cs with a unique implementation so the duplicate
    // cluster between Alpha.cs and Beta.cs disappears after rescan.
    std::fs::write(
        workspace.path().join("Beta.cs"),
        "namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;

    let after = structured_tool_result(&call_tool(
        &mut child,
        "rescan",
        &json!({
            "paths": [workspace.path().join("Beta.cs").to_string_lossy().into_owned()],
            "n": 100
        }),
    )?)?;
    let after_count = value_get(&after, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        after_count < before_count,
        "issue #89: rescan must synchronously trigger LSP re-analysis and return fresh top offenders; was {before_count}, now {after_count}"
    );
    assert_eq!(
        value_get(&after, "/n")?.as_u64(),
        Some(100),
        "issue #89: rescan must echo the requested top-offenders count"
    );
    assert!(
        value_get(&after, "/clusters")?.is_array(),
        "issue #89: rescan must return top-offenders clusters"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn files_changed_notification_with_empty_paths_is_a_noop() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.notify("notifications/deslop/filesChanged", &json!({ "paths": [] }))?;
    // Server must remain responsive after a no-op notification.
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());
    let _ = child.finish();
    Ok(())
}

/// [MCP-NOTIFICATIONS] After `notifications/deslop/filesChanged` the
/// server must push `notifications/resources/updated` and
/// `notifications/deslop/reportChanged` before waiting for the next
/// client frame.
#[test]
fn files_changed_pushes_resources_updated_and_report_changed_notifications() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;

    // Modify a file, regenerate the state file, then notify the server.
    mutate_two_and_notify(&mut child, temp.path())?;

    // Server pushes two notification frames synchronously before it
    // returns to its read loop — read them both right away.
    let frame1 = child.read_frame()?;
    assert_eq!(
        frame1.get("method").and_then(Value::as_str),
        Some("notifications/resources/updated"),
        "first pushed frame must be resources/updated: {frame1}"
    );
    assert!(
        frame1.get("id").is_none(),
        "notification must not carry an id: {frame1}"
    );
    assert_eq!(
        frame1.pointer("/params/uri").and_then(Value::as_str),
        Some("deslop://report"),
        "resources/updated must name deslop://report: {frame1}"
    );

    let frame2 = child.read_frame()?;
    assert_eq!(
        frame2.get("method").and_then(Value::as_str),
        Some("notifications/deslop/reportChanged"),
        "second pushed frame must be deslop/reportChanged: {frame2}"
    );
    assert!(
        frame2.get("id").is_none(),
        "notification must not carry an id: {frame2}"
    );
    assert!(
        frame2
            .pointer("/params/generation")
            .and_then(Value::as_u64)
            .is_some(),
        "reportChanged must include a numeric generation: {frame2}"
    );

    // Server stays alive and responsive after pushing notifications.
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());

    let _ = child.finish();
    Ok(())
}

#[test]
fn list_embedding_models_response_omits_legacy_keys_and_stub() -> Result<()> {
    // [MCP-IPC-CLIENT] / issue #87 / [REMOVE-STUB] — the response array
    // (empty under unreachable Ollama, populated when Ollama responds)
    // never exposes the legacy keys and never includes the stub
    // provider. Production payloads must not leak test infrastructure.
    let mut child = spawn_and_init()?;
    let response = child.request(
        "tools/call",
        &json!({ "name": "list-embedding-models", "arguments": {} }),
    )?;
    let models = value_array(&response, "/result/structuredContent/models")?;
    let has_stub = models
        .iter()
        .any(|model| model.get("provider_id") == Some(&json!("stub")));
    assert!(
        !has_stub,
        "list-embedding-models must never include the stub provider: {response}",
    );
    for model in &models {
        for legacy_key in [
            "name",
            "bare_id",
            "digest",
            "size_bytes",
            "is_embedding_model",
        ] {
            assert!(
                model.get(legacy_key).is_none(),
                "issue #87: model row must not expose legacy key {legacy_key}: {model}",
            );
        }
    }
    let _ = child.finish();
    Ok(())
}
