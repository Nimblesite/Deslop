//! `notify`-backed file watcher ([LIVE-WATCHER]).
//!
//! Bridges the synchronous `notify` callback into an async
//! [`tokio::sync::mpsc`] channel. Filtering by extension and exclusion
//! happens before paths reach the channel — the scheduler downstream
//! never re-parses an excluded file.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use notify::{
    event::EventKind, recommended_watcher, EventHandler, RecommendedWatcher, RecursiveMode,
    Watcher as NotifyWatcher,
};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::config::ExclusionConfig;

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
    pub fn start(
        root: &Path,
        extensions: Vec<String>,
        exclusion: Arc<ExclusionConfig>,
    ) -> Result<(Self, Receiver<PathBuf>), notify::Error> {
        let (tx, rx) = mpsc::channel::<PathBuf>(CHANNEL_CAPACITY);
        let allowed: HashSet<String> = extensions
            .into_iter()
            .map(|ext| ext.to_lowercase())
            .collect();
        let handler = WatcherHandler {
            sender: tx,
            allowed,
            exclusion,
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
    /// Exclusion config consulted before forwarding.
    exclusion: Arc<ExclusionConfig>,
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
            self.forward_one(path);
        }
    }
}

impl WatcherHandler {
    /// Forwards one path if it passes the filter set.
    fn forward_one(&self, path: PathBuf) {
        if !path_matches_filter(&path, &self.allowed) {
            return;
        }
        if self.exclusion.is_excluded(&path, None) {
            return;
        }
        let _result = self.sender.try_send(path);
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
