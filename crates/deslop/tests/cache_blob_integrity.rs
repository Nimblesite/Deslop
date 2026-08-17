//! The parse store must never serve bytes it cannot prove belong to
//! the address that selected them ([PIPELINE-INCREMENTAL-INTEGRITY]).
//!
//! Pins the blob-trust regressions from the incremental persistence
//! audit (`docs/incremental-persistence-regression-audit.md`):
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
use crate::common::{incremental::*, multilang::seed_twins, seeded::*, *};

/// Every `.bin` blob under the scan root's fingerprint store, sorted so
/// scenarios pick blobs deterministically.
fn blob_paths(scan_root: &Path) -> Result<Vec<PathBuf>> {
    let store = scan_root.join(".deslop/cache/fingerprints");
    let mut found = Vec::new();
    collect_blobs(&store, &mut found)?;
    found.sort();
    anyhow::ensure!(
        !found.is_empty(),
        "no blobs under {} — the cold pass did not fill the store",
        store.display()
    );
    Ok(found)
}

/// Recursively collects `.bin` files under `dir` into `found`.
fn collect_blobs(dir: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_blobs(&path, found)?;
        } else if path.extension().is_some_and(|ext| ext == "bin") {
            found.push(path);
        }
    }
    Ok(())
}

/// Seeds the corpus and runs the store-filling cold pass, returning the
/// scan root, its tempdir, and the truth report every later pass owes.
fn cold_truth() -> Result<(tempfile::TempDir, PathBuf, serde_json::Value)> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, truth, _cold_events) =
        seeded_cold_pass(tmp.path(), seed_corpus, SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    assert_seeded_corpus(&truth, "cold")?;
    Ok((tmp, scan_root, truth))
}

/// Runs one store-on pass and asserts it healed the whole store — every
/// damaged blob missed and was rewritten — and rendered the truth.
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

    for blob in blob_paths(&scan_root)? {
        let mut bytes = fs::read(&blob)?;
        let last = bytes
            .last_mut()
            .ok_or_else(|| anyhow!("empty blob at {}", blob.display()))?;
        *last ^= 0xFF;
        fs::write(&blob, bytes)?;
    }

    // Every tampered blob must miss, re-parse, and render the truth…
    assert_heals_to_truth(
        &scan_root,
        &tmp.path().join("tampered"),
        &truth,
        SEEDED_FILE_COUNT,
        "pass over tampered store",
    )?;
    // …and the misses must have overwritten the blobs, so the next pass
    // hits cleanly.
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
    let [first, second, _third] = blobs.as_slice() else {
        anyhow::bail!("three byte-distinct files must fill three blobs, got {blobs:?}");
    };
    let first_bytes = fs::read(first)?;
    let second_bytes = fs::read(second)?;
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

    let ts_blob = partition_blob(&scan_root, "typescript")?;
    let js_blob = partition_blob(&scan_root, "javascript")?;
    assert_eq!(
        ts_blob.file_name(),
        js_blob.file_name(),
        "byte-identical twins must share a content-hash filename"
    );
    let _bytes_copied = fs::copy(&ts_blob, &js_blob)?;

    let (report, events) = run_store_on(
        &scan_root,
        &tmp.path().join("crossed"),
        SEEDED_MIN_NODES,
        &[],
    )?;
    assert_pass(&report, &events, 1, 1, "cross-partition pass");
    assert_reports_equal(&report, &truth, "cross-partition pass vs truth");

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
        for blob in blob_paths(&scan_root)? {
            fs::write(&blob, mutate(&fs::read(&blob)?))?;
        }
        assert_heals_to_truth(
            &scan_root,
            &tmp.path().join(label.replace(' ', "-")),
            &truth,
            SEEDED_FILE_COUNT,
            &format!("pass over {label} store"),
        )?;
    }

    assert_fully_warm_truth(
        &scan_root,
        &tmp.path().join("final-warm"),
        &truth,
        SEEDED_FILE_COUNT,
        "final warm pass",
    )
}
