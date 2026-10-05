//! `notify`-backed file watcher ([LIVE-WATCHER]).
//!
//! Bridges the synchronous `notify` callback into an async
//! [`tokio::sync::mpsc`] channel. Filtering by extension and exclusion
//! happens before paths reach the channel — the scheduler downstream
//! never re-parses an excluded file.

#[cfg(test)]
mod tests;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use notify::{
    event::{CreateKind, EventKind, ModifyKind, RemoveKind, RenameMode},
    recommended_watcher, EventHandler, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    config::{is_config_path, ExclusionConfig},
    discover::is_ignore_rule_path,
};

/// Exclusion policy shared between the analysis session and the live
/// watcher ([CONFIG-EXCLUDE-DEPENDENCIES], [LIVE-CONFIG-LIVE]).
///
/// The watcher decides what reaches the scheduler, the session decides
/// what reaches the corpus. Handing each its own config lets them
/// disagree: a workspace opted into dependency analysis had its cold
/// scan read `node_modules`, while the watcher — started with an empty
/// config — filtered every new dependency file out before scheduling, so
/// the live report silently stopped matching the batch report for the
/// same corpus. One handle, swapped in place, cannot drift.
pub type LiveExclusion = Arc<RwLock<Arc<ExclusionConfig>>>;

/// Wraps a resolved config in a fresh [`LiveExclusion`] handle.
#[must_use]
pub fn live_exclusion(exclusion: Arc<ExclusionConfig>) -> LiveExclusion {
    Arc::new(RwLock::new(exclusion))
}

/// Replaces the policy behind `handle`, so the watcher's next decision
/// uses it. Both sides move together or the corpus definitions diverge.
pub fn publish_exclusion(handle: &LiveExclusion, exclusion: Arc<ExclusionConfig>) {
    let mut guard = handle
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = exclusion;
}

/// Async-friendly file watcher handle. Drop the value to stop watching.
#[derive(Debug)]
pub struct LiveWatcher {
    /// Concrete `notify` watcher kept alive for the value's lifetime.
    _watcher: RecommendedWatcher,
}

impl LiveWatcher {
    /// Starts watching `root` recursively. Returns the handle and the
    /// channel the scheduler should drain.
    ///
    /// # Errors
    ///
    /// Returns the underlying `notify` error when the watcher cannot
    /// be constructed (e.g. permission denied).
    ///
    /// `config_paths` lists exact filesystem paths that bypass the
    /// extension filter — used so `.deslop.toml` (and any explicit
    /// override) reaches the scheduler as a first-class live change
    /// ([LIVE-WATCHER]). Each entry is canonicalised before
    /// comparison so notify-reported paths line up on macOS
    /// (`/private/var/...` vs `/var/...`).
    pub fn start(
        root: &Path,
        extensions: Vec<String>,
        exclusion: LiveExclusion,
        config_paths: Vec<PathBuf>,
    ) -> Result<(Self, UnboundedReceiver<PathBuf>), notify::Error> {
        let (tx, rx) = mpsc::unbounded_channel::<PathBuf>();
        let allowed: HashSet<String> = extensions
            .into_iter()
            .map(|ext| ext.to_lowercase())
            .collect();
        let watched_config_paths: Vec<PathBuf> = config_paths
            .into_iter()
            .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
            .collect();
        let handler = WatcherHandler {
            root: root.to_path_buf(),
            sender: tx,
            allowed,
            exclusion,
            watched_config_paths,
        };
        let mut watcher = recommended_watcher(handler)?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok((Self { _watcher: watcher }, rx))
    }
}

/// Receives raw `notify` events and forwards filtered paths to the
/// async channel. Implements [`EventHandler`] (the `notify` callback
/// trait) so it can be installed in [`recommended_watcher`].
struct WatcherHandler {
    /// Scope restored when the backend reports lost events ([LIVE-WATCHER-RESCAN]).
    root: PathBuf,
    /// FIFO delivery retains bursts while analysis is busy ([LIVE-WATCHER-DELIVERY]).
    sender: UnboundedSender<PathBuf>,
    /// Allowed lowercase extensions (no leading `.`).
    allowed: HashSet<String>,
    /// Exclusion policy consulted before forwarding. Shared with the
    /// session so a `.deslop.toml` edit re-scopes both sides at once —
    /// the watcher and the cold scan must never disagree about what the
    /// corpus contains ([CONFIG-EXCLUDE-DEPENDENCIES]).
    exclusion: LiveExclusion,
    /// Canonical paths that bypass the extension/exclusion filter and
    /// reach the scheduler directly — `.deslop.toml` plus any explicit
    /// override path ([LIVE-WATCHER]).
    watched_config_paths: Vec<PathBuf>,
}

impl std::fmt::Debug for WatcherHandler {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WatcherHandler")
            .field("allowed", &self.allowed)
            .finish_non_exhaustive()
    }
}

impl EventHandler for WatcherHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let Ok(event) = event else {
            return;
        };
        if event.need_rescan() {
            deliver(&self.sender, self.root.clone());
            return;
        }
        if is_relevant_event(event.kind) {
            self.forward_paths(event);
        }
    }
}

impl WatcherHandler {
    /// Coalesces callback-local paths without suppressing later edits.
    fn forward_paths(&self, event: notify::Event) {
        // [LIVE-WATCHER] Dedup paths within this callback only; later callbacks
        // forward the same path again so every subsequent edit is delivered.
        let mut seen_in_batch: HashSet<PathBuf> = HashSet::new();
        for path in event.paths {
            if !seen_in_batch.insert(path.clone()) {
                continue;
            }
            self.forward_one(path, event.kind);
        }
    }
    /// Forwards one path if it passes the filter set.
    ///
    /// [LIVE-WATCHER-REMOVAL] Known files retain extension admission;
    /// directories and uncertain removals may own source descendants.
    /// Source removals bypass exclusions so former members remain evictable.
    /// Configurations and ignore rules always reach the session to rescope it.
    fn forward_one(&self, path: PathBuf, kind: EventKind) {
        if is_config_path(&path, &self.watched_config_paths) || is_ignore_rule_path(&path) {
            deliver(&self.sender, path);
            return;
        }
        if !path_matches_filter(&path, &self.allowed)
            && !may_remove_subtree(&path, kind)
            && !may_create_subtree(&path, kind)
        {
            return;
        }
        if !may_remove_members(kind) && self.current_exclusion().is_excluded(&path, None) {
            return;
        }
        deliver(&self.sender, path);
    }

    /// Reads the currently-published exclusion policy.
    fn current_exclusion(&self) -> Arc<ExclusionConfig> {
        let guard = self
            .exclusion
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(&guard)
    }
}

/// Untyped and non-file removals may invalidate a directory or an alias.
/// Existing regular rename targets still use ordinary source extension admission.
fn may_remove_subtree(path: &Path, kind: EventKind) -> bool {
    match kind {
        EventKind::Remove(removal) => removal != RemoveKind::File,
        EventKind::Modify(ModifyKind::Name(_)) => {
            !std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
        }
        _ => false,
    }
}

/// [LIVE-WATCHER-DIRECTORIES] Untyped creates need directory metadata, never traversal.
fn may_create_subtree(path: &Path, kind: EventKind) -> bool {
    matches!(kind, EventKind::Create(CreateKind::Folder))
        || (matches!(kind, EventKind::Create(CreateKind::Any | CreateKind::Other)) && path.is_dir())
}

/// Old rename paths must remain evictable even if current rules exclude them.
fn may_remove_members(kind: EventKind) -> bool {
    matches!(kind, EventKind::Remove(_))
        || matches!(kind, EventKind::Modify(ModifyKind::Name(mode)) if mode != RenameMode::To)
}

/// [LIVE-WATCHER-DELIVERY] Enqueues synchronously without blocking or dropping bursts.
/// A failed send means the scheduler has closed, so there is no live consumer.
fn deliver(sender: &UnboundedSender<PathBuf>, path: PathBuf) {
    if sender.send(path).is_err() {
        tracing::debug!("watcher scheduler receiver closed; event delivery stopped");
    }
}

/// Returns `true` for the `notify` event kinds that should trigger a
/// re-analysis pass. Access-only events are ignored.
fn is_relevant_event(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

/// Returns `true` when `path`'s lowercase extension appears in the
/// allowed set. Paths without an extension fail the test.
fn path_matches_filter(path: &Path, allowed: &HashSet<String>) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_lowercase)
        .is_some_and(|extension| allowed.contains(&extension))
}
