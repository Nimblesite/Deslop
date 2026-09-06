//! Verifies the MCP server exits cleanly when its parent process dies
//! (orphan-exit safety net mirroring the LSP `parent_process` monitor).

#![cfg(unix)]

use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

use crate::common;
use common::{
    fixture_root, pid_exists, read_mcp_pid,
    rpc::{StdioRpc, MCP_PROTOCOL_VERSION},
    terminate_pid, value_get, wait_for_pid_exit, KILLABLE_PARENT_SCRIPT,
};

/// How long an orphaned `deslop-mcp` may take to notice and exit.
const ORPHAN_EXIT_TIMEOUT: Duration = Duration::from_secs(5);

/// The killable launcher shell, its JSON-RPC link to the MCP grandchild,
/// and that grandchild's pid so the test can reap it if it lingers.
struct McpParent {
    child: Child,
    rpc: StdioRpc,
    mcp_pid: Option<u32>,
}

impl McpParent {
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
    parent.rpc.initialize("mcp-orphan-exit-harness")
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
    let rpc = StdioRpc::take(&mut child)?;
    Ok((
        McpParent {
            child,
            rpc,
            mcp_pid: Some(mcp_pid),
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
        json!(MCP_PROTOCOL_VERSION)
    );
    assert!(
        value_get(&response, "/result/capabilities/resources")?.is_object(),
        "resources capability missing: {response}"
    );

    // Checked before the kill, not after. After it, "the mcp is still alive"
    // is the negation of the contract this test exists to prove, and it holds
    // only while the mcp has not yet noticed — so a server that reacts
    // promptly fails here, and one that reacts at all fails under contention.
    // Observed once in a full-workspace run. Before the kill the same fact is
    // certain, and it is the fact that matters: the exit below belongs to this
    // process rather than to one that was already gone.
    assert!(
        pid_exists(mcp_pid)?,
        "mcp must be alive when its parent is killed, or its exit proves nothing"
    );
    let started = Instant::now();
    parent.child.kill()?;
    let parent_status = parent.child.wait()?;
    assert!(
        !parent_status.success(),
        "launcher parent should be killed during orphan-exit test"
    );
    let exited = wait_for_pid_exit(mcp_pid, ORPHAN_EXIT_TIMEOUT)?;
    let elapsed = started.elapsed();
    if !exited {
        terminate_pid(mcp_pid)?;
    }
    assert!(
        exited,
        "deslop-mcp must exit within 5s when its launching parent disappears"
    );
    assert!(
        elapsed < ORPHAN_EXIT_TIMEOUT,
        "orphan-exit observation should complete within 5s, took {elapsed:?}"
    );
    assert!(
        !pid_exists(mcp_pid)?,
        "mcp pid must be gone after orphan-exit wait"
    );
    parent.disarm_mcp_cleanup();
    Ok(())
}
