//! `CodeDedup` CLI binary.
//!
//! Thin shell over `codededup-core`. Parses args, initialises tracing, and
//! dispatches to the library. A future MCP/LSP daemon will be a sibling
//! binary over the same crate.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

/// Command-line interface for `CodeDedup`.
#[derive(Debug, Parser)]
#[command(
    name = "codededup",
    version,
    about = "Detect duplicated code across a codebase, ordered by worst offenders first."
)]
struct Cli {
    /// Directory to analyse. Defaults to the current working directory.
    #[arg(value_name = "PATH", default_value = ".")]
    path: std::path::PathBuf,

    /// Minimum AST subtree node count to consider a clone candidate.
    #[arg(long, default_value_t = 30)]
    min_nodes: u32,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

/// Report format selector.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    /// Human-readable text. Pretty-printer over the JSON schema.
    Text,
    /// Canonical JSON schema, stable across releases.
    Json,
}

/// Entry point. Parses args, wires tracing, and (once implemented) invokes
/// the analysis pipeline. Returns an [`anyhow::Result`] because every
/// downstream pipeline stage is fallible — the `Result` is load-bearing as
/// soon as the real work lands.
fn main() -> Result<()> {
    init_tracing()?;
    let args = Cli::parse();
    tracing::info!(
        path = %args.path.display(),
        min_nodes = args.min_nodes,
        format = ?args.format,
        "codededup invoked",
    );
    tracing::warn!("analysis pipeline not yet implemented");
    Ok(())
}

/// Configures the global `tracing` subscriber. Honours `RUST_LOG` when set
/// and defaults to `info`-level events otherwise. Writes to stderr so that
/// stdout stays reserved for the (future) report stream.
fn init_tracing() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|source| anyhow::anyhow!("failed to initialise tracing: {source}"))?;
    Ok(())
}
