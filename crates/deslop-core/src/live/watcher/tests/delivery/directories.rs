//! Structural events retain complete subtrees ([LIVE-WATCHER-DIRECTORIES]).

use std::{path::PathBuf, sync::Arc};

use notify::{
    event::{CreateKind, Flag, ModifyKind, RenameMode},
    Event, EventHandler, EventKind,
};
use tokio::sync::mpsc;

use super::{assert_delivered, drain, handler, source_event, FIRST_SOURCE};
use crate::{config::ExclusionConfig, live::watcher::WatcherHandler};

/// Directory newly created or moved into the workspace.
const CREATED: &str = "created";
/// A removed rename source has no surviving metadata.
const PREVIOUS: &str = "previous";
/// Built-in exclusions must stop subtree traversal before it starts.
const EXCLUDED: &str = "node_modules";
/// A regular build file must not trigger directory discovery.
const BUILD_FILE: &str = "compiled.tmp";
/// The fixture watch root used by the common callback handler.
pub(super) const WATCH_ROOT: &str = "workspace";
/// Every rename classification must preserve structural changes.
const RENAME_MODES: &[RenameMode] = &[
    RenameMode::From,
    RenameMode::To,
    RenameMode::Both,
    RenameMode::Any,
    RenameMode::Other,
];
/// Root and receiver keep callback tests independent of OS event timing.
type DirectoryFixture = (
    tempfile::TempDir,
    WatcherHandler,
    mpsc::UnboundedReceiver<PathBuf>,
);

fn directory_fixture() -> std::io::Result<DirectoryFixture> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join(CREATED))?;
    std::fs::create_dir(root.path().join(EXCLUDED))?;
    std::fs::write(root.path().join(BUILD_FILE), [])?;
    std::fs::write(root.path().join(FIRST_SOURCE), [])?;
    let (sender, receiver) = mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    let exclusion = ExclusionConfig::empty().with_scan_root(root.path());
    watcher.exclusion = super::super::super::live_exclusion(Arc::new(exclusion));
    Ok((root, watcher, receiver))
}

#[test]
fn directory_creation_keeps_sources_and_rejects_excluded_build_output() -> std::io::Result<()> {
    let (root, mut watcher, mut receiver) = directory_fixture()?;
    let created = root.path().join(CREATED);
    let paths = [created.clone(), root.path().join(EXCLUDED)];
    for kind in [CreateKind::Folder, CreateKind::Any, CreateKind::Other] {
        watcher.handle_event(Ok(source_event(&paths).set_kind(EventKind::Create(kind))));
        assert_delivered(&drain(&mut receiver), std::slice::from_ref(&created));
    }
    Ok(())
}

#[test]
fn rename_events_keep_missing_old_directories_and_existing_new_directories() -> std::io::Result<()>
{
    let (root, mut watcher, mut receiver) = directory_fixture()?;
    let expected = [root.path().join(PREVIOUS), root.path().join(CREATED)];
    for mode in RENAME_MODES {
        let kind = EventKind::Modify(ModifyKind::Name(*mode));
        watcher.handle_event(Ok(source_event(&expected).set_kind(kind)));
        assert_delivered(&drain(&mut receiver), &expected);
    }
    Ok(())
}

#[test]
fn rename_source_eviction_survives_exclusions_but_new_excluded_targets_do_not(
) -> std::io::Result<()> {
    let (root, mut watcher, mut receiver) = directory_fixture()?;
    let paths = [root.path().join(CREATED), root.path().join(EXCLUDED)];
    for mode in RENAME_MODES.iter().filter(|mode| **mode != RenameMode::To) {
        let kind = EventKind::Modify(ModifyKind::Name(*mode));
        watcher.handle_event(Ok(source_event(&paths).set_kind(kind)));
        assert_delivered(&drain(&mut receiver), &paths);
    }
    let kind = EventKind::Modify(ModifyKind::Name(RenameMode::To));
    watcher.handle_event(Ok(source_event(&paths).set_kind(kind)));
    assert_delivered(&drain(&mut receiver), &[root.path().join(CREATED)]);
    Ok(())
}

#[test]
fn file_creations_keep_sources_without_discovering_regular_build_files() -> std::io::Result<()> {
    let (root, mut watcher, mut receiver) = directory_fixture()?;
    let source = root.path().join(FIRST_SOURCE);
    let paths = [source.clone(), root.path().join(BUILD_FILE)];
    for kind in [CreateKind::File, CreateKind::Any, CreateKind::Other] {
        watcher.handle_event(Ok(source_event(&paths).set_kind(EventKind::Create(kind))));
        assert_delivered(&drain(&mut receiver), std::slice::from_ref(&source));
    }
    Ok(())
}

#[test]
fn rename_events_skip_regular_build_files_and_keep_source_or_structural_paths(
) -> std::io::Result<()> {
    let (root, mut watcher, mut receiver) = directory_fixture()?;
    let expected = [PREVIOUS, CREATED, FIRST_SOURCE].map(|name| root.path().join(name));
    let paths = [expected.to_vec(), vec![root.path().join(BUILD_FILE)]].concat();
    for mode in RENAME_MODES {
        let kind = EventKind::Modify(ModifyKind::Name(*mode));
        watcher.handle_event(Ok(source_event(&paths).set_kind(kind)));
        assert_delivered(&drain(&mut receiver), &expected);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn rename_events_keep_leaf_and_directory_symlink_aliases() -> std::io::Result<()> {
    const FILE_ALIAS: &str = "source-link.tmp";
    const DIRECTORY_ALIAS: &str = "directory-link.tmp";
    let (root, mut watcher, mut receiver) = directory_fixture()?;
    let expected = [FILE_ALIAS, DIRECTORY_ALIAS].map(|name| root.path().join(name));
    std::os::unix::fs::symlink(root.path().join(FIRST_SOURCE), root.path().join(FILE_ALIAS))?;
    std::os::unix::fs::symlink(root.path().join(CREATED), root.path().join(DIRECTORY_ALIAS))?;
    for mode in RENAME_MODES {
        let kind = EventKind::Modify(ModifyKind::Name(*mode));
        watcher.handle_event(Ok(source_event(&expected).set_kind(kind)));
        assert_delivered(&drain(&mut receiver), &expected);
    }
    Ok(())
}

/// A kernel overflow contains no individual paths ([LIVE-WATCHER-RESCAN]).
#[test]
fn rescan_without_paths_preserves_the_workspace_reconciliation_request() {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut watcher = handler(sender);
    watcher.handle_event(Ok(Event::new(EventKind::Other).set_flag(Flag::Rescan)));
    assert_delivered(&drain(&mut receiver), &[PathBuf::from(WATCH_ROOT)]);
}
