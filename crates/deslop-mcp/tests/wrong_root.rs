//! Regression coverage for issue #141: `deslop-mcp` must bind to an
//! absolute, canonicalised workspace root. Returning duplicate
//! clusters from a wrong root (e.g. `~/.cargo/git/checkouts/...`) and
//! hanging on `session-config` are the two failure shapes the issue
//! tracks; the fixes are: canonicalise `--root` at start-up and
//! refuse to scan vendored Cargo cache trees.
//!
//! Under [MCP-IPC-CLIENT] every read tool call delegates to the LSP
//! over a unix socket, so each test spawns a companion `deslop-lsp`
//! against the same workspace before driving the MCP child.

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use tempfile::TempDir;

const SOCKET_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Kill-on-drop guard for the companion LSP child so per-test
/// workspaces can be reclaimed without leaking sockets when the test
/// panics.
struct LspGuard(Child);

impl Drop for LspGuard {
    fn drop(&mut self) {
        let _killed = self.0.kill();
        let _waited = self.0.wait();
    }
}

fn spawn_lsp(root: &Path) -> Result<LspGuard> {
    let bin = cargo_bin("deslop-lsp");
    let mut child = Command::new(bin)
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn deslop-lsp")?;
    let mut stdin = child.stdin.take().context("lsp stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().context("lsp stdout")?);
    lsp_handshake(&mut stdin, &mut stdout).context("lsp handshake")?;
    child.stdin = Some(stdin);
    child.stdout = Some(stdout.into_inner());
    Ok(LspGuard(child))
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
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(payload.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

fn read_lsp_frame(reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let _bytes = reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = Some(rest.trim().parse::<usize>()?);
        }
    }
    let length = content_length.ok_or_else(|| anyhow!("missing Content-Length"))?;
    let mut buf = vec![0_u8; length];
    reader.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

fn wait_for_socket(path: &Path) -> Result<()> {
    let started = Instant::now();
    loop {
        if path.exists() {
            return Ok(());
        }
        if started.elapsed() >= SOCKET_TIMEOUT {
            return Err(anyhow!(
                "timed out waiting for socket at {}",
                path.display()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

struct McpChild {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
    /// Companion LSP. Reaped on drop so each test tears down both
    /// processes before the workspace tempdir is reclaimed.
    _lsp: LspGuard,
}

impl McpChild {
    fn spawn_with_cwd(cwd: &Path, root_arg: &str) -> Result<Self> {
        let lsp = spawn_lsp(cwd)?;
        let socket = cwd.join(".deslop-cache").join("deslop.sock");
        wait_for_socket(&socket)?;
        let binary = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut cmd = Command::new(binary);
        let _ = cmd
            .arg("--root")
            .arg(root_arg)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().context("spawn deslop-mcp binary")?;
        let stdin = child.stdin.take().context("child stdin")?;
        let stdout = child.stdout.take().context("child stdout")?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 0,
            _lsp: lsp,
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
            if response.get("id").cloned().unwrap_or(Value::Null) == json!(id) {
                return Ok(response);
            }
        }
    }

    fn read_frame(&mut self) -> Result<Value> {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line)?;
        if bytes == 0 {
            return Err(anyhow!("mcp stdout closed unexpectedly"));
        }
        serde_json::from_str(&line).with_context(|| format!("invalid JSON from mcp: {line}"))
    }

    fn send_frame(&mut self, frame: &Value) -> Result<()> {
        let bytes = serde_json::to_vec(frame)?;
        self.stdin.write_all(&bytes)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn finish(mut self) {
        drop(self.stdin);
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn init(child: &mut McpChild) -> Result<()> {
    let _ = child.request(
        "initialize",
        &json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "wrong-root-test", "version": "0.1.0" }
        }),
    )?;
    Ok(())
}

fn call_tool(child: &mut McpChild, name: &str) -> Result<Value> {
    let response = child.request("tools/call", &json!({ "name": name, "arguments": {} }))?;
    if let Some(error) = response.get("error") {
        return Err(anyhow!("tools/call {name} failed: {error}"));
    }
    response
        .get("result")
        .and_then(|result| result.get("structuredContent"))
        .cloned()
        .ok_or_else(|| anyhow!("missing structuredContent in {response}"))
}

fn seed_one_csharp_file(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join("OnlyFile.cs"),
        "namespace OnlyNs { public class OnlyClass { public int OnlyMethod() { return 1; } } }\n",
    )?;
    Ok(())
}

// Issue #141: starting `deslop-mcp --root .` while CWD points at the
// intended workspace must bind the session to the absolute,
// canonicalised path of that directory — never the literal "." or a
// path elsewhere. Without this the renderer reports occurrences from
// a different filesystem root than the client thinks it is asking
// about.
#[test]
fn issue_141_relative_root_resolves_to_canonical_workspace() -> Result<()> {
    let workspace = TempDir::new()?;
    seed_one_csharp_file(workspace.path())?;
    let canonical_workspace = fs::canonicalize(workspace.path())?;

    let mut child = McpChild::spawn_with_cwd(&canonical_workspace, ".")?;
    init(&mut child)?;
    let snapshot = call_tool(&mut child, "session-config")?;
    let reported_root = snapshot
        .get("root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("session-config did not return a string root: {snapshot}"))?;
    assert!(
        reported_root.is_absolute(),
        "session-config root must be absolute, got {reported_root:?}",
    );
    let canonical_reported = fs::canonicalize(&reported_root)?;
    assert_eq!(
        canonical_reported, canonical_workspace,
        "session-config root must canonicalise to the launch CWD",
    );
    child.finish();
    Ok(())
}

// Issue #141 / #142: even when MCP is correctly rooted, vendored
// Cargo cache trees nested under the scan root (e.g. a developer who
// runs `cargo` inside the workspace) must not enter discovery. The
// built-in exclude set covers `.cargo/`; this test asserts the live
// MCP backend honours it.
#[test]
fn issue_141_cargo_cache_under_root_is_not_analysed() -> Result<()> {
    let workspace = TempDir::new()?;
    let canonical_workspace = fs::canonicalize(workspace.path())?;
    seed_one_csharp_file(&canonical_workspace)?;
    let cargo_checkouts = canonical_workspace
        .join(".cargo")
        .join("git")
        .join("checkouts")
        .join("ruff-deadbeef")
        .join("c6516e9");
    fs::create_dir_all(&cargo_checkouts)?;
    for index in 0..3 {
        fs::write(
            cargo_checkouts.join(format!("Generated{index}.cs")),
            "namespace G { public class W { public int R() { return 1; } } }\n",
        )?;
    }

    let workspace_str = canonical_workspace
        .to_str()
        .ok_or_else(|| anyhow!("workspace path is not UTF-8"))?;
    let mut child = McpChild::spawn_with_cwd(&canonical_workspace, workspace_str)?;
    init(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({ "name": "report-get", "arguments": { "offset": 0, "limit": 50 } }),
    )?;
    let result = response
        .get("result")
        .and_then(|r| r.get("structuredContent"))
        .cloned()
        .ok_or_else(|| anyhow!("missing structuredContent: {response}"))?;
    let clusters = result
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for cluster in clusters {
        let summary = cluster.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(
            !summary.contains(".cargo/"),
            "cluster summary leaked cargo cache: {summary}",
        );
    }
    child.finish();
    Ok(())
}
