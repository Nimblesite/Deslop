//! Centralised global state for the analysis pipeline.
//!
//! Implements [STATE-FILE-REGISTRY]: per-run mapping of `FileId ↔ PathBuf`,
//! scoped to a single pipeline instance so a future long-running daemon can
//! hold multiple analyses live in one process without cross-contamination.
//! Per the project charter (see `CLAUDE.md`), this is the **only** module
//! permitted to hold mutable state shared across pipeline stages.

use std::{collections::HashMap, path::PathBuf};

/// Opaque handle assigned by the [`FileRegistry`] to a discovered file.
///
/// `FileId` values are dense, monotonically increasing, and valid only within
/// the registry instance that issued them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

impl FileId {
    /// Returns the raw integer representation, primarily for logging and
    /// deterministic report output.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Bidirectional mapping between [`FileId`] and absolute file paths.
#[derive(Debug, Default)]
pub struct FileRegistry {
    /// Dense `FileId.0 == index` storage.
    paths: Vec<PathBuf>,
    /// Reverse lookup for deduplication when the same path is registered
    /// twice.
    lookup: HashMap<PathBuf, FileId>,
}

impl FileRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `path` and returns its [`FileId`]. Re-registering an already
    /// known path returns the existing id rather than allocating a new one.
    pub fn register(&mut self, path: PathBuf) -> FileId {
        if let Some(existing) = self.lookup.get(&path) {
            return *existing;
        }
        let index = u32::try_from(self.paths.len()).unwrap_or(u32::MAX);
        let id = FileId(index);
        self.paths.push(path.clone());
        let _previous = self.lookup.insert(path, id);
        id
    }

    /// Returns the path associated with `id`, if any.
    #[must_use]
    pub fn path(&self, id: FileId) -> Option<&std::path::Path> {
        let index = usize::try_from(id.0).ok()?;
        self.paths.get(index).map(PathBuf::as_path)
    }

    /// Number of files currently registered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Returns `true` when no files have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}
