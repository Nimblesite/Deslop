//! E2E pins for parse-store retention ([PIPELINE-INCREMENTAL-RETENTION]).
//!
//! A full pass owns the only exact knowledge of which blobs its corpus
//! can address, so retention runs there and nowhere else: stale
//! tool-version partitions are removed outright, orphans are *kept*
//! under budget because they are the content-addressed reuse set a
//! revert or branch switch full-hits
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]), and a pass that never
//! consults the store never touches it either
//! ([CONFIG-INCREMENTAL-OPTOUT]). Every scenario asserts the rendered
//! report against the seeded truth — retention must never move a
//! reported figure.

mod common;

use std::{fs, path::Path};

use serde_json::Value;

use crate::common::{incremental::*, seeded::*, store::*, Result};

/// Sweep summary event every store-on pass emits exactly once.
const SWEEP_LOG: &str = "fingerprint store swept";

/// A version directory no running binary addresses.
const STALE_VERSION_DIR: &str = "0.0.0-superseded";

/// Plants a fake superseded-version partition holding one junk blob
/// inside the seeded corpus's Rust store, returning its root.
fn plant_stale_partition(scan_root: &Path) -> Result<std::path::PathBuf> {
    let stale_root = store_dir(scan_root)
        .join("rust")
        .join(STALE_VERSION_DIR)
        .join(SEEDED_MIN_NODES.to_string());
    fs::create_dir_all(&stale_root)?;
    fs::write(stale_root.join("deadbeef.bin"), b"superseded-format blob")?;
    Ok(stale_root)
}

/// Rewrites `alpha.rs`'s banner comment with a byte-distinct,
/// same-length variant: the file's content hash changes while every
/// AST node, byte offset, line count, and therefore every reported
/// figure stays exactly the seeded truth.
fn edit_alpha_banner(scan_root: &Path, from: &str, to: &str) -> Result<()> {
    assert_eq!(
        from.len(),
        to.len(),
        "the banner edit must preserve byte offsets to keep the report pinned"
    );
    let path = scan_root.join("alpha.rs");
    let original = fs::read_to_string(&path)?;
    let edited = original.replacen(from, to, 1);
    assert_ne!(
        original, edited,
        "the banner edit must actually change alpha.rs — `{from}` not found"
    );
    fs::write(&path, edited)?;
    Ok(())
}

/// Asserts a report is byte-for-byte the seeded truth *and* its pass
/// counters match `(hits, misses)` — retention scenarios must never
/// move a reported figure.
fn assert_seeded_pass(
    report: &Value,
    events: &ReuseCounters,
    hits: u64,
    misses: u64,
    label: &str,
) -> Result<()> {
    assert_pass(report, events, hits, misses, label);
    assert_seeded_corpus(report, label)
}

// [PIPELINE-INCREMENTAL-RETENTION] A partition under a superseded tool
// version is unaddressable by construction. The next store-on pass
// removes it — and touches nothing else: the live blobs survive
// byte-identically and the report is still the seeded truth.
#[test]
fn a_stale_tool_version_partition_is_removed_by_the_next_full_pass() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, _cold, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_corpus, SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    let live_before = blob_bytes(&scan_root)?;
    let stale_root = plant_stale_partition(&scan_root)?;

    let out_dir = tmp.path().join("warm");
    let (warm, warm_events) = run_store_on(&scan_root, &out_dir, SEEDED_MIN_NODES, &[])?;

    assert_warm_pass(&warm, &warm_events, SEEDED_FILE_COUNT, "post-sweep warm");
    assert_seeded_corpus(&warm, "post-sweep warm")?;
    assert!(
        !stale_root.exists(),
        "the superseded version partition must be removed by the sweep: {}",
        stale_root.display()
    );
    assert_eq!(
        blob_bytes(&scan_root)?,
        live_before,
        "the sweep must leave every live blob byte-identical"
    );
    assert_log_mentions(&out_dir, SWEEP_LOG, 1, "post-sweep warm")?;
    assert_log_mentions(&out_dir, "stale_partitions=1", 1, "post-sweep warm")?;
    Ok(())
}

// [PIPELINE-INCREMENTAL-RETENTION] An edit strands the old content's
// blob as an orphan. Under budget the sweep keeps it — it is exactly
// the blob a revert re-addresses — so the revert pass full-hits the
// store and owes the cold report field for field
// ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
#[test]
fn an_edit_cycle_keeps_the_orphan_and_the_revert_full_hits() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, cold, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_corpus, SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    let original_blobs = blob_paths(&scan_root)?;
    assert_eq!(
        original_blobs.len(),
        3,
        "the seeded cold pass stores one blob per file: {original_blobs:?}"
    );

    edit_alpha_banner(&scan_root, "the canonical copy.", "the canonical copy!")?;
    let edit_out = tmp.path().join("edit");
    let (edited, edit_events) = run_store_on(&scan_root, &edit_out, SEEDED_MIN_NODES, &[])?;
    assert_seeded_pass(&edited, &edit_events, 2, 1, "edit pass")?;
    let after_edit = blob_paths(&scan_root)?;
    assert_eq!(
        after_edit.len(),
        4,
        "the edit adds one blob and the sweep keeps the orphan — it is \
         the revert-reuse set: {after_edit:?}"
    );
    assert!(
        original_blobs.iter().all(|blob| after_edit.contains(blob)),
        "every pre-edit blob must survive, the orphaned one included: \
         before {original_blobs:?} after {after_edit:?}"
    );
    assert_log_mentions(&edit_out, "orphan_blobs=1", 1, "edit pass")?;
    assert_log_mentions(&edit_out, "evicted_blobs=0", 1, "edit pass")?;

    edit_alpha_banner(&scan_root, "the canonical copy!", "the canonical copy.")?;
    let revert_out = tmp.path().join("revert");
    let (reverted, revert_events) = run_store_on(&scan_root, &revert_out, SEEDED_MIN_NODES, &[])?;
    assert_warm_pass(&reverted, &revert_events, SEEDED_FILE_COUNT, "revert pass");
    assert_seeded_corpus(&reverted, "revert pass")?;
    assert_reports_equal(&reverted, &cold, "revert pass vs original cold pass");
    Ok(())
}

// [CONFIG-INCREMENTAL-OPTOUT] A pass that never consults the store
// never sweeps it either: `--no-incremental` leaves a planted stale
// partition and every blob byte exactly where they were.
#[test]
fn a_disabled_store_pass_never_sweeps_the_store() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, _cold, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_corpus, SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    let stale_root = plant_stale_partition(&scan_root)?;
    let blobs_before = blob_bytes(&scan_root)?;

    let out_dir = tmp.path().join("disabled");
    let (bytes, events) =
        run_capturing_bytes(&scan_root, &out_dir, SEEDED_MIN_NODES, Store::Off, &[])?;
    let report: Value = serde_json::from_slice(&bytes)?;

    events.assert_store_disabled("disabled pass");
    assert_seeded_corpus(&report, "disabled pass")?;
    assert!(
        stale_root.exists(),
        "a disabled-store pass must not sweep: the planted stale partition \
         must survive at {}",
        stale_root.display()
    );
    assert_eq!(
        blob_bytes(&scan_root)?,
        blobs_before,
        "a disabled-store pass must leave every blob byte-identical"
    );
    assert_log_mentions(&out_dir, SWEEP_LOG, 0, "disabled pass")?;
    Ok(())
}
