//! Source-event delivery stays complete and ordered ([LIVE-WATCHER-DELIVERY]).

mod directories;
mod removals;

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use notify::{event::DataChange, Event, EventHandler, EventKind};
use tokio::sync::mpsc;

use super::super::{live_exclusion, WatcherHandler};
use crate::config::ExclusionConfig;

/// Burst deliberately exceeds a small watcher queue's capacity.
const BURST_SIZE: usize = 257;
/// First generated path index.
const FIRST_EVENT: usize = 0;
/// Supported language suffix for source changes.
const SOURCE_EXTENSION: &str = "rs";
/// Stable generated path prefix.
const SOURCE_PREFIX: &str = "source";
/// Repeated source edits remain separate across callbacks.
const FIRST_SOURCE: &str = "first.rs";
/// A second source makes arrival order distinguishable.
const SECOND_SOURCE: &str = "second.rs";
/// An explicit configuration override can have no supported extension.
const CONFIG_PATH: &str = "workspace-settings";
/// Ignore changes bypass the source filter.
const IGNORE_PATH: &str = ".gitignore";
/// Removed directories must reach descendant eviction without an extension.
const REMOVED_DIRECTORY: &str = "removed-directory";
/// Reusable change kind for direct admission checks.
const CONTENT_CHANGE: EventKind =
    EventKind::Modify(notify::event::ModifyKind::Data(DataChange::Content));
/// A definite directory removal must bypass source extension admission.
const DIRECTORY_REMOVAL: EventKind = EventKind::Remove(notify::event::RemoveKind::Folder);

/// Builds distinct source changes beyond the former bounded queue limit.
fn source_paths() -> Vec<PathBuf> {
    (FIRST_EVENT..BURST_SIZE)
        .map(|index| PathBuf::from(format!("{SOURCE_PREFIX}{index}.{SOURCE_EXTENSION}")))
        .collect()
}

/// Installs the actual watcher admission policy with an observable receiver.
fn handler(sender: mpsc::UnboundedSender<PathBuf>) -> WatcherHandler {
    WatcherHandler {
        root: PathBuf::from(directories::WATCH_ROOT),
        sender,
        allowed: HashSet::from([SOURCE_EXTENSION.to_owned()]),
        exclusion: live_exclusion(Arc::new(ExclusionConfig::empty())),
        watched_config_paths: Vec::new(),
    }
}

/// Builds a real notify content-change event with the supplied ordered paths.
fn source_event(paths: &[PathBuf]) -> Event {
    paths
        .iter()
        .fold(Event::new(CONTENT_CHANGE), |event, path| {
            event.add_path(path.clone())
        })
}

/// Every expected source edit must arrive, in the original order.
fn assert_delivered(received: &[PathBuf], expected: &[PathBuf]) {
    assert_eq!(
        received.first(),
        expected.first(),
        "the first source edit must arrive"
    );
    assert_eq!(
        received.len(),
        expected.len(),
        "a full watcher queue must not discard source edits"
    );
    assert_eq!(
        received, expected,
        "every changed source path must reach the scheduler in order"
    );
}

/// Drains only paths the real callback already delivered, without waiting.
fn drain(receiver: &mut mpsc::UnboundedReceiver<PathBuf>) -> Vec<PathBuf> {
    std::iter::from_fn(|| receiver.try_recv().ok()).collect()
}

#[test]
fn source_burst_preserves_every_changed_path() {
    let expected = source_paths();
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    watcher.handle_event(Ok(source_event(&expected)));
    let delivered = drain(&mut receiver);
    assert_delivered(&delivered, &expected);
}

#[test]
fn callback_deduplication_preserves_later_edits_to_the_same_source() {
    let first = PathBuf::from(FIRST_SOURCE);
    let second = PathBuf::from(SECOND_SOURCE);
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    watcher.handle_event(Ok(source_event(&[
        first.clone(),
        first.clone(),
        second.clone(),
    ])));
    watcher.handle_event(Ok(source_event(std::slice::from_ref(&first))));
    assert_delivered(&drain(&mut receiver), &[first.clone(), second, first]);
}

#[test]
fn admitted_event_classes_drain_in_order_after_sender_closes() {
    let expected = [CONFIG_PATH, IGNORE_PATH, REMOVED_DIRECTORY, FIRST_SOURCE].map(PathBuf::from);
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    watcher
        .watched_config_paths
        .push(PathBuf::from(CONFIG_PATH));
    watcher.forward_one(PathBuf::from(CONFIG_PATH), CONTENT_CHANGE);
    watcher.forward_one(PathBuf::from(IGNORE_PATH), CONTENT_CHANGE);
    watcher.forward_one(PathBuf::from(REMOVED_DIRECTORY), DIRECTORY_REMOVAL);
    watcher.forward_one(PathBuf::from(FIRST_SOURCE), CONTENT_CHANGE);
    drop(watcher);
    assert_delivered(&drain(&mut receiver), &expected);
    assert!(matches!(
        receiver.try_recv(),
        Err(mpsc::error::TryRecvError::Disconnected)
    ));
}

#[test]
fn closed_receiver_allows_callbacks_to_finish() {
    let (sender, receiver) = mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    drop(receiver);
    watcher.handle_event(Ok(source_event(&source_paths())));
    assert!(watcher.sender.is_closed());
}
