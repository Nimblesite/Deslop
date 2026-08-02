//! [OUTPUT-DIR] Canonical on-disk output layout for a workspace.
//!
//! Every artefact Deslop writes for a scanned workspace lands under a
//! single `.deslop/` directory at the scan root, so a user has exactly
//! one path to gitignore, inspect, or delete:
//!
//! ```text
//! <root>/
//!   .deslop.toml                     # config — user-authored, tracked
//!   .deslop/                         # everything Deslop writes
//!     deslop-report.{json,txt,html}  # rendered reports (CLI)
//!     logs/deslop-<epoch>.log        # tracing sink (CLI)
//!     cache/                         # analysis state, never hand-edited
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
