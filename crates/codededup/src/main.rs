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
    debug_ast_dump, render::render_html, render::render_text, validate_threshold_percent,
    EmbeddingMode, EmbeddingSettings, ExclusionConfig, OllamaProvider, PipelineSession, Report,
    ReportDelta, StubProvider, ThresholdSource, ThresholdSummary, DEFAULT_OLLAMA_ENDPOINT,
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

    /// Parse a single source file and print the normalised AST to
    /// stdout, then exit. Developer tool — bypasses the analysis
    /// pipeline, writes nothing to disk, and mutates no caches.
    /// Conflicts with `--from-report`.
    #[arg(long, value_name = "FILE", conflicts_with = "from_report")]
    debug_ast: Option<PathBuf>,

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

    /// Duplication percentage above which `codededup` exits `3`
    /// ([EXIT-CODES]). Finite float in `[0.0, 100.0]`. Takes
    /// precedence over `[threshold] max_duplication_percent` in
    /// `.codededup.toml`. Mutually exclusive with `--no-fail-over`.
    #[arg(
        long,
        value_name = "PERCENT",
        conflicts_with = "no_fail_over",
        value_parser = parse_fail_over_percent,
    )]
    fail_over: Option<f64>,

    /// Clears any `.codededup.toml` fail-over threshold for this run
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
    if let Some(file) = args.debug_ast.as_deref() {
        return run_debug_ast(file);
    }
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
            technical: args.technical,
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
    let outcome = match produce_report(&args, mode, &formats) {
        Ok(outcome) => outcome,
        Err(err) => {
            summary::finish_err(color, &log_sink, &err);
            return Err(err);
        }
    };
    let mut report = outcome.report;
    apply_threshold(&args, &mut report)?;
    let mut written = emit_all(&report, &formats, &output, &args.path)?;
    if let Some(delta) = outcome.delta.as_ref() {
        let delta_path = output.with_extension("delta.json");
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
    let config_percent = resolve_config_threshold(args)?;
    report.metrics.threshold = match config_percent {
        Some(percent) => ThresholdSummary::resolve(
            percent,
            ThresholdSource::Config,
            report.metrics.duplication_percent,
        ),
        None => ThresholdSummary::none(),
    };
    Ok(())
}

/// Loads `.codededup.toml` (if any) to surface the
/// `[threshold] max_duplication_percent` key without mutating the
/// pipeline path. Returns `None` when no config file exists or the
/// key is absent.
fn resolve_config_threshold(args: &Cli) -> Result<Option<f64>> {
    let config = match args.config.as_deref() {
        Some(path) => ExclusionConfig::load(path)
            .with_context(|| format!("load config {}", path.display()))?,
        None => ExclusionConfig::discover(&args.path)
            .with_context(|| format!("discover config in {}", args.path.display()))?,
    };
    Ok(config.fail_over_percent())
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
    let provider_ref: Option<&dyn codededup_core::EmbeddingProvider> = provider.as_deref();
    let embedding = || EmbeddingSettings {
        mode,
        provider: provider_ref,
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
    let updated = session
        .update_files(&touched, embedding())
        .context("incremental rerun failed")?;
    let delta = ReportDelta::between(Some((0, &initial)), 1, &updated);
    Ok(PipelineOutcome {
        report: updated,
        delta: Some(delta),
    })
}

/// Parsed `--rerun-add` entry: copy `src` to `dst` between the initial
/// analysis and the rerun, then replay `dst` through `update_files`.
#[derive(Debug)]
struct RerunAdd {
    /// Source path copied from (must exist at rerun time).
    src: PathBuf,
    /// Destination path copied to (inside the scan root).
    dst: PathBuf,
}

/// Parses the `SRC=DST` spec. Rejects specs that do not contain exactly
/// one `=` separator.
fn parse_rerun_adds(specs: &[String]) -> Result<Vec<RerunAdd>> {
    specs
        .iter()
        .map(|spec| {
            let (src, dst) = spec
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("--rerun-add spec must be SRC=DST, got {spec:?}"))?;
            Ok(RerunAdd {
                src: PathBuf::from(src),
                dst: PathBuf::from(dst),
            })
        })
        .collect()
}

/// Merges `--rerun-touch`, `--rerun-remove`, and `--rerun-add` paths
/// into the single vector passed to [`PipelineSession::update_files`].
/// Deletions and additions are implicitly touched so the session picks
/// them up.
fn assemble_touched(args: &Cli, adds: &[RerunAdd]) -> Vec<PathBuf> {
    let capacity = args
        .rerun_touch
        .len()
        .saturating_add(args.rerun_remove.len())
        .saturating_add(adds.len());
    let mut out: Vec<PathBuf> = Vec::with_capacity(capacity);
    out.extend(args.rerun_touch.iter().cloned());
    for path in &args.rerun_remove {
        if !out.contains(path) {
            out.push(path.clone());
        }
    }
    for add in adds {
        if !out.contains(&add.dst) {
            out.push(add.dst.clone());
        }
    }
    out
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
    scan_root: &std::path::Path,
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
        let html = render_html(report, Some(scan_root));
        let path = output.with_extension("html");
        write_file(&path, html.as_bytes())?;
        written.push(path);
    }
    Ok(written)
}

/// Writes `payload` to `path`, creating parent directories as needed.
fn write_file(path: &std::path::Path, payload: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("create parent {}", parent.display()))?;
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
