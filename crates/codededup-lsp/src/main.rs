//! Binary entry point for the `CodeDedup` LSP server ([LSP-TRANSPORT]).
//!
//! Bootstrapping only — every protocol concern lives in
//! [`codededup_lsp::backend`] and friends. Argument shape:
//! `codededup-lsp <workspace-root> [--min-nodes N]`.

use std::{env, path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    if let Err(error) = run().await {
        tracing::error!(%error, "codededup-lsp exited with error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Parses CLI arguments and starts the server.
async fn run() -> Result<()> {
    init_tracing();
    let args: Vec<String> = env::args().collect();
    tracing::info!(argv = ?args, "codededup-lsp starting");
    let workspace_root = parse_workspace_root(&args)?;
    let min_nodes = parse_min_nodes(&args)?;
    tracing::info!(
        workspace_root = %workspace_root.display(),
        min_nodes,
        "codededup-lsp args parsed",
    );
    codededup_lsp::run_stdio(workspace_root, min_nodes).await
}

/// Initialises `tracing-subscriber` against the `RUST_LOG`
/// environment variable. Logs go to stderr per [LSP-TRANSPORT].
fn init_tracing() {
    let _result = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();
}

/// Reads the workspace root from the first positional argument.
fn parse_workspace_root(args: &[String]) -> Result<PathBuf> {
    args.get(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("usage: codededup-lsp <workspace-root> [--min-nodes N]"))
}

/// Reads the optional `--min-nodes` value, defaulting to 30.
fn parse_min_nodes(args: &[String]) -> Result<u32> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--min-nodes" {
            return parse_min_nodes_value(args, index);
        }
    }
    Ok(30)
}

/// Reads the value following `--min-nodes`.
fn parse_min_nodes_value(args: &[String], index: usize) -> Result<u32> {
    let next_index = index.saturating_add(1);
    let value = args
        .get(next_index)
        .ok_or_else(|| anyhow!("--min-nodes requires a value"))?;
    Ok(value.parse::<u32>()?)
}
