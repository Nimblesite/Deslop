//! Verifies the MCP server exits cleanly when its parent process dies
//! (orphan-exit safety net mirroring the LSP `parent_process` monitor).

#![cfg(unix)]

use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    fixture_root, pid_exists, read_mcp_pid, terminate_pid, value_get, wait_for_pid_exit,
    KILLABLE_PARENT_SCRIPT,
};

struct McpParent {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    mcp_pid: Option<u32>,
    next_id: i64,
}

impl McpParent {
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
            if response.get("method").is_none() {
                return Err(anyhow!("unexpected frame without id match: {response:?}"));
            }
        }
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

    fn disarm_mcp_cleanup(&mut self) {
        self.mcp_pid = None;
    }
}

impl Drop for McpParent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(pid) = self.mcp_pid.take() {
            let _ = terminate_pid(pid);
        }
    }
}

fn init_session(parent: &mut McpParent) -> Result<Value> {
    parent.request(
        "initialize",
        &json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "mcp-orphan-exit-harness", "version": "0.1.0" }
        }),
    )
}

fn spawn_mcp_with_killable_parent(root: &Path) -> Result<(McpParent, u32)> {
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
        McpParent {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            mcp_pid: Some(mcp_pid),
            next_id: 0,
        },
        mcp_pid,
    ))
}

#[test]
fn exits_when_launching_parent_disappears_with_stdio_open_issue_102() -> Result<()> {
    let (mut parent, mcp_pid) = spawn_mcp_with_killable_parent(fixture_root())?;
    ensure!(mcp_pid > 1, "mcp pid must be monitorable");
    assert_ne!(
        mcp_pid,
        parent.child.id(),
        "test must observe the mcp child separately from its shell parent"
    );
    assert!(pid_exists(mcp_pid)?, "mcp pid must exist before initialize");
    assert!(
        parent.child.try_wait()?.is_none(),
        "launcher parent must stay alive until killed by the test"
    );

    let response = init_session(&mut parent)?;
    assert_eq!(
        value_get(&response, "/result/serverInfo/name")?,
        json!("deslop-mcp")
    );
    assert_eq!(
        value_get(&response, "/result/protocolVersion")?,
        json!("2024-11-05")
    );

    let started = Instant::now();
    parent.child.kill()?;
    let parent_status = parent.child.wait()?;
    assert!(
        !parent_status.success(),
        "launcher parent should be killed during orphan-exit test"
    );
    let exited = wait_for_pid_exit(mcp_pid, Duration::from_secs(5))?;
    let elapsed = started.elapsed();
    if !exited {
        terminate_pid(mcp_pid)?;
    }
    assert!(
        exited,
        "deslop-mcp must exit within 5s when its launching parent disappears"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "orphan-exit observation should complete within 5s, took {elapsed:?}"
    );
    assert!(
        !pid_exists(mcp_pid)?,
        "mcp pid must be gone after orphan-exit wait"
    );
    parent.disarm_mcp_cleanup();
    Ok(())
}
