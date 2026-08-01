//! `Deslop` CLI binary.
//!
//! Thin shell over `deslop-core`. Parses args, initialises tracing
//! (to a timestamped log file by default; see [`logging`]), prints a
//! human-readable preamble + summary on stderr (see [`summary`]),
//! and either runs the pipeline or re-renders an existing JSON report
//! (`--from-report`). Always emits the canonical JSON plus derived
//! text and HTML views unless suppressed ([OUTPUT-SCHEMA-JSON]).

mod logging;
mod output;
mod rerun;
mod summary;

use std::{env, fs, io::Write as _, path::PathBuf, str::FromStr};

use anyhow::{bail, Context, Result};
use clap::Parser;
use deslop_core::{
    debug_ast_dump, validate_threshold_percent, version_contract_output, ComponentKind,
    EmbeddingMode, EmbeddingSettings, ExclusionConfig, OllamaProvider, PipelineSession, Report,
    ReportDelta, ThresholdSource, ThresholdSummary, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL,
    DEFAULT_PROVIDER_ID,
};
use tracing::Level;

use crate::{
    logging::LogSink,
    output::{emit_all, load_report, write_file, FormatSelection, OutputPaths},
    rerun::{assemble_touched, parse_rerun_adds},
    summary::{ColorChoice, PreambleKnobs, WrittenArtefacts},
};

/// Command-line interface for `Deslop`.
#[derive(Debug, Parser)]
#[command(
    name = "deslop",
    version,
    about = "Find, rank, and help fix duplicated code across a codebase — worst offenders first."
)]
struct Cli {
    /// Directory to analyse. Defaults to the current working directory.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Minimum AST subtree node count to consider a clone candidate.
    // [DECISION-MIN-NODES] default 30; subtrees below this floor are
    // excluded from fingerprinting, clustering, and embedding.
    #[arg(long, default_value_t = 30)]
    min_nodes: u32,

    /// Base path for the rendered reports. Extensions `.json`, `.txt`,
    /// `.html` are appended. Defaults to `.deslop/deslop-report` under
    /// the scan root; logs follow the reports into `<dir>/logs/`.
    #[arg(long, value_name = "PATH_PREFIX")]
    output: Option<PathBuf>,

    /// Skip analysis and re-render the canonical JSON report at this
    /// path into `.txt` and `.html` (and, unless `--nojson` is set,
    /// copy the JSON itself).
    #[arg(long, value_name = "FILE")]
    from_report: Option<PathBuf>,

    /// Parse a single source file and print the normalised AST to
    /// stdout, then exit. Developer tool — bypasses the analysis
    /// pipeline, writes nothing to disk, and mutates no caches.
    /// Conflicts with `--from-report`.
    #[arg(long, value_name = "FILE", conflicts_with = "from_report")]
    debug_ast: Option<PathBuf>,

    /// Path to an explicit `.deslop.toml` exclusion config. Defaults
    /// to `.deslop.toml` next to the scan root.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Divide the HTML report into one section per language
    /// ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]). Also enabled by
    /// `[report] split_by_language = true` in `.deslop.toml`.
    #[arg(long)]
    split_by_language: bool,

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

    /// Duplication percentage above which `deslop` exits `3`
    /// ([EXIT-CODES]). Finite float in `[0.0, 100.0]`. Takes
    /// precedence over `[threshold] max_duplication_percent` in
    /// `.deslop.toml`. Mutually exclusive with `--no-fail-over`.
    #[arg(
        long,
        value_name = "PERCENT",
        conflicts_with = "no_fail_over",
        value_parser = parse_fail_over_percent,
    )]
    fail_over: Option<f64>,

    /// Clears any `.deslop.toml` fail-over threshold for this run
    /// so the CLI can never exit `3`. Useful when running the tool
    /// locally against a repo whose CI gate the developer does not
    /// want to trip.
    #[arg(long)]
    no_fail_over: bool,

    /// Show the researcher view on stderr — taxonomy IDs (Type-1/2/3),
    /// signal letters (s=structural, j=token, e=embedding), AST node
    /// counts, weight, LSH terminology. Off by default; the plain
    /// English summary is what humans actually want.
    #[arg(long)]
    technical: bool,

    /// After the initial analysis, re-run each listed path through the
    /// incremental session ([`PipelineSession::update_files`]) and emit
    /// the [`ReportDelta`] between the two generations as
    /// `<base>.delta.json`. Primary use: simulating a watcher or LSP
    /// transport driving [LIVE-STATE] updates end-to-end.
    #[arg(long = "rerun-touch", value_name = "PATH", num_args = 1.., action = clap::ArgAction::Append)]
    rerun_touch: Vec<PathBuf>,

    /// Remove each listed path from disk between the initial analysis
    /// and the rerun. Simulates a file deletion observed by a watcher:
    /// the rerun sees the path as missing and drops it from the corpus
    /// ([LIVE-STATE] `drop_path`). Implies `--rerun-touch` for each
    /// listed path. Must name a path inside the scan root.
    #[arg(long = "rerun-remove", value_name = "PATH", num_args = 1.., action = clap::ArgAction::Append)]
    rerun_remove: Vec<PathBuf>,

    /// Copy `SRC` to `DST` between the initial analysis and the rerun,
    /// then replay `DST` through [`PipelineSession::update_files`].
    /// Simulates a new file appearing mid-session: the initial corpus
    /// does not see `DST`; the rerun does and the delta surfaces the
    /// new clusters it joins ([LIVE-DELTA] `clusters_added`). Spec is
    /// `SRC=DST`; both paths must be absolute or resolvable against
    /// the current working directory.
    #[arg(long = "rerun-add", value_name = "SRC=DST", num_args = 1.., action = clap::ArgAction::Append)]
    rerun_add: Vec<String>,
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
    /// `<root>/.deslop/cache/fingerprints/...` keyed by
    /// `(language, tool_version, min_nodes, content_hash)`. On the
    /// next run, unchanged files skip tree-sitter entirely. Off by
    /// default — analysing a read-only checkout should not mutate it.
    #[arg(long)]
    incremental: bool,
    /// Send log events to stderr instead of a timestamped file. By
    /// default the CLI writes logs to `logs/deslop-<timestamp>.log`
    /// beside the report so the stderr stream stays readable.
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
            eprintln!("deslop: {err:#}");
            // Usage errors exit `2` like clap's own rejections
            // ([EXIT-CODES]); everything else is a runtime failure.
            let code = if err.is::<output::UsageError>() { 2 } else { 1 };
            std::process::exit(code);
        }
    }
}

/// Top-level flow. Kept separate from `main` so the error path can
/// be wrapped with a non-zero exit code without losing the
/// `anyhow::Error` chain.
fn run_cli() -> Result<()> {
    let raw_args: Vec<String> = env::args().collect();
    if let Some(output) = version_contract_output(&raw_args, "deslop", ComponentKind::Cli)? {
        print!("{output}");
        return Ok(());
    }
    let args = Cli::parse_from(raw_args);
    if let Some(file) = args.debug_ast.as_deref() {
        return run_debug_ast(file);
    }
    validate_scan_path(&args.path)?;
    let formats = FormatSelection::from_args(&args)?;
    let output = OutputPaths::new(args.output.as_deref(), &args.path);
    let mode: EmbeddingMode = parse_embedding_mode(&args.embeddings)?;
    let log_level = parse_log_level(&args.behaviour.log_level)?;
    let color = ColorChoice::resolve(args.behaviour.no_color);
    let log_sink = logging::init(
        &output.log_directory(),
        args.behaviour.log_to_console,
        log_level,
    )?;
    summary::preamble(
        color,
        &args.path,
        output.base_path(),
        &log_sink,
        &PreambleKnobs {
            min_nodes: args.min_nodes,
            embedding_mode: mode.as_str(),
            incremental: args.behaviour.incremental,
            technical: args.technical,
        },
    );
    tracing::info!(
        path = %args.path.display(),
        min_nodes = args.min_nodes,
        json = formats.json_enabled(),
        text = formats.text_enabled(),
        html = formats.html_enabled(),
        embeddings = mode.as_str(),
        incremental = args.behaviour.incremental,
        "deslop invoked",
    );
    let outcome = match produce_report(&args, mode, &formats) {
        Ok(outcome) => outcome,
        Err(err) => {
            summary::finish_err(color, &log_sink, &err);
            return Err(err);
        }
    };
    let mut report = outcome.report;
    apply_threshold(&args, &mut report)?;
    // The static schema_doc is served on demand (schema-doc / deslop://schema,
    // #110/#111); inlining ~13 KB of it into every rendered report drowns the
    // actual content and bloats the file. Drop it from the CLI output — the
    // wire field stays present-but-empty.
    report.schema_doc.clear();
    let split_by_language = resolve_split_by_language(&args)?;
    let mut written = emit_all(&report, &formats, &output, &args.path, split_by_language)?;
    if let Some(delta) = outcome.delta.as_ref() {
        let delta_path = output.path_with_extension("delta.json");
        let payload = serde_json::to_string_pretty(delta).context("serialise delta as JSON")?;
        write_file(&delta_path, payload.as_bytes())?;
        written.push(delta_path);
    }
    summary::summary(color, &report, args.technical);
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
    if report.metrics.breached() {
        std::process::exit(3);
    }
    Ok(())
}

/// Resolves the fail-over threshold from CLI flags + config file and
/// writes the verdict into `report.metrics.threshold`. Per [EXIT-CODES]:
/// `--no-fail-over` wins; `--fail-over` beats the config key; absence
/// means no gate.
fn apply_threshold(args: &Cli, report: &mut Report) -> Result<()> {
    if let Some(percent) = args.fail_over {
        report.metrics.threshold = ThresholdSummary::resolve(
            percent,
            ThresholdSource::Cli,
            report.metrics.duplication_percent,
        );
        return Ok(());
    }
    if args.no_fail_over {
        report.metrics.threshold = ThresholdSummary::none();
        return Ok(());
    }
    report.metrics.threshold =
        load_run_config(args)?.resolve_threshold(report.metrics.duplication_percent);
    Ok(())
}

/// Loads the effective `.deslop.toml` for this run — an explicit
/// `--config` path, or discovery next to the scan root. Missing config
/// is not an error; it yields the empty config.
fn load_run_config(args: &Cli) -> Result<ExclusionConfig> {
    match args.config.as_deref() {
        Some(path) => ExclusionConfig::load_for_root(path, &args.path)
            .with_context(|| format!("load config {}", path.display())),
        None => ExclusionConfig::discover(&args.path)
            .with_context(|| format!("discover config in {}", args.path.display())),
    }
}

/// Resolves whether the HTML report splits by language: the CLI
/// `--split-by-language` flag OR `[report] split_by_language` in
/// `.deslop.toml` ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]).
fn resolve_split_by_language(args: &Cli) -> Result<bool> {
    if args.split_by_language {
        return Ok(true);
    }
    Ok(load_run_config(args)?.split_by_language())
}

/// Names that look like CLI subcommands but are actually MCP tool
/// names or VSIX panel labels. When a user types `deslop top-offenders`
/// (as some agent recipes wrongly suggest), clap parses the word as
/// the positional `PATH`, the path resolves to a non-existent directory,
/// and the pipeline cheerfully reports zero clones ([Deslop#132]).
const KNOWN_NON_CLI_TOOL_NAMES: &[&str] = &[
    "top-offenders",
    "find-similar",
    "rescan",
    "report-get",
    "report-query",
    "report-for-file",
    "report-for-range",
    "cluster-by-id",
    "list-embedding-models",
    "set-embedding-model",
    "session-config",
    "schema-doc",
];

/// Refuses to scan a path that does not exist or matches a known MCP
/// tool name. Without this guard, `deslop top-offenders` silently
/// "succeeds" with a clean-looking report against a non-existent
/// directory ([Deslop#132]).
// [CLI-SUBCOMMAND-LOOKALIKE] rejects a positional path that is actually
// an MCP tool name / UI label with a named error (cli.md).
fn validate_scan_path(path: &std::path::Path) -> Result<()> {
    if let Some(name) = path.to_str() {
        if KNOWN_NON_CLI_TOOL_NAMES.contains(&name) {
            bail!(
                "{name:?} is an MCP tool name / UI label, not a CLI subcommand or directory. To scan the current workspace run `deslop .` (or `deslop <path>`); the MCP tool form is exposed via the deslop-mcp server, not the CLI.",
            );
        }
    }
    if !path.exists() {
        bail!(
            "scan path {} does not exist. Pass a real directory (or no argument to scan the current working directory).",
            path.display()
        );
    }
    Ok(())
}

/// Parses `file` and writes the normalised AST dump to stdout.
/// Developer entry point for `--debug-ast` ([PIPELINE-NORMALIZE-AST]):
/// no logging, no report, no cache mutation — just the tree.
fn run_debug_ast(file: &std::path::Path) -> Result<()> {
    let dump = debug_ast_dump(file).with_context(|| format!("debug-ast {}", file.display()))?;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(dump.as_bytes())
        .context("write ast dump to stdout")?;
    Ok(())
}

/// Either loads a cached report (`--from-report`) or runs the
/// pipeline end-to-end, optionally following with an incremental
/// rerun + [`ReportDelta`] ([LIVE-DELTA]).
#[derive(Debug)]
struct PipelineOutcome {
    /// Final report to emit as JSON/text/HTML.
    report: Report,
    /// Delta between the initial and rerun generations. `None` unless
    /// `--rerun-touch` was passed.
    delta: Option<ReportDelta>,
}

/// Either loads a cached report (`--from-report`) or runs the
/// pipeline end-to-end.
fn produce_report(
    args: &Cli,
    mode: EmbeddingMode,
    _formats: &FormatSelection,
) -> Result<PipelineOutcome> {
    if let Some(source) = &args.from_report {
        return Ok(PipelineOutcome {
            report: load_report(source)?,
            delta: None,
        });
    }
    let provider = configured_provider(args, mode)?;
    let provider_ref: Option<&dyn deslop_core::EmbeddingProvider> = provider.as_deref();
    let embedding = || EmbeddingSettings {
        mode,
        provider: provider_ref,
        batch_yield: None,
        progress: None,
    };
    let (mut session, initial) = PipelineSession::initialise(
        args.path.clone(),
        args.min_nodes,
        args.behaviour.incremental,
        args.config.clone(),
        embedding(),
    )
    .context("analysis pipeline failed")?;
    let adds = parse_rerun_adds(&args.rerun_add)?;
    let touched = assemble_touched(args, &adds);
    if touched.is_empty() {
        return Ok(PipelineOutcome {
            report: initial,
            delta: None,
        });
    }
    for path in &args.rerun_remove {
        fs::remove_file(path).with_context(|| format!("rerun-remove {}", path.display()))?;
    }
    for add in &adds {
        let _bytes = fs::copy(&add.src, &add.dst)
            .with_context(|| format!("rerun-add {} -> {}", add.src.display(), add.dst.display()))?;
    }
    tracing::info!(
        touched = touched.len(),
        removed = args.rerun_remove.len(),
        added = adds.len(),
        "rerun: replaying paths through PipelineSession::update_files",
    );
    // `None` means the replayed paths touched no analysed file, so the
    // initial report still stands and re-rendering it would be waste (#299).
    let updated = session
        .update_files(&touched, embedding())
        .context("incremental rerun failed")?;
    let delta = ReportDelta::between(Some((0, &initial)), 1, updated.as_ref().unwrap_or(&initial));
    Ok(PipelineOutcome {
        report: updated.unwrap_or(initial),
        delta: Some(delta),
    })
}

/// Parses and validates `--fail-over` at clap-time so invalid values
/// exit `2` (clap's default for argument errors) instead of surfacing
/// as a runtime `1`.
fn parse_fail_over_percent(raw: &str) -> Result<f64, String> {
    let parsed = raw
        .parse::<f64>()
        .map_err(|err| format!("--fail-over expects a number: {err}"))?;
    validate_threshold_percent(parsed)
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
) -> Result<Option<Box<dyn deslop_core::EmbeddingProvider>>> {
    if matches!(mode, EmbeddingMode::Off) {
        return Ok(None);
    }
    match args.embedding_provider.as_str() {
        DEFAULT_PROVIDER_ID => build_ollama_provider(args, mode),
        other => bail!("unknown embedding provider {other:?}"),
    }
}

/// Builds the Ollama provider. Under `auto`, a connection failure
/// downgrades to `None` with a warning so the pipeline can fall back.
/// Under `required`, it propagates as an error.
fn build_ollama_provider(
    args: &Cli,
    mode: EmbeddingMode,
) -> Result<Option<Box<dyn deslop_core::EmbeddingProvider>>> {
    let source = match OllamaProvider::connect(&args.embedding_endpoint, &args.embedding_model) {
        Ok(provider) => return Ok(Some(Box::new(provider))),
        Err(source) => source,
    };
    if matches!(mode, EmbeddingMode::Required) {
        return Err(anyhow::anyhow!(
            "embedding provider required but unreachable: {source}"
        ));
    }
    tracing::warn!(%source, "embedding provider unreachable — continuing without Type-4 recall");
    Ok(None)
}
