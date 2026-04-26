//! Binary entry point for the `Deslop` LSP server ([LSP-TRANSPORT]).
//!
//! Bootstrapping only — every protocol concern lives in
//! [`deslop_lsp::backend`] and friends. Argument shape:
//! `deslop-lsp <workspace-root> [--min-nodes N]`.

use std::{env, path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Result};
use deslop_core::{
    embedding::{DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID},
    version_contract_output, ComponentKind,
};
use deslop_lsp::backend::LspEmbeddingConfig;
use tokio::runtime::{Builder, Runtime};
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if let Err(error) = print_version_contract(&args) {
        tracing::error!(%error, "deslop-lsp version output failed");
        return ExitCode::from(1);
    }
    if requests_version(&args) {
        return ExitCode::SUCCESS;
    }
    if let Err(error) = build_runtime().and_then(|runtime| runtime.block_on(run(args))) {
        tracing::error!(%error, "deslop-lsp exited with error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Parses CLI arguments and starts the server.
async fn run(args: Vec<String>) -> Result<()> {
    init_tracing();
    tracing::info!(argv = ?args, "deslop-lsp starting");
    let workspace_root = parse_workspace_root(&args)?;
    let min_nodes = parse_min_nodes(&args)?;
    let worker_threads = parse_worker_threads(&args)?;
    let embedding = parse_embedding_config(&args)?;
    tracing::info!(
        workspace_root = %workspace_root.display(),
        min_nodes,
        worker_threads,
        embedding_mode = embedding.mode.as_str(),
        embedding_provider = %embedding.provider_id,
        embedding_model = %embedding.model_id,
        "deslop-lsp args parsed",
    );
    deslop_lsp::run_stdio(workspace_root, min_nodes, embedding).await
}

/// Builds the Tokio runtime only after version preflight has returned false.
fn build_runtime() -> Result<Runtime> {
    Ok(Builder::new_multi_thread().enable_all().build()?)
}

/// Prints Deployment Toolkit version output when requested.
fn print_version_contract(args: &[String]) -> Result<()> {
    if let Some(output) = version_contract_output(args, "deslop-lsp", ComponentKind::Lsp)? {
        print!("{output}");
    }
    Ok(())
}

/// Returns whether args request version output.
fn requests_version(args: &[String]) -> bool {
    args.iter()
        .skip(1)
        .any(|arg| matches!(arg.as_str(), "--version" | "-V"))
}

/// Reads the optional `--worker-threads` value, defaulting to 0 which
/// means "use tokio's default (one worker per CPU)". Users who need
/// to background-ise the analyser on large workspaces per issue #28
/// pass a positive integer to cap the worker pool.
fn parse_worker_threads(args: &[String]) -> Result<usize> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--worker-threads" {
            let next_index = index.saturating_add(1);
            let value = args
                .get(next_index)
                .ok_or_else(|| anyhow!("--worker-threads requires a value"))?;
            return Ok(value.parse::<usize>()?);
        }
    }
    Ok(0)
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
        .ok_or_else(|| anyhow!("usage: deslop-lsp <workspace-root> [--min-nodes N]"))
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

/// Parses embedding startup flags into the LSP backend config.
fn parse_embedding_config(args: &[String]) -> Result<LspEmbeddingConfig> {
    let mode = parse_flag_value(args, "--embeddings")
        .unwrap_or("off")
        .parse()?;
    Ok(LspEmbeddingConfig {
        mode,
        provider_id: parse_flag_value(args, "--embedding-provider")
            .unwrap_or(DEFAULT_PROVIDER_ID)
            .to_owned(),
        model_id: parse_flag_value(args, "--embedding-model")
            .unwrap_or(DEFAULT_OLLAMA_MODEL)
            .to_owned(),
        endpoint: parse_flag_value(args, "--embedding-endpoint")
            .unwrap_or(DEFAULT_OLLAMA_ENDPOINT)
            .to_owned(),
    })
}

/// Returns the string value immediately following `flag`.
fn parse_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2).find_map(|pair| {
        if pair.first().is_some_and(|candidate| candidate == flag) {
            pair.get(1).map(String::as_str)
        } else {
            None
        }
    })
}
