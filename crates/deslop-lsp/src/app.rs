//! Testable application layer for the `deslop-lsp` binary.
//!
//! The binary entry point is intentionally thin: collect process
//! handles and delegate here. Argument interpretation, version output,
//! runtime sizing, and startup dispatch live in this module so tests can
//! exercise the highest-level behavior without spawning a process.

use std::{io::Write, path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Result};
use deslop_core::{requests_version, version_contract_output, ComponentKind};
use tokio::runtime::{Builder, Runtime};
use tracing_subscriber::EnvFilter;

use deslop_core::{config::ClonePolicy, live::transport::IpcMode};

use crate::backend::LspEmbeddingConfig;

/// Startup flag setting the analysis worker-thread count.
const WORKER_THREADS_FLAG: &str = "--worker-threads";
/// Startup flag lowering the analysis threads' scheduling priority.
const NICE_FLAG: &str = "--nice";
/// Startup flag choosing the IPC transport (stdio or TCP).
const IPC_TRANSPORT_FLAG: &str = "--ipc-transport";
/// Startup flag restricting ranking to structural evidence.
const RANKING_STRUCTURAL_ONLY_FLAG: &str = "--ranking-structural-only";

/// Fully parsed startup configuration for the LSP app layer.
#[derive(Debug, Clone)]
pub struct LspStartup {
    /// Workspace root passed as the first positional argument.
    pub workspace_root: PathBuf,
    /// Minimum syntax-node count used by duplicate detection.
    pub min_nodes: u32,
    /// Tokio worker-thread cap. Zero means Tokio default.
    pub worker_threads: usize,
    /// Unix nice value applied at startup. Zero leaves priority unchanged.
    pub nice: i32,
    /// Embedding startup configuration.
    pub embedding: LspEmbeddingConfig,
    /// IPC transport for the MCP bridge ([LIVE-IPC-TCP]). Platform
    /// default unless `--ipc-transport` overrides it.
    pub ipc_mode: IpcMode,
    /// Optional [RANK-STRUCTURAL-ONLY] policy override from
    /// `--ranking-structural-only`, fed by the
    /// `deslop.ranking.structuralOnly` editor setting
    /// ([VSIX-SETTINGS-RANKING]). `None` defers to `.deslop.toml`.
    pub ranking_structural_only: Option<ClonePolicy>,
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
pub fn run_process_result<I, S, W, R>(args: I, mut stdout: W, runner: R) -> Result<ExitCode>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
    R: FnOnce(LspStartup) -> Result<ExitCode>,
{
    match action_from_args(args)? {
        LspAction::Version { output } => {
            write_version(&mut stdout, &output).map(|()| ExitCode::SUCCESS)
        }
        LspAction::Serve(startup) => runner(startup),
    }
}

/// Runs the app layer and converts failures into process exit codes.
pub fn run_process<I, S, W, R>(args: I, stdout: W, runner: R) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    W: Write,
    R: FnOnce(LspStartup) -> Result<ExitCode>,
{
    // Install the subscriber at the process boundary so diagnostics reach
    // stderr on *every* path — including argv parse errors that never reach
    // the serve path (e.g. a rootless `deslop-lsp --stdio`). Without
    // this, `failure_exit`'s `tracing::error!` hits `NoSubscriber` and the
    // process exits 1 silently. `try_init` makes it idempotent.
    init_tracing();
    match run_process_result(args, stdout, runner) {
        Ok(code) => code,
        Err(error) => failure_exit(&error),
    }
}

/// Runs parsed startup using an injected async server function.
///
/// # Errors
///
/// Returns Tokio runtime construction or injected server errors.
pub fn run_startup_with<F, Fut, T>(startup: LspStartup, server: F) -> Result<T>
where
    F: FnOnce(PathBuf, u32, LspEmbeddingConfig, IpcMode) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let _profile_guard = crate::profiling::LspProfileGuard::from_env();
    #[cfg(unix)]
    apply_process_nice(startup.nice)?;
    #[cfg(not(unix))]
    apply_process_nice(startup.nice);
    if let Some(policy) = startup.ranking_structural_only {
        deslop_core::state::set_structural_only_override(policy);
    }
    log_startup(&startup);
    let runtime = build_runtime(startup.worker_threads)?;
    let outcome = runtime.block_on(server(
        startup.workspace_root,
        startup.min_nodes,
        startup.embedding,
        startup.ipc_mode,
    ));
    // The stdio reader is a `spawn_blocking` task parked inside a read that
    // only returns at EOF, and dropping a runtime joins its blocking tasks.
    // A client that says `exit` never closes stdin, so waiting here wedges
    // the process in `Runtime::drop` forever — one abandoned analyser per
    // editor window ever opened ([LSP-LIFECYCLE]). The serve loop has
    // already finished by this point, so there is nothing left to wait for.
    runtime.shutdown_background();
    outcome
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

/// Parses non-version argv into server startup configuration.
fn startup_from_args(args: &[String]) -> Result<LspStartup> {
    reject_unsupported_startup_flags(args)?;
    Ok(LspStartup {
        workspace_root: parse_workspace_root(args)?,
        min_nodes: 30,
        worker_threads: parse_worker_threads(args)?,
        nice: parse_nice(args)?,
        embedding: LspEmbeddingConfig::default(),
        ipc_mode: parse_ipc_mode(args)?,
        ranking_structural_only: parse_ranking_structural_only(args)?,
    })
}

/// Reads the workspace root from the first positional (non-flag) argument.
///
/// The client's transport flag (`--stdio`, injected by
/// `vscode-languageclient`) and `--debug` are not paths. Treating one as
/// the root pointed the file watcher at a path named `--stdio` and
/// crash-looped the server. The workspace root is the first argument that
/// does not begin with `-` (absolute paths start with `/`, a drive
/// letter, or a UNC prefix — never `-`); when the whole argv is flags
/// (no folder open) that is a usage error, not a root. Scanning for the
/// first non-flag argument — rather than trusting `args[1]` — keeps the
/// guard correct no matter where the client injects the transport flag,
/// so a future library that prepends it cannot reopen the crash-loop.
fn parse_workspace_root(args: &[String]) -> Result<PathBuf> {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("usage: deslop-lsp <workspace-root>"))
}

/// Reads the optional `--worker-threads` value, defaulting to Tokio behavior.
fn parse_worker_threads(args: &[String]) -> Result<usize> {
    for (index, arg) in args.iter().enumerate() {
        if arg == WORKER_THREADS_FLAG {
            return parse_required_usize(args, index, WORKER_THREADS_FLAG);
        }
    }
    Ok(0)
}

/// Reads the optional `--nice` value, defaulting to no priority change.
fn parse_nice(args: &[String]) -> Result<i32> {
    for (index, arg) in args.iter().enumerate() {
        if arg == NICE_FLAG {
            let nice = parse_required_i32(args, index, NICE_FLAG)?;
            if !(-20..=19).contains(&nice) {
                return Err(anyhow!("--nice must be in the range -20..=19"));
            }
            return Ok(nice);
        }
    }
    Ok(0)
}

/// Reads the optional `--ipc-transport` value ([LIVE-IPC-TCP]),
/// defaulting to the platform transport (Unix socket on Unix, TCP
/// loopback on Windows).
fn parse_ipc_mode(args: &[String]) -> Result<IpcMode> {
    for (index, arg) in args.iter().enumerate() {
        if arg == IPC_TRANSPORT_FLAG {
            return match required_flag_value(args, index, IPC_TRANSPORT_FLAG)? {
                "unix" => Ok(IpcMode::Unix),
                "tcp" => Ok(IpcMode::Tcp),
                other => Err(anyhow!(
                    "--ipc-transport must be `unix` or `tcp`, got {other:?}"
                )),
            };
        }
    }
    Ok(IpcMode::platform_default())
}

/// Reads the optional `--ranking-structural-only` value
/// ([RANK-STRUCTURAL-ONLY], [VSIX-SETTINGS-RANKING]). `None` defers to
/// `.deslop.toml`.
fn parse_ranking_structural_only(args: &[String]) -> Result<Option<ClonePolicy>> {
    for (index, arg) in args.iter().enumerate() {
        if arg == RANKING_STRUCTURAL_ONLY_FLAG {
            let value = required_flag_value(args, index, RANKING_STRUCTURAL_ONLY_FLAG)?;
            return value
                .parse::<ClonePolicy>()
                .map(Some)
                .map_err(|message| anyhow!("--ranking-structural-only: {message}"));
        }
    }
    Ok(None)
}

/// Parses a required usize flag value after `flag`.
fn parse_required_usize(args: &[String], index: usize, flag: &str) -> Result<usize> {
    Ok(required_flag_value(args, index, flag)?.parse::<usize>()?)
}

/// Parses a required i32 flag value after `flag`.
fn parse_required_i32(args: &[String], index: usize, flag: &str) -> Result<i32> {
    Ok(required_flag_value(args, index, flag)?.parse::<i32>()?)
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
            WORKER_THREADS_FLAG | NICE_FLAG | IPC_TRANSPORT_FLAG | RANKING_STRUCTURAL_ONLY_FLAG => {
                index = index.saturating_add(2);
            }
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

/// Startup flags removed from the LSP command line by.
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

/// Applies the user-requested process nice value when configured.
#[cfg(unix)]
fn apply_process_nice(nice: i32) -> Result<()> {
    if nice == 0 {
        return Ok(());
    }
    apply_process_nice_impl(nice)
}

/// Applies Unix process priority through `setpriority(PRIO_PROCESS)`.
#[cfg(unix)]
fn apply_process_nice_impl(nice: i32) -> Result<()> {
    rustix::process::setpriority_process(None, nice)
        .map_err(|error| anyhow!("failed to apply --nice {nice}: {error}"))?;
    Ok(())
}

/// Ignores `--nice` on targets where POSIX priorities do not exist.
#[cfg(not(unix))]
fn apply_process_nice(nice: i32) {
    if nice != 0 {
        tracing::warn!(nice, "--nice is only supported on macOS/Linux");
    }
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
        nice = startup.nice,
        ipc_transport = ?startup.ipc_mode,
        ranking_structural_only = ?startup.ranking_structural_only,
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
