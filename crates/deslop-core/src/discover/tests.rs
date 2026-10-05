//! [LIVE-WATCHER-DIRECTORIES] Scoped discovery must be complete and prune excluded trees.

use std::{collections::BTreeSet, fs, io};

use super::*;

const SUBTREE: &str = "incoming";
const INCLUDED: &str = "incoming/source.rs";
const EXCLUDED: &str = "incoming/target/cache/source.rs";
const EMPTY_SOURCE: &str = "";
const DENIED: &str = "directory cannot be read";
const MISSING_FAILURE: &str = "the original walk failure must be retained";
const MISSING_PARENT: &str = "excluded file has a parent";

/// [LIVE-WATCHER-RESCAN] A failed walk cannot be interpreted as confirmed absence.
#[test]
fn incomplete_inventory_returns_the_discovery_failure() -> io::Result<()> {
    let error = ignore::Error::Io(io::Error::new(io::ErrorKind::PermissionDenied, DENIED));
    let result = collect_complete(
        [Err(error)],
        Path::new(SUBTREE),
        &HashMap::new(),
        &ExclusionConfig::empty(),
    );
    assert!(
        result.is_err(),
        "an incomplete inventory cannot authorize eviction"
    );
    let error = result
        .err()
        .ok_or_else(|| io::Error::other(MISSING_FAILURE))?;
    assert!(error.to_string().contains(DENIED));
    Ok(())
}

/// The iterator itself must never visit excluded descendants, including ignore-rule walks.
#[test]
fn scoped_walk_prunes_excluded_descendants_before_visiting_them() -> io::Result<()> {
    let root = traversal_fixture()?;
    let expected = BTreeSet::from([
        root.path().to_path_buf(),
        root.path().join(SUBTREE),
        root.path().join(INCLUDED),
    ]);
    let config = Arc::new(ExclusionConfig::empty().with_scan_root(root.path()));
    let mut walker = scoped_walker(root.path(), &root.path().join(SUBTREE), Some(config));
    assert_eq!(walked_paths(walker.build())?, expected);
    assert_eq!(walked_paths(walker.hidden(false).build())?, expected);
    Ok(())
}

/// Creates both eligible source and an excluded populated descendant.
fn traversal_fixture() -> io::Result<tempfile::TempDir> {
    let root = tempfile::tempdir()?;
    let excluded = root.path().join(EXCLUDED);
    fs::create_dir_all(
        excluded
            .parent()
            .ok_or_else(|| io::Error::other(MISSING_PARENT))?,
    )?;
    fs::write(&excluded, EMPTY_SOURCE)?;
    fs::write(root.path().join(INCLUDED), EMPTY_SOURCE)?;
    Ok(root)
}

/// Keeps the resource assertion on actual yielded entries rather than file admission.
fn walked_paths(walker: ignore::Walk) -> io::Result<BTreeSet<PathBuf>> {
    walker
        .map(|entry| entry.map(ignore::DirEntry::into_path))
        .collect::<Result<_, _>>()
        .map_err(io::Error::other)
}
