//! Source-event delivery must not lose edits during bursts ([LIVE-WATCHER]).

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use notify::{event::DataChange, Event, EventHandler, EventKind};
use tokio::sync::mpsc;

use super::super::{live_exclusion, WatcherHandler, CHANNEL_CAPACITY};
use crate::config::ExclusionConfig;

const EXTRA_EVENT: usize = 1;
const FIRST_EVENT: usize = 0;
const SOURCE_EXTENSION: &str = "rs";
const SOURCE_PREFIX: &str = "source";

fn source_paths() -> Vec<PathBuf> {
    (FIRST_EVENT..CHANNEL_CAPACITY + EXTRA_EVENT)
        .map(|index| PathBuf::from(format!("{SOURCE_PREFIX}{index}.{SOURCE_EXTENSION}")))
        .collect()
}

fn handler(sender: mpsc::Sender<PathBuf>) -> WatcherHandler {
    WatcherHandler {
        sender,
        allowed: HashSet::from([SOURCE_EXTENSION.to_owned()]),
        exclusion: live_exclusion(Arc::new(ExclusionConfig::empty())),
        watched_config_paths: HashSet::new(),
    }
}

fn source_event(paths: &[PathBuf]) -> Event {
    let kind = EventKind::Modify(notify::event::ModifyKind::Data(DataChange::Content));
    paths
        .iter()
        .fold(Event::new(kind), |event, path| event.add_path(path.clone()))
}

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

#[test]
fn source_burst_preserves_every_changed_path() {
    let expected = source_paths();
    let (sender, mut receiver) = mpsc::channel(CHANNEL_CAPACITY);
    let mut watcher = handler(sender);
    watcher.handle_event(Ok(source_event(&expected)));
    let received: Vec<PathBuf> = std::iter::from_fn(|| receiver.try_recv().ok()).collect();
    assert_delivered(&received, &expected);
}
