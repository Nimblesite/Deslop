//! `deslop-mcp` binary — MCP server over stdio.
//!
//! Thin shell over [`deslop_mcp::McpServer`]: parse CLI args,
//! configure tracing, construct the [`PipelineSessionBackend`], and
//! drive the server against stdin / stdout.

use std::{env, io, path::PathBuf, sync::Arc};

use clap::Parser;
use deslop_core::{
    embedding::{DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL},
    version_contract_output, ComponentKind, EmbeddingMode, DEFAULT_PROVIDER_ID,
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
    let mode: EmbeddingMode = cli.embeddings.parse()?;
    // [#141 MCP-SAFETY] Canonicalise the workspace root immediately so
    // `--root .` (the default) cannot bind a session to whatever
    // directory the agent harness happened to launch the binary
    // from. Surfacing a stable absolute path here means session-config
    // and every report path is anchored to a known location — without
    // it, a client thinks it asked about workspace A while MCP scans
    // workspace B.
    let canonical_root = std::fs::canonicalize(&cli.root).map_err(|err| {
        format!(
            "--root {} could not be canonicalised: {err}",
            cli.root.display()
        )
    })?;
    let config = SessionBackendConfig {
        root: canonical_root,
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
