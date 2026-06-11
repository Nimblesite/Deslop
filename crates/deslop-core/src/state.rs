//! Centralised global state for the analysis pipeline.
//!
//! Implements [STATE-FILE-REGISTRY]: per-run mapping of `FileId ↔ PathBuf`,
//! scoped to a single pipeline instance so a future long-running daemon can
//! hold multiple analyses live in one process without cross-contamination.
//! Per the project charter (see `CLAUDE.md`), this is the **only** module
//! permitted to hold mutable state shared across pipeline stages.

use std::{path::PathBuf, sync::OnceLock};

use crate::config::ClonePolicy;

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

/// Process-wide override of the `[ranking] structural_only` policy
/// ([RANK-STRUCTURAL-ONLY]). Set at most once, at process startup,
/// from the surface's own channel — `deslop-lsp
/// --ranking-structural-only`, fed by the
/// `deslop.ranking.structuralOnly` editor setting
/// ([VSIX-SETTINGS-RANKING]). Consulted by every subsequent config
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
