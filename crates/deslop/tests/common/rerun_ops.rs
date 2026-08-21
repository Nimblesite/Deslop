//! The `--rerun-add` spec vocabulary shared by every suite that drives
//! [`deslop_core::PipelineSession::update_files`] through the binary
//! ([LIVE-STATE]).
//!
//! `--rerun-add SRC=DST` copies `SRC` over `DST` *between* the initial
//! analysis and the rerun, so a file staged outside the scan root is
//! invisible to `initialise` and arrives only as a live change. That is
//! how both `rerun.rs` (delta shape) and `live_session_equivalence.rs`
//! (equivalence against a cold pass) mutate a tree mid-session, so the
//! spec is built in one place rather than spelled out per suite.

use std::{ffi::OsString, fs, path::Path};

use super::Result;

/// The `SRC=DST` spec `--rerun-add` takes.
///
/// An [`OsString`] rather than a formatted `String`: a scan root can sit
/// under a path the platform does not guarantee is UTF-8, and rendering
/// it through `Display` would hand the CLI a lossily-transcoded path
/// that names a different file — or no file at all.
pub(crate) fn add_spec(src: &Path, dst: &Path) -> OsString {
    let mut spec = OsString::from(src);
    spec.push("=");
    spec.push(dst);
    spec
}

/// Writes `contents` to `<staging_dir>/<stem>` — outside the scan root,
/// so `initialise` cannot see it — and returns the [`add_spec`] that
/// lands it at `dst` between generation 0 and generation 1.
///
/// # Errors
///
/// Returns the underlying I/O error when the staged file cannot be
/// written.
pub(crate) fn staged_spec(
    staging_dir: &Path,
    stem: &str,
    contents: &str,
    dst: &Path,
) -> Result<OsString> {
    let staged = staging_dir.join(stem);
    fs::write(&staged, contents)?;
    Ok(add_spec(&staged, dst))
}
