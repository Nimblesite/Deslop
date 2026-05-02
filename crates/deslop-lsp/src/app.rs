//! Testable application layer for the `deslop-lsp` binary.
//!
//! The binary entry point is intentionally thin: collect process
//! handles and delegate here. Argument interpretation, version output,
//! runtime sizing, and startup dispatch live in this module so tests can
//! exercise the highest-level behavior without spawning a process.

use std::{io::Write, path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Result};
use deslop_core::{
    embedding::{DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID},
    version_contract_output, ComponentKind,
};
use tokio::runtime::{Builder, Runtime};
use tracing_subscriber::EnvFilter;

use crate::backend::LspEmbeddingConfig;

/// Fully parsed startup configuration for the LSP app layer.
#[derive(Debug, Clone)]
pub struct LspStartup {
    /// Workspace root passed as the first positional argument.
    pub workspace_root: PathBuf,
    /// Minimum syntax-node count used by duplicate detection.
    pub min_nodes: u32,
    /// Tokio worker-thread cap. Zero means Tokio default.
    pub worker_threads: usize,
    /// Embedding startup configuration.
    pub embedding: LspEmbeddingConfig,
}

/// The top-level action requested by the user-facing argv.
#[derive(Debug, Clone)]
pub enum LspAction {
    /// Print a version-contract payload and exit successfully.
    Version {
        /// Exact bytes to write to stdout.
        output: String,
    },
    /// Start the LSP server with parsed configuration.
    Serve(LspStartup),
}

/// Interprets argv into the top-level LSP action.
///
/// # Errors
///
/// Returns parse errors for invalid flags or missing workspace root.
pub fn action_from_args<I, S>(args: I) -> Result<LspAction>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = collect_args(args);
    if let Some(output) = version_contract(&args)? {
        return Ok(LspAction::Version { output });
    }
    Ok(LspAction::Serve(startup_from_args(&args)?))
}

/// Runs argv through the app layer using an injected server runner.
///
/// # Errors
///
/// Returns argument, stdout, runtime, or server startup errors.
pub fn run_process_result<I, S, W, R>(args: I, mut stdout: W, runner: R) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
    R: FnOnce(LspStartup) -> Result<()>,
{
    match action_from_args(args)? {
        LspAction::Version { output } => write_version(&mut stdout, &output),
        LspAction::Serve(startup) => runner(startup),
    }
}

/// Runs the app layer and converts failures into process exit codes.
pub fn run_process<I, S, W, R>(args: I, stdout: W, runner: R) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
    R: FnOnce(LspStartup) -> Result<()>,
{
    match run_process_result(args, stdout, runner) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failure_exit(&error),
    }
}

/// Starts the real stdio LSP server for parsed app configuration.
///
/// # Errors
///
/// Returns Tokio runtime construction or LSP server startup errors.
pub fn run_stdio_process(startup: LspStartup) -> Result<()> {
    init_tracing();
    log_startup(&startup);
    build_runtime(startup.worker_threads)?.block_on(crate::run_stdio(
        startup.workspace_root,
        startup.min_nodes,
        startup.embedding,
    ))
}

/// Collects argv into owned strings so all downstream parsing borrows one slice.
fn collect_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    args.into_iter().map(Into::into).collect()
}

/// Returns the version-contract output when argv requested it.
fn version_contract(args: &[String]) -> Result<Option<String>> {
    if let Some(output) = version_contract_output(args, "deslop-lsp", ComponentKind::Lsp)? {
        return Ok(Some(output));
    }
    Ok(requests_version(args).then(String::new))
}

/// Returns whether argv requests version output.
fn requests_version(args: &[String]) -> bool {
    args.iter()
        .skip(1)
        .any(|arg| matches!(arg.as_str(), "--version" | "-V"))
}

/// Parses non-version argv into server startup configuration.
fn startup_from_args(args: &[String]) -> Result<LspStartup> {
    Ok(LspStartup {
        workspace_root: parse_workspace_root(args)?,
        min_nodes: parse_min_nodes(args)?,
        worker_threads: parse_worker_threads(args)?,
        embedding: parse_embedding_config(args)?,
    })
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
            return parse_required_u32(args, index, "--min-nodes");
        }
    }
    Ok(30)
}

/// Reads the optional `--worker-threads` value, defaulting to Tokio behavior.
fn parse_worker_threads(args: &[String]) -> Result<usize> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--worker-threads" {
            return parse_required_usize(args, index, "--worker-threads");
        }
    }
    Ok(0)
}

/// Parses a required unsigned 32-bit flag value after `flag`.
fn parse_required_u32(args: &[String], index: usize, flag: &str) -> Result<u32> {
    Ok(required_flag_value(args, index, flag)?.parse::<u32>()?)
}

/// Parses a required usize flag value after `flag`.
fn parse_required_usize(args: &[String], index: usize, flag: &str) -> Result<usize> {
    Ok(required_flag_value(args, index, flag)?.parse::<usize>()?)
}

/// Returns the string value immediately following a required flag.
fn required_flag_value<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str> {
    args.get(index.saturating_add(1))
        .map(String::as_str)
        .ok_or_else(|| anyhow!("{flag} requires a value"))
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
    args.windows(2).find_map(|pair| match pair {
        [candidate, value] if candidate == flag => Some(value.as_str()),
        _ => None,
    })
}

/// Builds the Tokio runtime for the app layer.
fn build_runtime(worker_threads: usize) -> Result<Runtime> {
    let mut builder = Builder::new_multi_thread();
    if worker_threads > 0 {
        let _ = builder.worker_threads(worker_threads);
    }
    Ok(builder.enable_all().build()?)
}

/// Initialises tracing diagnostics against `RUST_LOG`.
fn init_tracing() {
    let _result = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();
}

/// Writes exact version-contract bytes to stdout.
fn write_version<W: Write>(stdout: &mut W, output: &str) -> Result<()> {
    stdout.write_all(output.as_bytes())?;
    Ok(())
}

/// Logs parsed startup settings before the server begins serving.
fn log_startup(startup: &LspStartup) {
    tracing::info!(
        workspace_root = %startup.workspace_root.display(),
        min_nodes = startup.min_nodes,
        worker_threads = startup.worker_threads,
        embedding_mode = startup.embedding.mode.as_str(),
        embedding_provider = %startup.embedding.provider_id,
        embedding_model = %startup.embedding.model_id,
        "deslop-lsp args parsed",
    );
}

/// Logs a failure and returns a non-zero process exit code.
fn failure_exit(error: &anyhow::Error) -> ExitCode {
    tracing::error!(%error, "deslop-lsp exited with error");
    ExitCode::from(1)
}
