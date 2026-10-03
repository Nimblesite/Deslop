//! [LIVE-SCHEDULER-REMOVAL-COST] Isolated membership and prefix-index contracts.

use std::path::{Path, PathBuf};

use super::{FileId, FileRegistry, LivePaths};

/// A removed directory and its similarly named surviving sibling.
const DIRECTORY: &str = "pkg";
/// A normal live leaf under the removed directory.
const LEAF: &str = "pkg/A.rs";
/// Membership also retains a descendant with no fingerprints.
const EMPTY_LEAF: &str = "pkg/nested/empty.rs";
/// A component-prefix sibling must survive deletion of `pkg`.
const SIBLING: &str = "pkg_twin/B.rs";
/// A path outside the indexed subtree.
const UNKNOWN: &str = "other/removed.tmp";
/// Replacement path assigned to an existing file identity.
const MOVED: &str = "moved/A.rs";
/// Number of registered paths in the prefix fixture.
const INITIAL_FILES: usize = 3;
/// A surviving singleton index after the mutation sequence.
const SINGLE_FILE: usize = 1;
/// An empty index after its last identity is removed.
const NO_FILES: usize = 0;

/// Register one path through the same registry and index used by sessions.
fn register(registry: &mut FileRegistry, paths: &mut LivePaths, path: &str) -> FileId {
    let path = PathBuf::from(path);
    let id = registry.register(path.clone());
    assert_eq!(paths.insert(id, path), None);
    id
}

/// Verify that removing the final identity clears exact and prefix lookup.
fn assert_final_removal(paths: &mut LivePaths, id: FileId) {
    assert_eq!(paths.remove(id), Some(PathBuf::from(MOVED)));
    assert_eq!(paths.remove(id), None);
    assert_eq!(paths.file_id(Path::new(MOVED)), None);
    assert_eq!(paths.len(), NO_FILES);
    assert_eq!(paths.descendants(Path::new(MOVED)), Vec::<PathBuf>::new());
}

#[test]
fn subtree_lookup_respects_components_and_exact_leaves() {
    let mut registry = FileRegistry::new();
    let mut paths = LivePaths::default();
    let leaf = register(&mut registry, &mut paths, LEAF);
    let empty = register(&mut registry, &mut paths, EMPTY_LEAF);
    let sibling = register(&mut registry, &mut paths, SIBLING);
    assert_eq!(paths.len(), INITIAL_FILES);
    assert_eq!(
        paths.descendants(Path::new(DIRECTORY)),
        [PathBuf::from(LEAF), PathBuf::from(EMPTY_LEAF)]
    );
    assert_eq!(paths.descendants(Path::new(LEAF)), [PathBuf::from(LEAF)]);
    assert_eq!(paths.descendants(Path::new(UNKNOWN)), Vec::<PathBuf>::new());
    for (id, path) in [(leaf, LEAF), (empty, EMPTY_LEAF), (sibling, SIBLING)] {
        assert_eq!(paths.file_id(Path::new(path)), Some(id));
        assert_eq!(paths.get(id), Some(&PathBuf::from(path)));
    }
}

#[test]
fn replacements_and_removals_keep_both_indexes_consistent() {
    let mut registry = FileRegistry::new();
    let mut paths = LivePaths::default();
    let original = register(&mut registry, &mut paths, LEAF);
    assert_eq!(
        paths.insert(original, PathBuf::from(MOVED)),
        Some(PathBuf::from(LEAF))
    );
    assert_eq!(paths.file_id(Path::new(LEAF)), None);
    let replacement = register(&mut registry, &mut paths, MOVED);
    assert_eq!(paths.get(original), None);
    assert_eq!(paths.file_id(Path::new(MOVED)), Some(replacement));
    assert_eq!(paths.len(), SINGLE_FILE);
    assert_eq!(paths.values().count(), SINGLE_FILE);
    assert_eq!(paths.iter().count(), SINGLE_FILE);
    assert_final_removal(&mut paths, replacement);
}
