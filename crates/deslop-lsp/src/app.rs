//! Testable application layer for the `deslop-lsp` binary.
//!
//! The binary entry point is intentionally thin: collect process
//! handles and delegate here. Argument interpretation, version output,
//! runtime sizing, and startup dispatch live in this module so tests can
//! exercise the highest-level behavior without spawning a process.

use std::{io::Write, path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Result};
use deslop_core::{version_contract_output, ComponentKind};
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

/// Runs parsed startup using an injected async server function.
///
/// # Errors
///
/// Returns Tokio runtime construction or injected server errors.
pub fn run_startup_with<F, Fut>(startup: LspStartup, server: F) -> Result<()>
where
    F: FnOnce(PathBuf, u32, LspEmbeddingConfig) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    init_tracing();
    log_startup(&startup);
    build_runtime(startup.worker_threads)?.block_on(server(
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
    reject_unsupported_startup_flags(args)?;
    Ok(LspStartup {
        workspace_root: parse_workspace_root(args)?,
        min_nodes: 30,
        worker_threads: parse_worker_threads(args)?,
        embedding: LspEmbeddingConfig::default(),
    })
}

/// Reads the workspace root from the first positional argument.
fn parse_workspace_root(args: &[String]) -> Result<PathBuf> {
    args.get(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("usage: deslop-lsp <workspace-root>"))
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

/// Rejects removed startup flags before they can silently change state.
fn reject_unsupported_startup_flags(args: &[String]) -> Result<()> {
    let mut index = 2;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--worker-threads" => index = index.saturating_add(2),
            "--debug" | "--stdio" => index = index.saturating_add(1),
            flag if LEGACY_STARTUP_FLAGS.contains(&flag) => {
                return Err(anyhow!(
                    "unsupported LSP startup flag {flag}; configure Deslop through settings"
                ));
            }
            flag if flag.starts_with('-') => {
                return Err(anyhow!("unsupported LSP startup flag {flag}"));
            }
            _ => index = index.saturating_add(1),
        }
    }
    Ok(())
}

/// Startup flags removed from the LSP command line by issue #83.
const LEGACY_STARTUP_FLAGS: &[&str] = &[
    "--min-nodes",
    "--embeddings",
    "--embedding-provider",
    "--embedding-model",
    "--embedding-endpoint",
];

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
