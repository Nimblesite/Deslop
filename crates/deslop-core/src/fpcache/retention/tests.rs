//! Unit tests for store retention ([PIPELINE-INCREMENTAL-RETENTION]):
//! stale tool-version partitions always go, orphans are kept under
//! budget (they are the revert-reuse set), and budget pressure evicts
//! orphans first, then oldest, deterministically.

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

use super::*;

/// Language partition used throughout.
const LANGUAGE: &str = "rust";

/// Subtree-size floor of the current partition.
const MIN_NODES: u32 = 8;

/// Creates `<base>/fingerprints/<language>/<version>/<min>` and
/// returns it.
fn partition(base: &Path, language: &str, version: &str, min_nodes: u32) -> io::Result<PathBuf> {
    let dir = base
        .join(FINGERPRINT_DIR)
        .join(language)
        .join(version)
        .join(min_nodes.to_string());
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Writes a `len`-byte blob for `source` into `dir`, its mtime pinned
/// to `UNIX_EPOCH + age_seconds` — deterministic age without sleeping.
fn write_blob(dir: &Path, source: &[u8], len: usize, age_seconds: u64) -> io::Result<PathBuf> {
    let path = dir.join(blob_file_name(&bytes_hash(source)));
    fs::write(&path, vec![0_u8; len])?;
    let stamp = UNIX_EPOCH
        .checked_add(Duration::from_secs(age_seconds))
        .unwrap_or(UNIX_EPOCH);
    fs::File::options()
        .write(true)
        .open(&path)?
        .set_modified(stamp)?;
    Ok(path)
}

/// A [`LiveBlobs`] holding exactly the given sources under [`LANGUAGE`].
fn live_with(sources: &[&[u8]]) -> LiveBlobs {
    let mut live = LiveBlobs::default();
    for source in sources {
        live.record(LANGUAGE, source);
    }
    live
}

// [PIPELINE-INCREMENTAL-RETENTION] A partition under another tool
// version is unaddressable by construction — always removed. The
// current version's partition and everything in it survives.
#[test]
fn stale_tool_version_partitions_are_removed_and_the_current_one_kept() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let stale = partition(tmp.path(), LANGUAGE, "0.0.0-superseded", MIN_NODES)?;
    let stale_blob = write_blob(&stale, b"fn dead() {}", 64, 1_000)?;
    let current = partition(tmp.path(), LANGUAGE, TOOL_VERSION, MIN_NODES)?;
    let live_source: &[u8] = b"fn live() {}";
    let live_blob = write_blob(&current, live_source, 64, 1_000)?;

    sweep_store(tmp.path(), &live_with(&[live_source]), MIN_NODES);

    assert!(
        !stale_blob.exists() && !stale.exists(),
        "the superseded version partition and its blob must be removed"
    );
    assert!(
        live_blob.exists(),
        "the current partition's live blob must survive the sweep"
    );
    Ok(())
}

// [PIPELINE-INCREMENTAL-RETENTION] Under budget, an orphan is the
// content-addressed reuse set for a revert or branch switch
// ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] asserts a revert
// full-hits), so the sweep must keep it.
#[test]
fn orphans_survive_a_sweep_while_the_store_is_under_budget() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let current = partition(tmp.path(), LANGUAGE, TOOL_VERSION, MIN_NODES)?;
    let live_source: &[u8] = b"fn live() {}";
    let live_blob = write_blob(&current, live_source, 64, 2_000)?;
    let orphan_blob = write_blob(&current, b"fn old() {}", 64, 1_000)?;

    sweep_store(tmp.path(), &live_with(&[live_source]), MIN_NODES);

    assert!(
        live_blob.exists() && orphan_blob.exists(),
        "both the live blob and the revert-reuse orphan must survive under budget"
    );
    Ok(())
}

// [PIPELINE-INCREMENTAL-RETENTION] Only `.bin` blobs are retention's
// to manage — foreign files are never inventoried and never touched.
#[test]
fn foreign_files_are_never_touched_by_the_sweep_or_the_budget() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let current = partition(tmp.path(), LANGUAGE, TOOL_VERSION, MIN_NODES)?;
    let foreign = current.join("README.txt");
    fs::write(&foreign, b"not a blob")?;
    let inventory = blob_inventory(
        &tmp.path().join(FINGERPRINT_DIR),
        &live_with(&[]),
        MIN_NODES,
    );

    sweep_store(tmp.path(), &live_with(&[]), MIN_NODES);

    assert!(
        inventory.is_empty(),
        "a non-`.bin` file must never enter the eviction inventory: {inventory:?}"
    );
    assert!(foreign.exists(), "the foreign file must survive the sweep");
    Ok(())
}

// [PIPELINE-INCREMENTAL-RETENTION] Over budget: provable orphans are
// evicted before any live blob, oldest first, and eviction stops the
// moment the store fits.
#[test]
fn budget_pressure_evicts_orphans_first_then_oldest_and_stops_at_the_budget() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let current = partition(tmp.path(), LANGUAGE, TOOL_VERSION, MIN_NODES)?;
    let live_source: &[u8] = b"fn live() {}";
    let live_old = write_blob(&current, live_source, 100, 100)?;
    let orphan_new = write_blob(&current, b"fn a() {}", 100, 9_000)?;
    let orphan_old = write_blob(&current, b"fn b() {}", 100, 200)?;
    let root = tmp.path().join(FINGERPRINT_DIR);
    let live = live_with(&[live_source]);

    let evicted = enforce_budget(blob_inventory(&root, &live, MIN_NODES), 300, 200);

    assert_eq!(evicted, 1, "shedding exactly one blob reaches the budget");
    assert!(
        !orphan_old.exists(),
        "the oldest orphan must be the first eviction"
    );
    assert!(
        orphan_new.exists() && live_old.exists(),
        "the newer orphan and the older *live* blob must both survive — \
         orphan class outranks age"
    );
    Ok(())
}

// [PIPELINE-INCREMENTAL-RETENTION] With every orphan gone, budget
// pressure falls back to oldest-first over live blobs. Evicting a live
// blob is safe — the next pass misses and self-heals — so the budget
// is a hard bound.
#[test]
fn budget_pressure_falls_back_to_the_oldest_live_blob_once_orphans_are_gone() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let current = partition(tmp.path(), LANGUAGE, TOOL_VERSION, MIN_NODES)?;
    let old_source: &[u8] = b"fn old_live() {}";
    let new_source: &[u8] = b"fn new_live() {}";
    let live_old = write_blob(&current, old_source, 100, 100)?;
    let live_new = write_blob(&current, new_source, 100, 9_000)?;
    let root = tmp.path().join(FINGERPRINT_DIR);
    let live = live_with(&[old_source, new_source]);

    let evicted = enforce_budget(blob_inventory(&root, &live, MIN_NODES), 200, 100);

    assert_eq!(evicted, 1, "shedding exactly one blob reaches the budget");
    assert!(
        !live_old.exists() && live_new.exists(),
        "the older live blob is evicted; the newer one survives"
    );
    Ok(())
}

// [PIPELINE-INCREMENTAL-RETENTION] A blob under another `min_nodes`
// partition may still be addressed by a different invocation, so it is
// never a provable orphan — only age-ranked under pressure.
#[test]
fn another_min_nodes_partition_is_age_ranked_never_provably_orphaned() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let other = partition(tmp.path(), LANGUAGE, TOOL_VERSION, 20)?;
    let other_blob = write_blob(&other, b"fn other() {}", 64, 500)?;
    let root = tmp.path().join(FINGERPRINT_DIR);
    let live = live_with(&[]);

    let inventory = blob_inventory(&root, &live, MIN_NODES);
    sweep_store(tmp.path(), &live, MIN_NODES);

    assert_eq!(
        inventory
            .iter()
            .map(|record| record.orphan)
            .collect::<Vec<_>>(),
        vec![false],
        "the other-partition blob must be inventoried as non-orphan: {inventory:?}"
    );
    assert!(
        other_blob.exists(),
        "an under-budget sweep must keep the other partition's blob"
    );
    Ok(())
}
