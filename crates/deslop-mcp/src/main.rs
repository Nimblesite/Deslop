//! `deslop-mcp` binary — MCP server over stdio.
//!
//! Thin shell over [`deslop_mcp::McpServer`]: parse CLI args,
//! configure tracing, construct the [`PipelineSessionBackend`], and
//! drive the server against stdin / stdout.

use std::{io, path::PathBuf, sync::Arc};

use clap::Parser;
use deslop_core::{
    embedding::{DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL},
    EmbeddingMode, DEFAULT_PROVIDER_ID,
};
use deslop_mcp::{McpServer, PipelineSessionBackend, SessionBackendConfig};
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

    /// Minimum AST subtree node count for clustering.
    #[arg(long, default_value_t = 30)]
    min_nodes: u32,

    /// Enable the on-disk fingerprint cache.
    #[arg(long, default_value_t = false)]
    incremental: bool,

    /// Embedding-pass mode: `off`, `auto`, or `required`.
    #[arg(long, default_value = "off")]
    embeddings: String,

    /// Embedding provider id (`stub`, `ollama`).
    #[arg(long, default_value = DEFAULT_PROVIDER_ID)]
    embedding_provider: String,

    /// Embedding model id (meaningful for the `ollama` provider).
    #[arg(long, default_value = DEFAULT_OLLAMA_MODEL)]
    embedding_model: String,

    /// Embedding endpoint override (Ollama only).
    #[arg(long, default_value = DEFAULT_OLLAMA_ENDPOINT)]
    embedding_endpoint: String,

    /// Optional `.codededup.toml` override path.
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run() {
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
fn run() -> Result<(), Box<dyn std::error::Error>> {
    install_tracing();
    let cli = Cli::parse();
    let mode: EmbeddingMode = cli.embeddings.parse()?;
    let config = SessionBackendConfig {
        root: cli.root,
        min_nodes: cli.min_nodes,
        incremental: cli.incremental,
        embedding_mode: mode,
        embedding_provider: cli.embedding_provider,
        embedding_model: cli.embedding_model,
        embedding_endpoint: cli.embedding_endpoint,
        config_path: cli.config,
    };
    let backend = Arc::new(PipelineSessionBackend::initialise(config)?);
    let server = McpServer::new(backend);
    let stdin = io::stdin();
    let stdout = io::stdout();
    server.run(stdin.lock(), stdout.lock())?;
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
