//! Process-id plumbing for the orphan-exit suite ([MCP-TESTING]): a
//! killable parent that launches `deslop-mcp`, and pid liveness checks.

use std::{
    io::BufReader,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{ensure, Context, Result};

/// Shell wrapper that launches `deslop-mcp` as a grandchild and prints its
/// pid to stderr, so the orphan-exit suites can kill the launching parent
/// while keeping the MCP child stdio open.
#[cfg(unix)]
pub const KILLABLE_PARENT_SCRIPT: &str = r#"exec 3<&0; "$1" --root "$2" <&3 2>/dev/null & mcp_pid=$!; printf '%s\n' "$mcp_pid" >&2; wait "$mcp_pid""#;

/// Reads the MCP pid the killable-parent shell prints on its stderr line.
#[cfg(unix)]
pub fn read_mcp_pid(child: &mut Child) -> Result<u32> {
    use std::io::BufRead;
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
