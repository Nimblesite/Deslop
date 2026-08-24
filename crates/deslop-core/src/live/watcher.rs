//! `notify`-backed file watcher ([LIVE-WATCHER]).
//!
//! Bridges the synchronous `notify` callback into an async
//! [`tokio::sync::mpsc`] channel. Filtering by extension and exclusion
//! happens before paths reach the channel — the scheduler downstream
//! never re-parses an excluded file.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use notify::{
    event::EventKind, recommended_watcher, EventHandler, RecommendedWatcher, RecursiveMode,
    Watcher as NotifyWatcher,
};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{config::ExclusionConfig, discover::is_ignore_rule_path};

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

/// Default channel capacity for the watcher → scheduler bridge.
/// Sized for short bursts of saves; the scheduler's debouncer
/// coalesces anything beyond.
const CHANNEL_CAPACITY: usize = 256;

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
    ) -> Result<(Self, Receiver<PathBuf>), notify::Error> {
        let (tx, rx) = mpsc::channel::<PathBuf>(CHANNEL_CAPACITY);
        let allowed: HashSet<String> = extensions
            .into_iter()
            .map(|ext| ext.to_lowercase())
            .collect();
        let watched_config_paths: HashSet<PathBuf> = config_paths
            .into_iter()
            .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
            .collect();
        let handler = WatcherHandler {
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
    /// Async outbound channel.
    sender: Sender<PathBuf>,
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
    watched_config_paths: HashSet<PathBuf>,
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
        if !is_relevant_event(event.kind) {
            return;
        }
        let is_removal = matches!(event.kind, EventKind::Remove(_));
        // Dedup paths within this single callback batch so a
        // `Modify(Metadata) + Modify(Data)` pair only fires once. The
        // set is stack-local — every new callback starts fresh, so a
        // path seen in an earlier batch is forwarded again on the next
        // edit ([LIVE-WATCHER]).
        let mut seen_in_batch: HashSet<PathBuf> = HashSet::new();
        for path in event.paths {
            if !seen_in_batch.insert(path.clone()) {
                continue;
            }
            self.forward_one(path, is_removal);
        }
    }
}

impl WatcherHandler {
    /// Forwards one path if it passes the filter set.
    ///
    /// Removals bypass the extension and exclusion filters: a removed
    /// directory has no source extension yet may be an ancestor of live
    /// files, and a previously-admitted path that an exclusion now
    /// matches must still be evictable ([LIVE-WATCHER]). The
    /// session re-checks existence and ownership before mutating any
    /// map, so forwarding a removal it does not track is a cheap no-op.
    fn forward_one(&self, path: PathBuf, is_removal: bool) {
        if self.is_watched_config_path(&path) {
            let _result = self.sender.try_send(path);
            return;
        }
        // Ignore-rule files bypass the extension and exclusion filters
        // the same way `.deslop.toml` does: editing one re-scopes the
        // live ingest gate, so the session must see it to rebuild its
        // matcher and re-evaluate the corpus (ignore-rule parity #287).
        if is_ignore_rule_path(&path) {
            let _result = self.sender.try_send(path);
            return;
        }
        if is_removal {
            let _result = self.sender.try_send(path);
            return;
        }
        if !path_matches_filter(&path, &self.allowed) {
            return;
        }
        if self.current_exclusion().is_excluded(&path, None) {
            return;
        }
        let _result = self.sender.try_send(path);
    }

    /// Reads the currently-published exclusion policy.
    fn current_exclusion(&self) -> Arc<ExclusionConfig> {
        let guard = self
            .exclusion
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(&guard)
    }

    /// Returns `true` when `path` (or its canonical form) matches a
    /// watched config path. Used to bypass the extension filter for
    /// `.deslop.toml` ([LIVE-WATCHER]).
    fn is_watched_config_path(&self, path: &Path) -> bool {
        if self.watched_config_paths.contains(path) {
            return true;
        }
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.watched_config_paths.contains(&canonical)
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
