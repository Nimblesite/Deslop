//! E2E pins for parse-store retention ([PIPELINE-INCREMENTAL-RETENTION]).
//!
//! A full pass owns the only exact knowledge of which blobs its corpus
//! can address, so retention runs there and nowhere else — and under
//! budget it deletes *nothing*: orphans are the content-addressed reuse
//! set a revert or branch switch full-hits
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]), and another tool
//! version's partition may belong to a second binary still running
//! against the same workspace (an installed VSIX's LSP beside a
//! freshly-built CLI), which two mutually-sweeping binaries would
//! deadlock into permanent rebuild churn. A pass that never consults the
//! store never touches it either ([CONFIG-INCREMENTAL-OPTOUT]). Every
//! scenario asserts the rendered report against the seeded truth —
//! retention must never move a reported figure.

use std::{fs, path::Path};

use serde_json::Value;

use crate::common::{incremental::*, seeded::*, store::*, Result};

/// Sweep summary event every store-on pass emits exactly once.
const SWEEP_LOG: &str = "fingerprint store swept";

/// A version directory this binary cannot address — the partition a
/// differently-versioned binary sharing the workspace writes.
const OTHER_VERSION_DIR: &str = "0.0.0-superseded";

/// Blob file the planted other-version partition holds.
const OTHER_VERSION_BLOB: &str = "deadbeef.bin";

/// The seeded file whose banner comment the edit cycle rewrites.
const ALPHA: &str = "alpha.rs";

/// Plants an other-tool-version partition holding one blob inside the
/// seeded corpus's Rust store, returning its root.
fn plant_other_version_partition(scan_root: &Path) -> Result<std::path::PathBuf> {
    let other_root = store_dir(scan_root)
        .join("rust")
        .join(OTHER_VERSION_DIR)
        .join(SEEDED_MIN_NODES.to_string());
    fs::create_dir_all(&other_root)?;
    fs::write(other_root.join(OTHER_VERSION_BLOB), b"other-version blob")?;
    Ok(other_root)
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

// [PIPELINE-INCREMENTAL-RETENTION] A partition under another tool
// version is unaddressable by *this* binary — but a second binary
// sharing the workspace may own it, so an under-budget sweep classifies
// it and deletes nothing. The blob keeps its exact bytes, is never
// served (the pass still full-hits its own partition), the live blobs
// survive byte-identically, and the report is still the seeded truth.
#[test]
fn another_tool_versions_partition_survives_an_under_budget_sweep() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, _cold, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_corpus, SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    let live_before = blob_bytes(&scan_root)?;
    let other_root = plant_other_version_partition(&scan_root)?;
    let other_blob = other_root.join(OTHER_VERSION_BLOB);
    let other_bytes = fs::read(&other_blob)?;

    let out_dir = tmp.path().join("warm");
    let (warm, warm_events) = run_store_on(&scan_root, &out_dir, SEEDED_MIN_NODES, &[])?;

    assert_warm_pass(&warm, &warm_events, SEEDED_FILE_COUNT, "post-sweep warm");
    assert_seeded_corpus(&warm, "post-sweep warm")?;
    assert!(
        other_root.is_dir(),
        "another binary's version partition must survive an under-budget sweep: {}",
        other_root.display()
    );
    assert_eq!(
        fs::read(&other_blob).ok(),
        Some(other_bytes.clone()),
        "the other version's blob must keep its exact bytes: {}",
        other_blob.display()
    );
    let mut expected = live_before;
    expected.push(other_bytes);
    expected.sort();
    let mut after = blob_bytes(&scan_root)?;
    after.sort();
    assert_eq!(
        after, expected,
        "the sweep must leave the store exactly as it found it — every live \
         blob byte-identical and the other version's blob still present"
    );
    assert_log_mentions(&out_dir, SWEEP_LOG, 1, "post-sweep warm")?;
    assert_log_mentions(&out_dir, "other_version_blobs=1", 1, "post-sweep warm")?;
    assert_log_mentions(&out_dir, "evicted_blobs=0", 1, "post-sweep warm")?;
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

    edit_preserving_offsets(
        &scan_root,
        ALPHA,
        "the canonical copy.",
        "the canonical copy!",
    )?;
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

    edit_preserving_offsets(
        &scan_root,
        ALPHA,
        "the canonical copy!",
        "the canonical copy.",
    )?;
    let revert_out = tmp.path().join("revert");
    let (reverted, revert_events) = run_store_on(&scan_root, &revert_out, SEEDED_MIN_NODES, &[])?;
    assert_warm_pass(&reverted, &revert_events, SEEDED_FILE_COUNT, "revert pass");
    assert_seeded_corpus(&reverted, "revert pass")?;
    assert_reports_equal(&reverted, &cold, "revert pass vs original cold pass");
    Ok(())
}

// [CONFIG-INCREMENTAL-OPTOUT] A pass that never consults the store
// never sweeps it either: `--no-incremental` leaves a planted
// other-version partition and every blob byte exactly where they were.
#[test]
fn a_disabled_store_pass_never_sweeps_the_store() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, _cold, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_corpus, SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    let other_root = plant_other_version_partition(&scan_root)?;
    let blobs_before = blob_bytes(&scan_root)?;

    let out_dir = tmp.path().join("disabled");
    let (bytes, events) =
        run_capturing_bytes(&scan_root, &out_dir, SEEDED_MIN_NODES, Store::Off, &[])?;
    let report: Value = serde_json::from_slice(&bytes)?;

    events.assert_store_disabled("disabled pass");
    assert_seeded_corpus(&report, "disabled pass")?;
    assert!(
        other_root.exists(),
        "a disabled-store pass must not sweep: the planted other-version \
         partition must survive at {}",
        other_root.display()
    );
    assert_eq!(
        blob_bytes(&scan_root)?,
        blobs_before,
        "a disabled-store pass must leave every blob byte-identical"
    );
    assert_log_mentions(&out_dir, SWEEP_LOG, 0, "disabled pass")?;
    Ok(())
}
