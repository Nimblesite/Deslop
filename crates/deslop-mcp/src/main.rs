//! `deslop-mcp` binary — MCP server over stdio.
//!
//! Thin shell over [`deslop_mcp::McpServer`]: parse CLI args,
//! configure tracing, construct the [`StateFileBackend`], and
//! drive the server against stdin / stdout.

use std::{env, io, path::PathBuf, sync::Arc};

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
