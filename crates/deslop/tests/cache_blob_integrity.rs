//! The parse store must never serve bytes it cannot prove belong to
//! the address that selected them ([PIPELINE-INCREMENTAL-INTEGRITY]).
//!
//! Pins the blob-trust regressions from the incremental persistence
//! audit in `docs/plans/incremental-analysis-plan.md`:
//!
//! - a blob whose signature payload was corrupted decoded cleanly and
//!   was served as a valid hit, changing `token_jaccard` — a report
//!   difference bought by the cache, the exact thing
//!   [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] prohibits;
//! - a valid blob moved under another file's content address was served
//!   under it, swapping two files' reported spans;
//! - a blob with trailing garbage was accepted as a hit;
//! - malformed length fields drove allocations before any bounds check.
//!
//! The contract every scenario asserts: corruption or misplacement is a
//! *miss*, never a hit and never a crash. The miss re-parses from
//! source, overwrites the blob, and the very next pass hits cleanly —
//! and no pass over a damaged store may render anything but the truth
//! report.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::anyhow;

mod common;
use crate::common::{incremental::*, multilang::seed_twins, seeded::*, store::*, *};

/// Seeds the corpus and runs the store-filling cold pass, returning the
/// scan root, its tempdir, and the truth report every later pass owes.
fn cold_truth() -> Result<(tempfile::TempDir, PathBuf, serde_json::Value)> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, truth, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_corpus, SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    assert_seeded_corpus(&truth, "cold")?;
    Ok((tmp, scan_root, truth))
}

/// The `warn!` message every rejection path funnels through
/// ([PIPELINE-INCREMENTAL-INTEGRITY] failure modes).
const REJECTION_LOG: &str = "fingerprint cache entry rejected";

/// Rewrites every blob in the store through `mutate`, asserting the
/// damage actually landed — a mutation that silently produced identical
/// bytes would leave the scenario asserting nothing. Returns the damaged
/// bytes per blob so the caller can prove the heal overwrote them.
fn damage_every_blob(scan_root: &Path, mutate: impl Fn(&[u8]) -> Vec<u8>) -> Result<Vec<Vec<u8>>> {
    let mut damaged = Vec::new();
    for blob in blob_paths(scan_root)? {
        let original = fs::read(&blob)?;
        let corrupt = mutate(&original);
        anyhow::ensure!(
            corrupt != original,
            "mutation left {} byte-identical — the scenario would assert nothing",
            blob.display()
        );
        fs::write(&blob, &corrupt)?;
        damaged.push(corrupt);
    }
    anyhow::ensure!(!damaged.is_empty(), "no blob was damaged");
    Ok(damaged)
}

/// Asserts the healing pass replaced the damaged bytes on disk. A pass
/// that missed, re-parsed, and then failed to persist would still render
/// the truth and still hit on a later pass only by re-parsing every time
/// — indistinguishable from a heal by counters alone until the store is
/// read back.
fn assert_blobs_rewritten(scan_root: &Path, damaged: &[Vec<u8>], label: &str) -> Result<()> {
    let paths = blob_paths(scan_root)?;
    assert_eq!(
        paths.len(),
        damaged.len(),
        "{label}: the heal must rewrite blobs in place, not add or orphan any"
    );
    for (path, before) in paths.iter().zip(damaged) {
        assert_address_rewritten(path, before, label)?;
    }
    Ok(())
}

/// Asserts one address no longer holds the bytes that were rejected
/// there.
fn assert_address_rewritten(path: &Path, damaged: &[u8], label: &str) -> Result<()> {
    assert_ne!(
        fs::read(path)?,
        damaged,
        "{label}: {} still holds the bytes that were rejected — the miss \
         never persisted a replacement, so every later pass re-parses \
         while reporting a hit-free store forever",
        path.display()
    );
    Ok(())
}

/// Runs one store-on pass and asserts it healed the whole store — every
/// damaged blob missed, logged its rejection, and was rewritten — and
/// rendered the truth.
fn assert_heals_to_truth(
    scan_root: &Path,
    out_dir: &Path,
    truth: &serde_json::Value,
    misses: u64,
    label: &str,
) -> Result<()> {
    let (report, events) = run_store_on(scan_root, out_dir, SEEDED_MIN_NODES, &[])?;
    let hits = SEEDED_FILE_COUNT.saturating_sub(misses);
    assert_pass(&report, &events, hits, misses, label);
    assert_reports_equal(&report, truth, label);
    assert_seeded_corpus(&report, label)?;
    assert_log_mentions(out_dir, REJECTION_LOG, usize::try_from(misses)?, label)?;
    Ok(())
}

/// Runs one store-on pass over the untouched corpus and asserts it is
/// fully warm — the healing pass really did overwrite every damaged
/// blob — and still renders the truth.
fn assert_fully_warm_truth(
    scan_root: &Path,
    out_dir: &Path,
    truth: &serde_json::Value,
    files: u64,
    label: &str,
) -> Result<()> {
    let (warm, events) = run_store_on(scan_root, out_dir, SEEDED_MIN_NODES, &[])?;
    assert_warm_pass(&warm, &events, files, label);
    assert_reports_equal(&warm, truth, &format!("{label} vs truth"));
    Ok(())
}

// [PIPELINE-INCREMENTAL-INTEGRITY] Flipping the final byte of a blob —
// the tail of its last MinHash signature slot, leaving tree,
// fingerprints, counts and length untouched — must void the whole blob.
// Serving it would feed a corrupted signature straight into LSH and
// Jaccard scoring: the audit measured `token_jaccard` flipping from 0.0
// to 1.0, a false similarity the report presents as evidence.
#[test]
fn a_tampered_signature_payload_is_a_miss_that_self_heals() -> Result<()> {
    let (tmp, scan_root, truth) = cold_truth()?;

    // Flip only the final byte: the tail of the last signature slot,
    // leaving every length, count and tree byte exactly as written.
    let damaged = damage_every_blob(&scan_root, |bytes| {
        let mut flipped = bytes.to_vec();
        if let Some(last) = flipped.last_mut() {
            *last ^= 0xFF;
        }
        flipped
    })?;

    // Every tampered blob must miss, re-parse, and render the truth…
    assert_heals_to_truth(
        &scan_root,
        &tmp.path().join("tampered"),
        &truth,
        SEEDED_FILE_COUNT,
        "pass over tampered store",
    )?;
    // …the misses must have overwritten the damaged bytes on disk…
    assert_blobs_rewritten(&scan_root, &damaged, "tampered heal")?;
    // …so the next pass hits cleanly, with nothing left to reject.
    assert_fully_warm_truth(
        &scan_root,
        &tmp.path().join("healed"),
        &truth,
        SEEDED_FILE_COUNT,
        "healed pass",
    )
}

// [PIPELINE-INCREMENTAL-INTEGRITY] A valid blob moved under another
// file's content address must be served to neither file. The audit
// reproduced the swap serving both blobs as hits and exchanging the two
// files' reported spans — occurrences pointing at code the file does
// not contain at those offsets.
#[test]
fn a_blob_swapped_to_another_files_address_serves_neither() -> Result<()> {
    let (tmp, scan_root, truth) = cold_truth()?;

    let blobs = blob_paths(&scan_root)?;
    let [first, second, third] = blobs.as_slice() else {
        anyhow::bail!("three byte-distinct files must fill three blobs, got {blobs:?}");
    };
    let first_bytes = fs::read(first)?;
    let second_bytes = fs::read(second)?;
    let third_bytes = fs::read(third)?;
    assert_ne!(
        first_bytes, second_bytes,
        "the two swapped blobs must differ, or the swap is a no-op"
    );
    fs::write(first, &second_bytes)?;
    fs::write(second, &first_bytes)?;

    // The two swapped addresses miss and self-heal; the untouched third
    // blob still hits.
    assert_heals_to_truth(
        &scan_root,
        &tmp.path().join("swapped"),
        &truth,
        2,
        "pass over swapped store",
    )?;
    // Each swapped address must no longer hold the foreign blob it was
    // given; the untouched third address must be served, not rewritten.
    assert_address_rewritten(first, &second_bytes, "swap heal")?;
    assert_address_rewritten(second, &first_bytes, "swap heal")?;
    assert_eq!(
        fs::read(third)?,
        third_bytes,
        "the untouched third blob must be served, never rewritten — an \
         invalidation that swept past the two damaged addresses would show up \
         here as a rewritten neighbour"
    );
    assert_fully_warm_truth(
        &scan_root,
        &tmp.path().join("healed"),
        &truth,
        SEEDED_FILE_COUNT,
        "healed pass",
    )
}

// [PIPELINE-INCREMENTAL-INTEGRITY] The binding covers the language
// partition, not just the source hash. Two byte-identical files under
// two parsers share a blob *filename* (same content hash) in two
// partitions; copying the typescript blob over the javascript one must
// be a miss for the javascript file — serving it would hand the file a
// tree parsed under the wrong grammar.
#[test]
fn a_blob_copied_across_language_partitions_is_a_miss() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, truth, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_twins, SEEDED_MIN_NODES, 2)?;
    assert_eq!(
        cluster_count(&truth),
        0,
        "cross-language comparison is off by default ([CONFIG-CROSS-LANGUAGE]), \
         so the twins never pair — this scenario's assertions are the store \
         accounting and the on-disk blobs, deliberately not the clusters: {truth:#}"
    );

    let ts_blob = partition_blob(&scan_root, "typescript")?;
    let js_blob = partition_blob(&scan_root, "javascript")?;
    assert_eq!(
        ts_blob.file_name(),
        js_blob.file_name(),
        "byte-identical twins must share a content-hash filename"
    );
    let foreign = fs::read(&ts_blob)?;
    let displaced = fs::read(&js_blob)?;
    assert_ne!(
        foreign, displaced,
        "the two partitions' blobs must differ — identical bytes under two \
         grammars would make the copy a no-op and the test vacuous"
    );
    let _bytes_copied = fs::copy(&ts_blob, &js_blob)?;

    let crossed_out = tmp.path().join("crossed");
    let (report, events) = run_store_on(&scan_root, &crossed_out, SEEDED_MIN_NODES, &[])?;
    assert_pass(&report, &events, 1, 1, "cross-partition pass");
    assert_reports_equal(&report, &truth, "cross-partition pass vs truth");
    assert_log_mentions(&crossed_out, REJECTION_LOG, 1, "cross-partition pass")?;
    // The javascript address must be re-derived under its own grammar,
    // not left holding the typescript tree; the typescript blob it was
    // copied from is a valid hit and must survive untouched.
    assert_address_rewritten(&js_blob, &foreign, "cross-partition heal")?;
    assert_eq!(
        fs::read(&ts_blob)?,
        foreign,
        "the source partition's blob was never invalid and must be served \
         as-is, never rewritten"
    );

    assert_fully_warm_truth(
        &scan_root,
        &tmp.path().join("healed"),
        &truth,
        2,
        "healed pass",
    )
}

/// The single blob inside one language's store partition.
fn partition_blob(scan_root: &Path, language: &str) -> Result<PathBuf> {
    let blobs: Vec<PathBuf> = blob_paths(scan_root)?
        .into_iter()
        .filter(|path| path.components().any(|part| part.as_os_str() == language))
        .collect();
    match blobs.as_slice() {
        [only] => Ok(only.clone()),
        other => Err(anyhow!(
            "expected exactly one {language} blob, found {other:?}"
        )),
    }
}

/// One way of damaging a blob's byte shape.
type BlobMutation = fn(&[u8]) -> Vec<u8>;

// [PIPELINE-INCREMENTAL-INTEGRITY] Malformed blob shapes — truncation,
// trailing garbage, a corrupted interior — always degrade to a miss.
// Never a hit (the audit found trailing bytes accepted), never a panic
// (the audit produced a `capacity overflow` abort from a corrupt length
// field), and never anything but the truth report. Each mutation runs
// against a store the previous pass healed, so every scenario starts
// from three valid blobs.
#[test]
fn corrupt_blob_shapes_always_miss_and_never_crash() -> Result<()> {
    let (tmp, scan_root, truth) = cold_truth()?;

    let mutations: [(&str, BlobMutation); 3] = [
        ("truncated", |bytes| {
            bytes.get(..5).unwrap_or(bytes).to_vec()
        }),
        ("trailing garbage", |bytes| {
            let mut grown = bytes.to_vec();
            grown.extend_from_slice(&[0xAB; 64]);
            grown
        }),
        ("zeroed interior", |bytes| {
            bytes
                .iter()
                .enumerate()
                .map(|(index, byte)| if (4..68).contains(&index) { 0 } else { *byte })
                .collect()
        }),
    ];

    for (label, mutate) in mutations {
        let damaged = damage_every_blob(&scan_root, mutate)?;
        assert_heals_to_truth(
            &scan_root,
            &tmp.path().join(label.replace(' ', "-")),
            &truth,
            SEEDED_FILE_COUNT,
            &format!("pass over {label} store"),
        )?;
        assert_blobs_rewritten(&scan_root, &damaged, &format!("{label} heal"))?;
    }

    assert_fully_warm_truth(
        &scan_root,
        &tmp.path().join("final-warm"),
        &truth,
        SEEDED_FILE_COUNT,
        "final warm pass",
    )
}
