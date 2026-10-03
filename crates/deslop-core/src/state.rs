//! Centralised global state for the analysis pipeline.
//!
//! Implements [STATE-FILE-REGISTRY]: per-run mapping of `FileId ↔ PathBuf`,
//! scoped to a single pipeline instance so a future long-running daemon can
//! hold multiple analyses live in one process without cross-contamination.
//! Per the project charter (see `CLAUDE.md`), this is the **only** module
//! permitted to hold mutable state shared across pipeline stages.

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::config::ClonePolicy;

#[cfg(test)]
mod tests;

/// Opaque handle assigned by the [`FileRegistry`] to a discovered file.
///
/// `FileId` values are dense, monotonically increasing, and valid only within
/// the registry instance that issued them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

/// One-way mapping from [`FileId`] to absolute file paths. The
/// discovery walker produces a de-duplicated file list, so the
/// registry only needs forward lookup — re-registering the same path
/// is a caller bug and is not defended against.
#[derive(Debug, Default)]
pub struct FileRegistry {
    /// Dense `FileId.0 == index` storage.
    paths: Vec<PathBuf>,
}

impl FileRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `path` and returns its freshly allocated [`FileId`].
    pub fn register(&mut self, path: PathBuf) -> FileId {
        let index = u32::try_from(self.paths.len()).unwrap_or(u32::MAX);
        self.paths.push(path);
        FileId(index)
    }

    /// Returns the path associated with `id`, if any.
    #[must_use]
    pub fn path(&self, id: FileId) -> Option<&std::path::Path> {
        let index = usize::try_from(id.0).ok()?;
        self.paths.get(index).map(PathBuf::as_path)
    }
}

/// Current file membership, including files with no fingerprints.
/// Both indexes change together ([LIVE-SCHEDULER-REMOVAL-COST]).
#[derive(Debug, Default)]
pub(crate) struct LivePaths {
    /// Stable handles used by the analysis stages.
    by_id: HashMap<FileId, PathBuf>,
    /// Component-ordered paths for exact and subtree lookup.
    by_path: BTreeMap<PathBuf, FileId>,
}

impl LivePaths {
    /// Registers or replaces one live path in both indexes.
    pub(crate) fn insert(&mut self, id: FileId, path: PathBuf) -> Option<PathBuf> {
        let previous = self.remove(id);
        if let Some(previous_id) = self.by_path.insert(path.clone(), id) {
            let _previous_path = self.by_id.remove(&previous_id);
        }
        let _previous_path = self.by_id.insert(id, path);
        previous
    }

    /// Removes an identity and its matching ordered entry.
    pub(crate) fn remove(&mut self, id: FileId) -> Option<PathBuf> {
        let path = self.by_id.remove(&id)?;
        let _previous_id = self.by_path.remove(&path);
        Some(path)
    }

    /// Returns the live path assigned to a handle.
    pub(crate) fn get(&self, id: FileId) -> Option<&PathBuf> {
        self.by_id.get(&id)
    }

    /// Returns the live handle assigned to an exact path.
    pub(crate) fn file_id(&self, path: &Path) -> Option<FileId> {
        self.by_path.get(path).copied()
    }

    /// Visits identities and paths without exposing mutable index access.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&FileId, &PathBuf)> {
        self.by_id.iter()
    }

    /// Visits every current path, including files without fingerprints.
    pub(crate) fn values(&self) -> impl Iterator<Item = &PathBuf> {
        self.by_id.values()
    }

    /// Returns the analysed-file denominator's current membership count.
    pub(crate) fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Finds only matching descendants and the first boundary candidate.
    /// The trace counts visited paths, excluding the tree's key comparisons.
    pub(crate) fn descendants(&self, prefix: &Path) -> Vec<PathBuf> {
        let mut removal_paths_examined = usize::default();
        let matched: Vec<PathBuf> = self
            .by_path
            .range(prefix.to_path_buf()..)
            .take_while(|(path, _)| {
                removal_paths_examined = removal_paths_examined.saturating_add(1);
                path.starts_with(prefix)
            })
            .map(|(path, _)| path.clone())
            .collect();
        tracing::debug!(
            removal_paths_examined,
            matched_files = matched.len(),
            "live removal lookup"
        );
        matched
    }
}

/// Process-wide override of the `[ranking] structural_only` policy
/// ([RANK-STRUCTURAL-ONLY]). Set at most once, at process startup,
/// from the surface's own channel — `deslop-lsp
/// --ranking-structural-only`, fed by the
/// `deslop.ranking.structuralOnly` editor setting
/// ([RANK-STRUCTURAL-ONLY]). Consulted by every subsequent config
/// load so the editor channel wins over `.deslop.toml`.
static STRUCTURAL_ONLY_OVERRIDE: OnceLock<ClonePolicy> = OnceLock::new();

/// Records the process-wide [RANK-STRUCTURAL-ONLY] policy override.
/// First write wins; later calls are ignored — the override models a
/// startup flag, not a runtime toggle.
pub fn set_structural_only_override(policy: ClonePolicy) {
    let _first_write_wins = STRUCTURAL_ONLY_OVERRIDE.set(policy);
}

/// Returns the process-wide [RANK-STRUCTURAL-ONLY] policy override,
/// when one was recorded at startup.
#[must_use]
pub fn structural_only_override() -> Option<ClonePolicy> {
    STRUCTURAL_ONLY_OVERRIDE.get().copied()
}
