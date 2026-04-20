//! `CodeDedup` CLI binary.
//!
//! Thin shell over `codededup-core`. Parses args, initialises tracing
//! (to a timestamped log file by default; see [`logging`]), prints a
//! human-readable preamble + summary on stderr (see [`summary`]),
//! and either runs the pipeline or re-renders an existing JSON report
//! (`--from-report`). Always emits the canonical JSON plus derived
//! text and HTML views unless suppressed ([OUTPUT-SCHEMA-JSON]).

mod logging;
mod summary;

use std::{env, fs, io::Write as _, path::PathBuf, str::FromStr};

use anyhow::{bail, Context, Result};
use clap::Parser;
use codededup_core::{
    render::render_html, render::render_text, run, EmbeddingMode, EmbeddingSettings,
    OllamaProvider, PipelineConfig, Report, StubProvider, DEFAULT_OLLAMA_ENDPOINT,
    DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
};
use tracing::Level;

use crate::{
    logging::LogSink,
    summary::{ColorChoice, PreambleKnobs, WrittenArtefacts},
};

/// Default base name for the three-format output written to CWD when
/// `--output` is not provided.
const DEFAULT_OUTPUT_STEM: &str = "codededup-report";

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

    /// Base path for the rendered reports. Extensions `.json`, `.txt`,
    /// `.html` are appended. Defaults to `codededup-report` in the
    /// current working directory.
    #[arg(long, value_name = "PATH_PREFIX")]
    output: Option<PathBuf>,

    /// Skip analysis and re-render the canonical JSON report at this
    /// path into `.txt` and `.html` (and, unless `--nojson` is set,
    /// copy the JSON itself).
    #[arg(long, value_name = "FILE")]
    from_report: Option<PathBuf>,

    /// Path to an explicit `.codededup.toml` exclusion config. Defaults
    /// to `.codededup.toml` next to the scan root.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Output-format suppression flags (`--nojson`, `--notext`,
    /// `--nohtml`).
    #[command(flatten)]
    suppress: SuppressFlags,

    /// Embedding-layer policy. `auto` probes the provider and falls
    /// back with a warning; `required` hard-fails when the provider
    /// is unreachable; `off` skips embeddings entirely.
    #[arg(long, value_name = "MODE", default_value = "off")]
    embeddings: String,

    /// Embedding provider registry key. Only `ollama` is implemented
    /// today; future providers slot in behind the same flag.
    #[arg(long, value_name = "ID", default_value = DEFAULT_PROVIDER_ID)]
    embedding_provider: String,

    /// Embedding model identifier as understood by the provider. For
    /// `ollama`, this is the model name shown by `ollama list`.
    #[arg(long, value_name = "MODEL", default_value = DEFAULT_OLLAMA_MODEL)]
    embedding_model: String,

    /// Embedding provider endpoint. Defaults to the Ollama loopback
    /// URL.
    #[arg(long, value_name = "URL", default_value = DEFAULT_OLLAMA_ENDPOINT)]
    embedding_endpoint: String,

    /// Runtime-behaviour flags (`--no-incremental`, `--log-*`,
    /// `--no-color`).
    #[command(flatten)]
    behaviour: BehaviourFlags,
}

/// Suppression flags for each output format. Packed into their own
/// struct so the top-level `Cli` stays under the `pedantic`
/// three-bool ceiling.
#[derive(Debug, clap::Args)]
struct SuppressFlags {
    /// Suppress the canonical JSON output.
    #[arg(long)]
    nojson: bool,
    /// Suppress the terse AI-readable text output.
    #[arg(long)]
    notext: bool,
    /// Suppress the human-readable HTML output.
    #[arg(long)]
    nohtml: bool,
}

/// Runtime-behaviour flags — caching, logging, colour. Same packing
/// rationale as [`SuppressFlags`].
#[derive(Debug, clap::Args)]
struct BehaviourFlags {
    /// Enable the on-disk fingerprint cache ([PIPELINE-INCREMENTAL]).
    /// When set, the pipeline caches parsed AST + fingerprints under
    /// `<root>/.codededup-cache/fingerprints/...` keyed by
    /// `(language, tool_version, min_nodes, content_hash)`. On the
    /// next run, unchanged files skip tree-sitter entirely. Off by
    /// default — analysing a read-only checkout should not mutate it.
    #[arg(long)]
    incremental: bool,
    /// Send log events to stderr instead of a timestamped file. By
    /// default the CLI writes logs to `codededup-<timestamp>.log`
    /// next to the report so the stderr stream stays readable.
    #[arg(long)]
    log_to_console: bool,
    /// Minimum log severity emitted. Accepts `error`, `warn`, `info`,
    /// `debug`, `trace`. Overridden by `RUST_LOG` when set.
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    log_level: String,
    /// Disable colour in the stderr preamble / summary. Colour is
    /// also suppressed automatically when stderr is not a TTY or the
    /// `NO_COLOR` environment variable is set.
    #[arg(long)]
    no_color: bool,
}

fn main() {
    match run_cli() {
        Ok(()) => {}
        Err(err) => {
            // `run_cli` already printed the colored failure footer
            // (when tracing was up). Fall back to a plain eprintln!
            // so failures before logging initialises still surface.
            eprintln!("codededup: {err:#}");
            std::process::exit(1);
        }
    }
}

/// Top-level flow. Kept separate from `main` so the error path can
/// be wrapped with a non-zero exit code without losing the
/// `anyhow::Error` chain.
fn run_cli() -> Result<()> {
    let args = Cli::parse();
    let formats = FormatSelection::from_args(&args)?;
    let output = OutputPaths::new(args.output.as_deref());
    let mode: EmbeddingMode = parse_embedding_mode(&args.embeddings)?;
    let log_level = parse_log_level(&args.behaviour.log_level)?;
    let color = ColorChoice::resolve(args.behaviour.no_color);
    let log_sink = logging::init(output.directory(), args.behaviour.log_to_console, log_level)?;
    summary::preamble(
        color,
        &args.path,
        &output.base,
        &log_sink,
        &PreambleKnobs {
            min_nodes: args.min_nodes,
            embedding_mode: mode.as_str(),
            incremental: args.behaviour.incremental,
        },
    );
    tracing::info!(
        path = %args.path.display(),
        min_nodes = args.min_nodes,
        json = formats.json,
        text = formats.text,
        html = formats.html,
        embeddings = mode.as_str(),
        incremental = args.behaviour.incremental,
        "codededup invoked",
    );
    let report = match produce_report(&args, mode, &formats) {
        Ok(report) => report,
        Err(err) => {
            summary::finish_err(color, &log_sink, &err);
            return Err(err);
        }
    };
    let written = emit_all(&report, &formats, &output)?;
    summary::summary(color, &report);
    summary::finish_ok(
        color,
        &WrittenArtefacts {
            reports: &written,
            log: match &log_sink {
                LogSink::File(path) => Some(path.as_path()),
                LogSink::Console => None,
            },
        },
    );
    Ok(())
}

/// Either loads a cached report (`--from-report`) or runs the
/// pipeline end-to-end.
fn produce_report(args: &Cli, mode: EmbeddingMode, _formats: &FormatSelection) -> Result<Report> {
    if let Some(source) = &args.from_report {
        return load_report(source);
    }
    let provider = configured_provider(args, mode)?;
    let provider_ref: Option<&dyn codededup_core::EmbeddingProvider> = provider.as_deref();
    let pipeline_config = PipelineConfig {
        root: args.path.clone(),
        min_nodes: args.min_nodes,
        config_path: args.config.clone(),
        embedding: EmbeddingSettings {
            mode,
            provider: provider_ref,
        },
        incremental: args.behaviour.incremental,
    };
    run(&pipeline_config).context("analysis pipeline failed")
}

/// Parses `--embeddings` into the core enum, surfacing a user-facing
/// error message when the value is not one of the three accepted
/// variants.
fn parse_embedding_mode(source: &str) -> Result<EmbeddingMode> {
    source
        .parse::<EmbeddingMode>()
        .map_err(|err| anyhow::anyhow!("invalid --embeddings value {:?}: {err}", err.value))
}

/// Parses `--log-level` into `tracing::Level`.
fn parse_log_level(source: &str) -> Result<Level> {
    Level::from_str(source).map_err(|err| anyhow::anyhow!("invalid --log-level {source:?}: {err}"))
}

/// Instantiates the embedding provider. Returns `None` for
/// [`EmbeddingMode::Off`] so the pipeline never tries to reach out,
/// or when the provider id is not recognised under `auto` mode.
fn configured_provider(
    args: &Cli,
    mode: EmbeddingMode,
) -> Result<Option<Box<dyn codededup_core::EmbeddingProvider>>> {
    if matches!(mode, EmbeddingMode::Off) {
        return Ok(None);
    }
    match args.embedding_provider.as_str() {
        DEFAULT_PROVIDER_ID => build_ollama_provider(args, mode),
        STUB_PROVIDER_ID => Ok(Some(Box::new(StubProvider::new()))),
        other => bail!("unknown embedding provider {other:?}"),
    }
}

/// Builds the Ollama provider. Under `auto`, a connection failure
/// downgrades to `None` with a warning so the pipeline can fall back.
/// Under `required`, it propagates as an error.
fn build_ollama_provider(
    args: &Cli,
    mode: EmbeddingMode,
) -> Result<Option<Box<dyn codededup_core::EmbeddingProvider>>> {
    match OllamaProvider::connect(&args.embedding_endpoint, &args.embedding_model) {
        Ok(provider) => Ok(Some(Box::new(provider))),
        Err(source) => {
            if matches!(mode, EmbeddingMode::Required) {
                Err(anyhow::anyhow!(
                    "embedding provider required but unreachable: {source}"
                ))
            } else {
                tracing::warn!(%source, "embedding provider unreachable — continuing without Type-4 recall");
                Ok(None)
            }
        }
    }
}

/// Which of the three output formats are enabled for this run.
#[derive(Debug)]
struct FormatSelection {
    /// Emit canonical JSON (`<base>.json`).
    json: bool,
    /// Emit terse text view (`<base>.txt`).
    text: bool,
    /// Emit human-readable HTML view (`<base>.html`).
    html: bool,
}

impl FormatSelection {
    /// Builds the selection from the three suppression flags. Errors
    /// out when all three are suppressed — silent runs are never
    /// helpful.
    fn from_args(args: &Cli) -> Result<Self> {
        let selection = Self {
            json: !args.suppress.nojson,
            text: !args.suppress.notext,
            html: !args.suppress.nohtml,
        };
        if !selection.json && !selection.text && !selection.html {
            bail!("at least one of --nojson/--notext/--nohtml must remain enabled");
        }
        Ok(selection)
    }
}

/// Resolved output base path; renderers append their own extension.
#[derive(Debug)]
struct OutputPaths {
    /// `<base>` such that `<base>.json` etc. are the final paths.
    base: PathBuf,
}

impl OutputPaths {
    /// Picks the base path. When the user passed `--output`, use it
    /// verbatim; otherwise write into the current working directory
    /// under [`DEFAULT_OUTPUT_STEM`].
    fn new(explicit: Option<&std::path::Path>) -> Self {
        let base = explicit.map_or_else(
            || {
                env::current_dir()
                    .unwrap_or_default()
                    .join(DEFAULT_OUTPUT_STEM)
            },
            std::path::Path::to_path_buf,
        );
        Self { base }
    }

    /// Returns the concrete on-disk path for a given extension.
    fn with_extension(&self, extension: &str) -> PathBuf {
        let mut path = self.base.clone();
        let stem = path
            .file_name()
            .map(std::ffi::OsStr::to_os_string)
            .unwrap_or_default();
        let mut new_name = stem;
        new_name.push(".");
        new_name.push(extension);
        path.set_file_name(new_name);
        path
    }

    /// Directory that the report files sit in. Used as the default
    /// location for the timestamped log file so all per-run output
    /// lives together.
    fn directory(&self) -> &std::path::Path {
        self.base.parent().unwrap_or(std::path::Path::new("."))
    }
}

/// Writes every enabled format for `report` to its derived path under
/// `output`, returning the paths actually written (for the finish
/// footer).
fn emit_all(
    report: &Report,
    formats: &FormatSelection,
    output: &OutputPaths,
) -> Result<Vec<PathBuf>> {
    let mut written: Vec<PathBuf> = Vec::with_capacity(3);
    if formats.json {
        let json = serde_json::to_string_pretty(report).context("serialise report as JSON")?;
        let path = output.with_extension("json");
        write_file(&path, json.as_bytes())?;
        written.push(path);
    }
    if formats.text {
        let text = render_text(report);
        let path = output.with_extension("txt");
        write_file(&path, text.as_bytes())?;
        written.push(path);
    }
    if formats.html {
        let html = render_html(report);
        let path = output.with_extension("html");
        write_file(&path, html.as_bytes())?;
        written.push(path);
    }
    Ok(written)
}

/// Writes `payload` to `path`, creating parent directories as needed.
fn write_file(path: &std::path::Path, payload: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent {}", parent.display()))?;
        }
    }
    let mut file =
        fs::File::create(path).with_context(|| format!("create report file {}", path.display()))?;
    file.write_all(payload)
        .with_context(|| format!("write report file {}", path.display()))?;
    tracing::info!(path = %path.display(), bytes = payload.len(), "wrote report file");
    Ok(())
}

/// Loads a canonical JSON report from disk for `--from-report`.
fn load_report(path: &std::path::Path) -> Result<Report> {
    let source =
        fs::read_to_string(path).with_context(|| format!("read report {}", path.display()))?;
    serde_json::from_str::<Report>(&source)
        .with_context(|| format!("parse report {}", path.display()))
}
