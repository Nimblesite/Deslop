//! Report output helpers for the `deslop` CLI.

use std::{fs, io::Write as _, path::PathBuf};

use anyhow::{Context, Result};
use deslop_core::{paths, render::render_html, render::render_text, Report};

use crate::Cli;

/// A post-parse usage error: an invalid flag combination clap's
/// declarative rules cannot express. `main` maps it to exit `2` — the
/// same code clap uses for its own argument rejections, so CI
/// consumers never mistake a misconfiguration for an analysis failure
/// ([EXIT-CODES]).
#[derive(Debug)]
pub(crate) struct UsageError(String);

impl UsageError {
    /// Builds a usage error carrying `message` verbatim.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for UsageError {}

/// Which of the three output formats are enabled for this run.
#[derive(Debug)]
pub(crate) struct FormatSelection {
    /// Emit canonical JSON (`<base>.json`).
    json: bool,
    /// Emit terse text view (`<base>.txt`).
    text: bool,
    /// Emit human-readable HTML view (`<base>.html`).
    html: bool,
}

impl FormatSelection {
    /// Builds the selection from the three suppression flags.
    pub(crate) fn from_args(args: &Cli) -> Result<Self> {
        let selection = Self {
            json: !args.suppress.nojson,
            text: !args.suppress.notext,
            html: !args.suppress.nohtml,
        };
        if !selection.json && !selection.text && !selection.html {
            return Err(UsageError::new(
                "at least one of --nojson/--notext/--nohtml must remain enabled",
            )
            .into());
        }
        Ok(selection)
    }

    /// Whether canonical JSON output is enabled.
    pub(crate) fn json_enabled(&self) -> bool {
        self.json
    }

    /// Whether text output is enabled.
    pub(crate) fn text_enabled(&self) -> bool {
        self.text
    }

    /// Whether HTML output is enabled.
    pub(crate) fn html_enabled(&self) -> bool {
        self.html
    }
}

/// Resolved output base path; renderers append their own extension.
#[derive(Debug)]
pub(crate) struct OutputPaths {
    /// `<base>` such that `<base>.json` etc. are the final paths.
    base: PathBuf,
}

impl OutputPaths {
    /// Picks the base path for rendered reports: the explicit
    /// `--output` prefix when given, else [OUTPUT-DIR]'s
    /// `<scan_root>/.deslop/deslop-report`. Defaulting against the scan
    /// root rather than the working directory is what makes the CLI,
    /// LSP, and MCP agree on one location for a given workspace.
    pub(crate) fn new(explicit: Option<&std::path::Path>, scan_root: &std::path::Path) -> Self {
        let base = explicit.map_or_else(
            || paths::report_base(scan_root),
            std::path::Path::to_path_buf,
        );
        Self { base }
    }

    /// Returns the unresolved base path.
    pub(crate) fn base_path(&self) -> &std::path::Path {
        &self.base
    }

    /// Returns the concrete on-disk path for a given extension.
    pub(crate) fn path_with_extension(&self, extension: &str) -> PathBuf {
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

    /// Directory that the report files sit in.
    pub(crate) fn directory(&self) -> &std::path::Path {
        self.base.parent().unwrap_or(std::path::Path::new("."))
    }

    /// Directory that timestamped log files sit in — a `logs/`
    /// subdirectory of [`Self::directory`] ([OUTPUT-DIR]), so the log
    /// files that accumulate run after run never bury the three report
    /// files a user actually opens.
    pub(crate) fn log_directory(&self) -> PathBuf {
        paths::logs_dir(self.directory())
    }
}

/// Writes every enabled format for `report`. `split_by_language`
/// divides the HTML report into per-language sections
/// ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]).
pub(crate) fn emit_all(
    report: &Report,
    formats: &FormatSelection,
    output: &OutputPaths,
    scan_root: &std::path::Path,
    split_by_language: bool,
) -> Result<Vec<PathBuf>> {
    let mut written: Vec<PathBuf> = Vec::with_capacity(3);
    emit_json(report, formats, output, &mut written)?;
    emit_text(report, formats, output, &mut written)?;
    emit_html(
        report,
        formats,
        output,
        scan_root,
        split_by_language,
        &mut written,
    )?;
    Ok(written)
}

/// Writes canonical JSON when enabled.
fn emit_json(
    report: &Report,
    formats: &FormatSelection,
    output: &OutputPaths,
    written: &mut Vec<PathBuf>,
) -> Result<()> {
    if formats.json {
        let json = serde_json::to_string_pretty(report).context("serialise report as JSON")?;
        write_rendered(output, "json", json.as_bytes(), written)?;
    }
    Ok(())
}

/// Writes the terse text view when enabled.
fn emit_text(
    report: &Report,
    formats: &FormatSelection,
    output: &OutputPaths,
    written: &mut Vec<PathBuf>,
) -> Result<()> {
    if formats.text {
        let text = render_text(report);
        write_rendered(output, "txt", text.as_bytes(), written)?;
    }
    Ok(())
}

/// Writes the human-readable HTML view when enabled.
fn emit_html(
    report: &Report,
    formats: &FormatSelection,
    output: &OutputPaths,
    scan_root: &std::path::Path,
    split_by_language: bool,
    written: &mut Vec<PathBuf>,
) -> Result<()> {
    if formats.html {
        let html = render_html(report, Some(scan_root), split_by_language);
        write_rendered(output, "html", html.as_bytes(), written)?;
    }
    Ok(())
}

/// Writes a rendered payload and records its final path.
fn write_rendered(
    output: &OutputPaths,
    extension: &str,
    payload: &[u8],
    written: &mut Vec<PathBuf>,
) -> Result<()> {
    let path = output.path_with_extension(extension);
    write_file(&path, payload)?;
    written.push(path);
    Ok(())
}

/// Writes `payload` to `path`, creating parent directories as needed.
pub(crate) fn write_file(path: &std::path::Path, payload: &[u8]) -> Result<()> {
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
pub(crate) fn load_report(path: &std::path::Path) -> Result<Report> {
    let source =
        fs::read_to_string(path).with_context(|| format!("read report {}", path.display()))?;
    let mut report = serde_json::from_str::<Report>(&source)
        .with_context(|| format!("parse report {}", path.display()))?;
    migrate_legacy_embedding_coverage(&mut report);
    // A replayed report carries the figures it was written with, but the
    // derived fields — rank, band, shape, occurrence count, fused gate,
    // evidence sentence — are the engine's to state, and a report written
    // before one of them existed must not render a zero
    // ([SEVERITY-BAND], [FUSION-CONTENT-GATE]).
    deslop_core::report_restamp::restamp_derived_fields(&mut report);
    Ok(report)
}

/// Reconstructs `succeeded_subtrees` for reports written before
/// per-occurrence coverage counting existed. The field deserializes to
/// zero when absent, and every writer honours
/// `attempted = succeeded + failed`, so a zero alongside a non-zero
/// `attempted - failed` can only be a legacy report; the reconstruction
/// is the invariant solved for the missing term, and a no-op on every
/// report that already honours it.
fn migrate_legacy_embedding_coverage(report: &mut Report) {
    if let Some(provenance) = report.embedding_provenance.as_mut() {
        if provenance.succeeded_subtrees == 0 {
            provenance.succeeded_subtrees = provenance
                .attempted_subtrees
                .saturating_sub(provenance.failed_subtrees);
        }
    }
}
