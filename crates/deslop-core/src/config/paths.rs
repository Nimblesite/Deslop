//! Shared configuration path identity ([LIVE-WATCHER-CONFIG-COST]).

use std::path::{Path, PathBuf};

use super::DEFAULT_CONFIG_FILENAME;

#[cfg(test)]
mod tests;

/// Builds the canonical set of config paths that should trigger a live
/// exclusion reload — `<root>/.deslop.toml` plus the explicit override
/// (if any) ([LIVE-CONFIG-LIVE]).
#[must_use]
pub fn watched_config_paths(root: &Path, override_path: Option<&Path>) -> Vec<PathBuf> {
    let default = root.join(DEFAULT_CONFIG_FILENAME);
    let mut paths = vec![canonicalise_or_clone(&default)];
    if let Some(explicit) = override_path {
        paths.push(canonicalise_or_clone(explicit));
    }
    paths
}

/// Returns `true` when `candidate` matches one of the watched config
/// paths in either canonical or as-given form.
#[must_use]
pub fn is_config_path(candidate: &Path, watched: &[PathBuf]) -> bool {
    is_config_path_with(candidate, watched, |path| std::fs::canonicalize(path))
}

/// [LIVE-WATCHER-CONFIG-COST] Measures native resolution through the real matcher.
fn is_config_path_with(
    candidate: &Path,
    watched: &[PathBuf],
    resolve: impl FnOnce(&Path) -> std::io::Result<PathBuf>,
) -> bool {
    if watched.iter().any(|path| path == candidate) {
        return true;
    }
    if !needs_config_resolution(candidate, watched) {
        return false;
    }
    let canonical = resolve(candidate).unwrap_or_else(|_| candidate.to_path_buf());
    watched.iter().any(|path| path == &canonical)
}

/// Canonicalises `path` when possible; otherwise returns a clone so
/// non-existent override paths still compare predictably.
fn canonicalise_or_clone(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// [LIVE-WATCHER-CONFIG-COST] Avoids realpath for unrelated build events.
/// Windows retains native resolution for existing paths because DOS names,
/// trailing dots, and case aliases cannot be reproduced by filename equality.
fn needs_config_resolution(path: &Path, watched: &[PathBuf]) -> bool {
    if watched.is_empty() {
        return false;
    }
    if watched
        .iter()
        .any(|config| config_name_may_match(path, config))
    {
        return true;
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => cfg!(windows) || metadata.file_type().is_symlink(),
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

/// Keeps ambiguous Unicode and case comparisons on the native resolver path.
fn config_name_may_match(path: &Path, config: &Path) -> bool {
    let candidate = path.file_name().and_then(std::ffi::OsStr::to_str);
    let configured = config.file_name().and_then(std::ffi::OsStr::to_str);
    match (candidate, configured) {
        (Some(candidate), Some(configured)) if candidate.is_ascii() && configured.is_ascii() => {
            candidate.eq_ignore_ascii_case(configured)
        }
        _ => true,
    }
}
