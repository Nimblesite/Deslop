//! Mtime-based read freshness for live reports
//! ([LIVE-READ-FRESHNESS], [Deslop#153], [Deslop#156]).
//!
//! The watcher-driven scheduler is asynchronous and debounced: when a
//! file changes on disk there is a ~250 ms window before the
//! corresponding `apply_changes` pass commits. During that window any
//! IPC read (`report/get`, `cluster/byId`, `report/forFile`,
//! `report/forRange`) would return cluster occurrences whose byte
//! ranges no longer match the file on disk. Agents reading those
//! ranges either edit unrelated code or conclude a duplicate they just
//! eliminated is still present.
//!
//! [`FreshnessTracker`] records the on-disk mtime of every file the
//! analysis pass touched. Before serving a read, the session calls
//! [`FreshnessTracker::detect_stale_paths`] against the live report's
//! occurrence paths. Any path whose on-disk mtime is newer (or whose
//! file vanished) is funnelled through `apply_changes` synchronously,
//! so every read after a synchronous edit reflects the current file
//! contents — without relying on the watcher having fired.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::report::Report;

/// Per-file mtime ledger consulted before serving any read.
#[derive(Debug, Default, Clone)]
pub struct FreshnessTracker {
    /// Mtime of each file as last observed by the analysis pipeline.
    /// Missing entries are treated as "never analysed" — the read
    /// path triggers a fresh `apply_changes` on first encounter.
    analyzed_mtimes: HashMap<PathBuf, SystemTime>,
}

impl FreshnessTracker {
    /// Constructs an empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the current on-disk mtime for `path`. A missing file
    /// clears the entry so the next read picks up the deletion.
    pub fn record_path(&mut self, root: &Path, path: &Path) {
        let absolute = absolutise(root, path);
        match std::fs::metadata(&absolute).and_then(|metadata| metadata.modified()) {
            Ok(mtime) => {
                let _previous = self.analyzed_mtimes.insert(absolute, mtime);
            }
            Err(_) => {
                let _removed = self.analyzed_mtimes.remove(&absolute);
            }
        }
    }

    /// Records the current mtime of every file referenced by `report`.
    /// Called after every analysis pass commits a new snapshot.
    pub fn record_from_report(&mut self, root: &Path, report: &Report) {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for cluster in &report.clusters {
            for occurrence in &cluster.occurrences {
                if seen.insert(occurrence.path.clone()) {
                    self.record_path(root, &occurrence.path);
                }
            }
        }
    }

    /// Returns the set of `report` occurrence paths whose on-disk
    /// mtime is newer than the analysed mtime — or whose file
    /// vanished. Each path is returned exactly once, in deterministic
    /// (path-sorted) order so callers can pass them straight to
    /// `apply_changes` without surprising the registry.
    #[must_use]
    pub fn detect_stale_paths(&self, root: &Path, report: &Report) -> Vec<PathBuf> {
        let mut stale: HashSet<PathBuf> = HashSet::new();
        for cluster in &report.clusters {
            for occurrence in &cluster.occurrences {
                let absolute = absolutise(root, &occurrence.path);
                if self.is_stale(&absolute) {
                    let _inserted = stale.insert(absolute);
                }
            }
        }
        let mut ordered: Vec<PathBuf> = stale.into_iter().collect();
        ordered.sort();
        ordered
    }

    /// Returns true when `absolute` has a different on-disk mtime
    /// than the recorded value, when the file has vanished, or when
    /// the file has never been recorded. The "never recorded" case
    /// is a conservative miss — it asks the caller to refresh once
    /// so the ledger catches up.
    fn is_stale(&self, absolute: &Path) -> bool {
        let recorded = self.analyzed_mtimes.get(absolute);
        let metadata = std::fs::metadata(absolute);
        match (recorded, metadata) {
            (Some(recorded_mtime), Ok(meta)) => meta
                .modified()
                .map_or(true, |on_disk| &on_disk != recorded_mtime),
            (Some(_), Err(_)) | (None, _) => true,
        }
    }
}

/// Resolves `path` against `root` when it is relative. Used so the
/// tracker stores one canonical absolute key per file regardless of
/// whether the watcher or the IPC caller used a relative form.
fn absolutise(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
