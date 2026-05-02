//! Report output helpers for the `deslop` CLI.

use std::{env, fs, io::Write as _, path::PathBuf};

use anyhow::{bail, Context, Result};
use deslop_core::{render::render_html, render::render_text, Report};

use crate::Cli;

/// Default base name for the three-format output written to CWD when
/// `--output` is not provided.
const DEFAULT_OUTPUT_STEM: &str = "deslop-report";

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
            bail!("at least one of --nojson/--notext/--nohtml must remain enabled");
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
    /// Picks the base path for rendered reports.
    pub(crate) fn new(explicit: Option<&std::path::Path>) -> Self {
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
}

/// Writes every enabled format for `report`.
pub(crate) fn emit_all(
    report: &Report,
    formats: &FormatSelection,
    output: &OutputPaths,
    scan_root: &std::path::Path,
) -> Result<Vec<PathBuf>> {
    let mut written: Vec<PathBuf> = Vec::with_capacity(3);
    emit_json(report, formats, output, &mut written)?;
    emit_text(report, formats, output, &mut written)?;
    emit_html(report, formats, output, scan_root, &mut written)?;
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
    written: &mut Vec<PathBuf>,
) -> Result<()> {
    if formats.html {
        let html = render_html(report, Some(scan_root));
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
    serde_json::from_str::<Report>(&source)
        .with_context(|| format!("parse report {}", path.display()))
}
