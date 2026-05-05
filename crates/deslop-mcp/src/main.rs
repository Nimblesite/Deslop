//! `deslop-mcp` binary — MCP server over stdio.
//!
//! Thin shell over [`deslop_mcp::McpServer`]: parse CLI args,
//! configure tracing, construct the [`StateFileBackend`], and
//! drive the server against stdin / stdout.

use std::{env, io, path::PathBuf, sync::Arc};

#[cfg(unix)]
use std::{thread, time::Duration};

use clap::Parser;
use deslop_core::{version_contract_output, ComponentKind};
use deslop_mcp::{McpServer, SessionBackendConfig, StateFileBackend};
use tracing::error;
use tracing_subscriber::EnvFilter;

/// `deslop-mcp` — MCP server over stdio exposing live clone
/// detection to AI agents.
#[derive(Debug, Parser)]
#[command(
    name = "deslop-mcp",
    version,
    about = "Model Context Protocol server exposing Deslop live analysis to AI agents."
)]
struct Cli {
    /// Workspace root to analyse. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Optional `.deslop.toml` override path.
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match version_contract_output(&args, "deslop-mcp", ComponentKind::Mcp) {
        Ok(Some(output)) => {
            print!("{output}");
            return;
        }
        Ok(None) => {}
        Err(err) => {
            error!(reason = %err, "mcp_version_contract_failure");
            std::process::exit(1);
        }
    }
    if let Err(err) = run(args) {
        error!(reason = %err, "mcp_server_failure");
        std::process::exit(1);
    }
}

/// Entry point.
///
/// # Errors
///
/// Returns any error surfaced by backend construction or the
/// transport loop.
fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    install_tracing();
    start_parent_monitor();
    let cli = Cli::parse_from(args);
    let config = SessionBackendConfig {
        root: cli.root,
        config_path: cli.config,
    };
    let backend = Arc::new(StateFileBackend::initialise(config)?);
    let server = McpServer::new(backend);
    let stdin = io::stdin();
    // io::Stdout is Write + Send + 'static; StdoutLock<'_> is not Send,
    // so we pass the unlocked handle so background threads can push
    // notifications through the shared NotificationSender.
    server.run(stdin.lock(), io::stdout())?;
    Ok(())
}

/// Poll interval for detecting when the MCP launcher disappears.
#[cfg(unix)]
const PARENT_MONITOR_INTERVAL_MS: u64 = 250;

/// Starts a detached monitor that exits MCP when its launcher dies.
#[cfg(unix)]
fn start_parent_monitor() {
    let Some(parent_process_id) = current_parent_process_id() else {
        return;
    };
    match thread::Builder::new()
        .name("deslop-mcp-parent-process-monitor".to_owned())
        .spawn(move || monitor_parent(parent_process_id))
    {
        Ok(handle) => drop(handle),
        Err(error) => tracing::warn!(
            %error,
            parent_process_id,
            "failed to start mcp parent process monitor",
        ),
    }
}

/// Keeps MCP startup portable on platforms without parent-id probing.
#[cfg(not(unix))]
fn start_parent_monitor() {}

/// Polls the original parent until it disappears, then exits MCP.
#[cfg(unix)]
fn monitor_parent(parent_process_id: u32) -> ! {
    loop {
        if current_parent_process_id() != Some(parent_process_id)
            || !process_exists(parent_process_id)
        {
            tracing::warn!(parent_process_id, "mcp parent process disappeared; exiting",);
            std::process::exit(0);
        }
        thread::sleep(Duration::from_millis(PARENT_MONITOR_INTERVAL_MS));
    }
}

/// Returns the current parent process id when it is monitorable.
#[cfg(unix)]
fn current_parent_process_id() -> Option<u32> {
    let raw = nix::unistd::getppid().as_raw();
    u32::try_from(raw).ok().filter(|pid| *pid > 1)
}

/// Returns whether `process_id` currently resolves to a live process.
#[cfg(unix)]
fn process_exists(process_id: u32) -> bool {
    let Ok(pid_raw) = i32::try_from(process_id) else {
        return false;
    };
    match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid_raw), None) {
        Err(nix::errno::Errno::ESRCH) => false,
        Ok(()) | Err(_) => true,
    }
}

/// Installs `tracing_subscriber` against stderr so log lines never
/// leak into the stdio JSON-RPC channel.
fn install_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("deslop_mcp=info,deslop_core=info"));
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .try_init();
}
