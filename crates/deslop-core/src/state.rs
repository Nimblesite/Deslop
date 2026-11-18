//! Centralised global state for the analysis pipeline.
//!
//! Implements [STATE-FILE-REGISTRY]: per-run mapping of `FileId ↔ PathBuf`,
//! scoped to a single pipeline instance so a future long-running daemon can
//! hold multiple analyses live in one process without cross-contamination.
//! Per the project charter (see `CLAUDE.md`), this is the **only** module
//! permitted to hold mutable state shared across pipeline stages.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock, PoisonError},
};

#[cfg(test)]
mod tests;

/// Each yielded range entry contributes one candidate, including the stopping boundary.
const ONE_CANDIDATE: usize = 1;

/// Opaque handle assigned by the [`FileRegistry`] to a discovered file.
///
/// `FileId` values increase monotonically and are never reused, so nothing
/// cached under a released handle can attach to a later file
/// ([LIVE-SCHEDULER-REGISTRY-BOUNDED]). Valid only within the registry
/// instance that issued them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

/// Mapping from [`FileId`] to absolute file paths for the files a session
/// currently holds. Re-registering a held path is a caller bug and is not
/// defended against; a released handle frees its path
/// ([LIVE-SCHEDULER-REGISTRY-BOUNDED]).
#[derive(Debug, Default)]
pub struct FileRegistry {
    /// Paths of the handles still held.
    paths: HashMap<FileId, PathBuf>,
    /// The next handle to issue.
    next: u32,
}

impl FileRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `path` and returns its freshly allocated [`FileId`].
    pub fn register(&mut self, path: PathBuf) -> FileId {
        let id = FileId(self.next);
        self.next = self.next.saturating_add(1);
        let _previous = self.paths.insert(id, path);
        id
    }

    /// Releases `id` once its file leaves the session, returning its path.
    pub fn release(&mut self, id: FileId) -> Option<PathBuf> {
        self.paths.remove(&id)
    }

    /// Number of files that currently hold a handle ([LIVE-SCHEDULER-REGISTRY-BOUNDED]).
    #[must_use]
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// True when no file holds a handle.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Returns the path associated with `id`, if any.
    #[must_use]
    pub fn path(&self, id: FileId) -> Option<&std::path::Path> {
        self.paths.get(&id).map(PathBuf::as_path)
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
        let (matched, removal_paths_examined) = indexed_subtree_paths(&self.by_path, prefix);
        tracing::debug!(
            removal_paths_examined,
            matched_files = matched.len(),
            "live removal lookup"
        );
        matched
    }
}

/// [LIVE-SCHEDULER-REMOVAL-COST] Selects indexed descendants and counts visited candidates.
/// [LIVE-WATCHER-DIRECTORIES] Ignore-rule eviction uses the same component boundary.
pub(crate) fn indexed_subtree_paths<Value>(
    index: &BTreeMap<PathBuf, Value>,
    prefix: &Path,
) -> (Vec<PathBuf>, usize) {
    let mut candidates_visited = usize::default();
    let paths = index
        .range(prefix.to_path_buf()..)
        .take_while(|(path, _)| {
            candidates_visited = candidates_visited.saturating_add(ONE_CANDIDATE);
            path.starts_with(prefix)
        })
        .map(|(path, _)| path.clone())
        .collect();
    (paths, candidates_visited)
}

/// [CLI-ARG-JOBS] Process-wide worker budget: the most threads any one
/// parallel stage of a scan may run at once. Set at most once, at process
/// startup, from `deslop --jobs`. Unset, a stage may use every core the
/// machine reports.
static WORKER_BUDGET: OnceLock<NonZeroUsize> = OnceLock::new();

/// Records the process-wide [CLI-ARG-JOBS] worker budget. First write
/// wins; the budget models a startup flag, not a runtime toggle.
pub fn set_worker_budget(workers: NonZeroUsize) {
    let _first_write_wins = WORKER_BUDGET.set(workers);
}

/// The [CLI-ARG-JOBS] budget every parallel stage shares: the `--jobs`
/// value when one was set, else the machine's available parallelism.
#[must_use]
pub fn worker_budget() -> usize {
    WORKER_BUDGET.get().copied().map_or_else(
        || std::thread::available_parallelism().map_or(1, NonZeroUsize::get),
        NonZeroUsize::get,
    )
}

/// [LIVE-SCHEDULER-HEAP-STEADY] Every node-kind name the process has
/// interned, each stored once. The grammars fix the set of names, so this
/// stops growing once each kind has been seen, however many worker threads
/// parse and however many passes run.
static KIND_NAMES: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());

/// The process's one `&'static str` for the node-kind name `raw`, stored
/// on first sight. A poisoned lock still holds only whole entries, so it is
/// read through.
#[must_use]
pub fn kind_name(raw: &str) -> &'static str {
    let mut names = KIND_NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(existing) = names.get(raw) {
        return existing;
    }
    let stored: &'static str = Box::leak(raw.to_owned().into_boxed_str());
    let _new = names.insert(stored);
    stored
}
