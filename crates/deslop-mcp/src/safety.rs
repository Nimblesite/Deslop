//! Workspace-root pinning + path traversal guard.
//!
//! Implements [MCP-SAFETY]: every path argument the agent supplies is
//! resolved against the workspace root pinned at `initialize`. A path
//! that resolves outside the root is rejected with
//! [`PathResolutionError::OutsideRoot`] — the agent never reaches a
//! file it was not given access to.

use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

/// Reasons [`resolve_within_root`] rejects a path.
#[derive(Debug, Error)]
pub enum PathResolutionError {
    /// The resolved path lies outside the workspace root.
    #[error("path {path:?} resolves outside the pinned workspace root {root:?}")]
    OutsideRoot {
        /// The agent-supplied path (after joining with the root when
        /// relative).
        path: PathBuf,
        /// The workspace root pinned at `initialize`.
        root: PathBuf,
    },
    /// The filesystem rejected the canonicalisation attempt for some
    /// other reason (permissions, broken symlink, etc.).
    #[error("failed to canonicalise {path:?}: {source}")]
    Io {
        /// The path whose canonicalisation failed.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
}

/// Resolves `candidate` against `root`, rejecting any path that —
/// after canonicalisation — does not live under `root`.
///
/// Absolute paths are canonicalised directly; relative paths are first
/// joined onto `root`. Canonicalisation resolves `..` components and
/// symlinks so a path like `root/../etc/passwd` is caught.
///
/// When `candidate` does not exist yet we still resolve its parent
/// and reattach the final component — this supports `find-similar`
/// with a path that the agent is about to create.
///
/// # Errors
///
/// Returns [`PathResolutionError::OutsideRoot`] when the resolved path
/// is not a descendant of `root`, or [`PathResolutionError::Io`] when
/// canonicalisation fails for any other reason.
pub fn resolve_within_root(root: &Path, candidate: &Path) -> Result<PathBuf, PathResolutionError> {
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let canonical_root = canonicalise(root)?;
    let canonical_candidate = canonicalise_best_effort(&joined)?;
    if canonical_candidate.starts_with(&canonical_root) {
        Ok(canonical_candidate)
    } else {
        Err(PathResolutionError::OutsideRoot {
            path: canonical_candidate,
            root: canonical_root,
        })
    }
}

/// Canonicalises `path` — wraps [`std::fs::canonicalize`] so the error
/// path is uniform.
fn canonicalise(path: &Path) -> Result<PathBuf, PathResolutionError> {
    std::fs::canonicalize(path).map_err(|source| PathResolutionError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Canonicalises `path`; if the leaf does not exist, canonicalises
/// its parent and rejoins the final component. Non-existent leaves
/// are legal inputs to `find-similar` (the agent is querying about
/// code it is about to write).
fn canonicalise_best_effort(path: &Path) -> Result<PathBuf, PathResolutionError> {
    match std::fs::canonicalize(path) {
        Ok(resolved) => Ok(resolved),
        Err(io_err) if io_err.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(|| PathResolutionError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::NotFound,
                    "path has no parent; cannot resolve against workspace root",
                ),
            })?;
            let leaf = path.file_name().ok_or_else(|| PathResolutionError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "path has no final component; cannot resolve against workspace root",
                ),
            })?;
            let canonical_parent = canonicalise(parent)?;
            Ok(canonical_parent.join(leaf))
        }
        Err(source) => Err(PathResolutionError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}
