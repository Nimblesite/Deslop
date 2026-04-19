//! `CodeDedup` CLI binary.
//!
//! Thin shell over `codededup-core`. Parses args, initialises tracing, and
//! dispatches to the library. A future MCP/LSP daemon will be a sibling
//! binary over the same crate.

use std::{fs, io::Write, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use codededup_core::{run, PipelineConfig, Report};
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
    path: PathBuf,

    /// Minimum AST subtree node count to consider a clone candidate.
    #[arg(long, default_value_t = 30)]
    min_nodes: u32,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Write the report to this path instead of stdout.
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,
}

/// Report format selector.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    /// Human-readable text. Pretty-printer over the JSON schema.
    Text,
    /// Canonical JSON schema, stable across releases.
    Json,
}

fn main() -> Result<()> {
    init_tracing()?;
    let args = Cli::parse();
    tracing::info!(
        path = %args.path.display(),
        min_nodes = args.min_nodes,
        format = ?args.format,
        "codededup invoked",
    );
    let report = run(&PipelineConfig {
        root: args.path.clone(),
        min_nodes: args.min_nodes,
    })
    .context("analysis pipeline failed")?;
    emit_report(&report, args.format, args.output.as_deref())?;
    Ok(())
}

/// Serialises `report` in the requested format and writes it to either
/// `destination` (when provided) or stdout.
fn emit_report(
    report: &Report,
    format: OutputFormat,
    destination: Option<&std::path::Path>,
) -> Result<()> {
    let payload = match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(report).context("serialise report as JSON")?
        }
        OutputFormat::Text => render_text(report),
    };
    match destination {
        Some(path) => fs::write(path, &payload)
            .with_context(|| format!("write report to {}", path.display())),
        None => {
            let stdout = std::io::stdout();
            let mut guard = stdout.lock();
            guard
                .write_all(payload.as_bytes())
                .context("write report to stdout")?;
            guard
                .write_all(b"\n")
                .context("write trailing newline")?;
            Ok(())
        }
    }
}

/// ASCII-only pretty-printer over the report. See
/// [PRINCIPLES-AUDIENCE-AGENT].
fn render_text(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "codededup {tool} (schema v{schema}) — {files} file(s), {clusters} cluster(s)\n",
        tool = report.tool_version,
        schema = report.report_schema_version,
        files = report.files_analysed,
        clusters = report.clusters.len(),
    ));
    for (idx, cluster) in report.clusters.iter().enumerate() {
        out.push_str(&format!(
            "#{rank} [{id}] weight={weight:.2} size={size} nodes={nodes}\n  {summary}\n",
            rank = idx.saturating_add(1),
            id = cluster.id,
            weight = cluster.weight,
            size = cluster.size,
            nodes = cluster.canonical_node_count,
            summary = cluster.summary,
        ));
    }
    out
}

/// Configures the global `tracing` subscriber. Honours `RUST_LOG` when set
/// and defaults to `info`-level events otherwise. Writes to stderr so that
/// stdout stays reserved for the report stream.
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
