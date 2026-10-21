//! Typed file removals filter noise without losing eviction ([LIVE-WATCHER-REMOVAL]).

use std::{path::Path, sync::Arc};

use notify::{event::RemoveKind, EventHandler, EventKind};

use super::{
    assert_delivered, drain, handler, source_event, CONFIG_PATH, FIRST_SOURCE, IGNORE_PATH,
    REMOVED_DIRECTORY,
};
use crate::{
    config::ExclusionConfig,
    live::watcher::{live_exclusion, WatcherHandler},
};

/// Build products cannot enter the corpus by extension.
const BUILD_FILE: &str = "compiled.tmp";
/// Previously admitted source may become excluded after configuration changes.
const EXCLUDED_SOURCE: &str = "workspace/node_modules/former.rs";
/// Root binding makes dependency exclusion observable in the test.
const WORKSPACE: &str = "workspace";

fn send_removal(watcher: &mut WatcherHandler, kind: RemoveKind, names: &[&str]) {
    let paths: Vec<_> = names.iter().map(std::path::PathBuf::from).collect();
    watcher.handle_event(Ok(source_event(&paths).set_kind(EventKind::Remove(kind))));
}

fn admit_removed_source(watcher: &mut WatcherHandler) {
    let exclusion = ExclusionConfig::empty().with_scan_root(Path::new(WORKSPACE));
    assert!(exclusion.is_excluded(Path::new(EXCLUDED_SOURCE), None));
    watcher.exclusion = live_exclusion(Arc::new(exclusion));
    watcher.watched_config_paths.push(CONFIG_PATH.into());
}

#[test]
fn removed_files_keep_source_config_and_ignore_changes() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    admit_removed_source(&mut watcher);
    let expected = [FIRST_SOURCE, CONFIG_PATH, IGNORE_PATH, EXCLUDED_SOURCE];
    let paths = [
        BUILD_FILE,
        FIRST_SOURCE,
        CONFIG_PATH,
        IGNORE_PATH,
        EXCLUDED_SOURCE,
    ];
    send_removal(&mut watcher, RemoveKind::File, &paths);
    assert_delivered(
        &drain(&mut receiver),
        &expected.map(std::path::PathBuf::from),
    );
}

#[test]
fn removed_directories_and_unknown_kinds_keep_every_path() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    let names = [REMOVED_DIRECTORY, BUILD_FILE, FIRST_SOURCE];
    let expected = names.map(std::path::PathBuf::from);
    for kind in [RemoveKind::Folder, RemoveKind::Any, RemoveKind::Other] {
        send_removal(&mut watcher, kind, &names);
        assert_delivered(&drain(&mut receiver), &expected);
    }
}
