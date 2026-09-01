//! [OUTPUT-DIR] Canonical on-disk output layout for a workspace.
//!
//! Every artefact Deslop writes for a scanned workspace lands under a
//! single `.deslop/` directory at the scan root, so a user has exactly
//! one path to gitignore, inspect, or delete:
//!
//! ```text
//! <root>/
//!.deslop.toml # config — user-authored, tracked
//!.deslop/ # everything Deslop writes
//! deslop-report.{json,txt,html} # rendered reports (CLI)
//! logs/deslop-<epoch>.log # tracing sink (CLI)
//! cache/ # analysis state, never hand-edited
//!       fingerprints/ embeddings/
//!       live-report.json deslop.sock deslop.port
//! ```
//!
//! The CLI, LSP, and MCP surfaces all resolve through this module, so
//! the three never disagree about where a workspace's artefacts live.
//! The CLI's `--output` flag overrides the report base (and with it the
//! log directory); nothing else is configurable, because the cache is
//! addressed by the LSP and MCP independently and must be discoverable
//! from the scan root alone.

use std::path::{Path, PathBuf};

/// Name of the per-workspace output directory, relative to the scan
/// root. Dot-prefixed so the discovery pass's hidden-directory prune
/// keeps Deslop's own artefacts out of the corpus it analyses.
pub const OUTPUT_DIR_NAME: &str = ".deslop";

/// Cache subdirectory of [`OUTPUT_DIR_NAME`], holding derived analysis
/// state — fingerprints, embeddings, the live state file, and the IPC
/// endpoint artifacts. Safe to delete; everything in it is rebuildable.
pub const CACHE_DIR_NAME: &str = "cache";

/// Log subdirectory of [`OUTPUT_DIR_NAME`]. Timestamped log files
/// accumulate, so they get their own directory rather than piling up
/// alongside the reports a user actually opens.
pub const LOGS_DIR_NAME: &str = "logs";

/// Base file name, without extension, of the rendered reports. The
/// renderers append `.json`, `.txt`, and `.html`.
pub const REPORT_STEM: &str = "deslop-report";

/// Output directory for `root` — `<root>/.deslop`.
#[must_use]
pub fn output_dir(root: &Path) -> PathBuf {
    root.join(OUTPUT_DIR_NAME)
}

/// Cache directory for `root` — `<root>/.deslop/cache`.
#[must_use]
pub fn cache_dir(root: &Path) -> PathBuf {
    output_dir(root).join(CACHE_DIR_NAME)
}

/// Default report base path for `root` — `<root>/.deslop/deslop-report`.
/// Callers append the per-format extension.
#[must_use]
pub fn report_base(root: &Path) -> PathBuf {
    output_dir(root).join(REPORT_STEM)
}

/// Log directory for reports written to `report_dir` —
/// `<report_dir>/logs`. Defined relative to the report directory rather
/// than the scan root so that redirecting reports with `--output` takes
/// the logs with it.
#[must_use]
pub fn logs_dir(report_dir: &Path) -> PathBuf {
    report_dir.join(LOGS_DIR_NAME)
}

/// The separator every path Deslop *reports* is joined with, on every
/// platform — occurrence paths, `metrics.per_file`, `metrics.folders`,
/// boilerplate rows, and the paths its refusals name.
///
/// A reported path is a wire value: the corpus manifests, the VSIX links,
/// the MCP `path_contains` filter and every AI consumer all name a file
/// this way. Serialising the platform separator instead made one tree
/// render two different reports, and made every consumer's path
/// comparison platform-conditional — `metrics.folders` had always joined
/// with `/`, so a Windows report shipped both conventions at once
/// (gh #439).
pub const WIRE_PATH_SEPARATOR: char = '/';

/// Rewrites `path` into the form every consumer of a reported path reads:
/// [`WIRE_PATH_SEPARATOR`]-joined, on every platform.
///
/// Lossless by construction. It rewrites only where the platform
/// separator is not already [`WIRE_PATH_SEPARATOR`], and no such platform
/// admits one in a file name. On a POSIX platform a backslash *is* a
/// legal file-name character, so the path is returned untouched.
#[must_use]
pub fn wire_path(path: &Path) -> PathBuf {
    if std::path::MAIN_SEPARATOR == WIRE_PATH_SEPARATOR {
        return path.to_path_buf();
    }
    PathBuf::from(rewrite_separators(
        &path.to_string_lossy(),
        std::path::MAIN_SEPARATOR,
    ))
}

/// The rewrite behind [`wire_path`], taking the platform separator as an
/// argument rather than reading it.
///
/// gh #439 is invisible on a platform that already separates with
/// [`WIRE_PATH_SEPARATOR`], so an assertion that reads the running
/// platform's separator can only pin the contract on Windows. Passing it
/// in lets the unit tests pin both directions everywhere.
fn rewrite_separators(text: &str, native: char) -> String {
    if native == WIRE_PATH_SEPARATOR {
        return text.to_owned();
    }
    text.replace(native, &WIRE_PATH_SEPARATOR.to_string())
}

#[cfg(test)]
mod tests;
