//! [PERF-FLUTTER-TODO-MEMORY] What the bounded store keeps and drops.
//!
//! The store is a pure memo, so a wrong eviction cannot change a
//! reported cluster — it can only make a corpus-scale run re-parse the
//! file it is about to ask for again, which is how this stage became
//! minutes long in the first place. These assertions pin the residency
//! rules that keep the working set both bounded and useful.

use std::path::PathBuf;

use super::{Lookup, TreeStore, PARSE_TREE_SOURCE_BUDGET_BYTES};
use crate::state::{FileId, FileRegistry};

/// A file large enough that three of them cannot share the budget.
const LARGE_FILE_BYTES: usize = PARSE_TREE_SOURCE_BUDGET_BYTES / 2;

/// A file small enough to sit alongside anything.
const SMALL_FILE_BYTES: usize = 1_024;

/// Registers `count` distinct paths and returns their ids.
fn files(count: usize) -> Vec<FileId> {
    let mut registry = FileRegistry::new();
    (0..count)
        .map(|index| registry.register(PathBuf::from(format!("file{index}.dart"))))
        .collect()
}

/// Whether `store` still holds `file`.
fn holds(store: &mut TreeStore, file: FileId) -> bool {
    matches!(store.hit(file), Lookup::Remembered(_))
}

/// [PERF-FLUTTER-TODO-MEMORY] A hit is what makes an entry recent, so
/// the file asked for most recently survives the next eviction and the
/// one nobody has touched is the one that goes.
#[test]
fn the_least_recently_asked_for_file_is_the_one_evicted() {
    let ids = files(3);
    let (Some(&first), Some(&second), Some(&third)) = (ids.first(), ids.get(1), ids.get(2)) else {
        return;
    };
    let mut store = TreeStore::default();
    store.insert(first, LARGE_FILE_BYTES, None);
    store.insert(second, LARGE_FILE_BYTES, None);
    assert!(holds(&mut store, first), "both files fit the budget");
    assert!(holds(&mut store, second), "both files fit the budget");

    // `first` is now the most recently used: it was asked for last.
    assert!(holds(&mut store, first), "asking for a file keeps it");
    store.insert(third, LARGE_FILE_BYTES, None);

    assert!(
        holds(&mut store, third),
        "the file just inserted must be resident — it is what the caller is about to walk"
    );
    assert!(
        holds(&mut store, first),
        "`first` was asked for after `second`, so `second` is the stale one and `first` \
         must survive; evicting the file a caller just used is how a bounded cache turns \
         into no cache at all"
    );
    assert!(
        !holds(&mut store, second),
        "`second` was the least recently used and the budget had no room for three"
    );
}

/// [PERF-FLUTTER-TODO-MEMORY] Re-inserting a file replaces its entry
/// rather than adding a second one.
///
/// Two workers racing the same uncached file both parse it and both
/// store it. Counting that file's bytes twice would shrink the budget
/// every time it happened, until a store sized for a working set held
/// almost nothing.
#[test]
fn storing_one_file_twice_does_not_charge_the_budget_twice() {
    let ids = files(2);
    let (Some(&raced), Some(&other)) = (ids.first(), ids.get(1)) else {
        return;
    };
    let mut store = TreeStore::default();
    store.insert(raced, LARGE_FILE_BYTES, None);
    store.insert(raced, LARGE_FILE_BYTES, None);
    store.insert(other, LARGE_FILE_BYTES, None);
    assert!(
        holds(&mut store, raced),
        "one file stored twice still covers {LARGE_FILE_BYTES} bytes, not twice that, so \
         the second file fits beside it"
    );
    assert!(holds(&mut store, other), "the second file must be resident");
}

/// [PERF-FLUTTER-TODO-MEMORY] A single file past the whole budget is
/// kept anyway: one very large file is a legitimate working set, and
/// evicting it would mean re-parsing it for every member of every
/// cluster it appears in.
#[test]
fn a_file_larger_than_the_whole_budget_is_still_kept() {
    let ids = files(2);
    let (Some(&giant), Some(&ordinary)) = (ids.first(), ids.get(1)) else {
        return;
    };
    let mut store = TreeStore::default();
    store.insert(ordinary, SMALL_FILE_BYTES, None);
    store.insert(
        giant,
        PARSE_TREE_SOURCE_BUDGET_BYTES.saturating_mul(2),
        None,
    );
    assert!(
        holds(&mut store, giant),
        "the file just parsed must be resident even when it alone exceeds the budget"
    );
    assert!(
        !holds(&mut store, ordinary),
        "everything older must have been evicted first — the budget is a real bound, and \
         only the entry that cannot be dropped stays"
    );
}

/// [PERF-FLUTTER-TODO-MEMORY] A cached "this file has no tree" verdict
/// is a cache hit. Re-deriving it costs a full parse attempt, which is
/// exactly what the store exists to avoid.
#[test]
fn a_file_with_no_tree_is_remembered_as_a_hit() {
    let ids = files(1);
    let Some(&unparseable) = ids.first() else {
        return;
    };
    let mut store = TreeStore::default();
    assert!(
        matches!(store.hit(unparseable), Lookup::Absent),
        "nothing is cached before anything is stored"
    );
    store.insert(unparseable, SMALL_FILE_BYTES, None);
    let recalled = store.hit(unparseable);
    assert!(
        matches!(recalled, Lookup::Remembered(_)),
        "a stored file must read back as cached, whatever the verdict"
    );
    assert!(
        matches!(recalled, Lookup::Remembered(None)),
        "the stored verdict was `no tree`, and that is what must come back"
    );
}
