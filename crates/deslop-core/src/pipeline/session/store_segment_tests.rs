//! [PERF-FLUTTER-TODO-MEMORY] The signature population lives in
//! per-shard segments; incremental upserts and removals splice and
//! drain inside them. These pins mutate files at the beginning,
//! middle, and end of a multi-segment population and assert the
//! 1:1 fingerprint/signature positional alignment after every step
//! (`docs/release-audit.md`, "segmented-store remove/
//! upsert logic has no changed test").

use std::path::PathBuf;

use super::{CorpusStore, StoreEntry};
use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    state::{FileId, FileRegistry},
};

/// Each file's records carry its marker byte in both axes: the
/// fingerprint hash and every signature slot. Alignment is then
/// observable as "the signature at i carries the marker of the
/// fingerprint at i".
const MARKERS: [u8; 4] = *b"abcd";

/// The marker byte at `index`, falling back to the last marker for
/// out-of-range indexes (never happens inside the fixtures).
fn marker_at(index: usize) -> u8 {
    MARKERS.get(index).copied().unwrap_or(b'z')
}

/// The id at `index` of a fixture id list. The fixtures always
/// register enough files, so a miss is a fixture bug, not a
/// verdict.
fn id_at(ids: &[FileId], index: usize) -> Result<FileId, String> {
    ids.get(index)
        .copied()
        .ok_or_else(|| format!("fixture id {index} missing"))
}

fn registry_files(count: usize) -> (FileRegistry, Vec<FileId>) {
    let mut registry = FileRegistry::new();
    let ids = (0..count)
        .map(|index| {
            registry.register(PathBuf::from(format!(
                "{}.rs",
                char::from(marker_at(index))
            )))
        })
        .collect();
    (registry, ids)
}

fn records(
    marker: u8,
    count: usize,
    file_id: FileId,
) -> (Vec<Fingerprint>, Vec<crate::lsh::Signature>) {
    let fingerprints = (0..count)
        .map(|index| Fingerprint {
            hash: [marker; 32],
            file_id,
            byte_range: ByteRange {
                start: index.saturating_mul(10),
                end: index.saturating_mul(10).saturating_add(5),
            },
            node_count: 30,
        })
        .collect();
    let signatures = (0..count)
        .map(|_| [u64::from(marker) << 8; crate::lsh::SIGNATURE_LEN])
        .collect();
    (fingerprints, signatures)
}

fn cached(marker: u8, count: usize, file_id: FileId) -> crate::fpcache::CachedFile {
    let (fingerprints, signatures) = records(marker, count, file_id);
    crate::fpcache::CachedFile {
        tree: NormalizedNode {
            kind: "file",
            children: Vec::new(),
            byte_range: ByteRange { start: 0, end: 5 },
            file_id,
        },
        fingerprints,
        signatures,
    }
}

fn three_segment_store() -> (CorpusStore, Vec<FileId>) {
    let (_registry, ids) = registry_files(MARKERS.len().saturating_sub(1));
    let mut fingerprints = Vec::new();
    let mut signatures = Vec::new();
    let mut entries = Vec::new();
    for (index, &file_id) in ids.iter().enumerate() {
        let (file_fingerprints, file_signatures) = records(marker_at(index), 2, file_id);
        fingerprints.extend(file_fingerprints.clone());
        signatures.push(file_signatures);
        entries.push(StoreEntry {
            file_id,
            path_key: PathBuf::from(format!("{}.rs", char::from(marker_at(index)))),
            fingerprint_count: file_fingerprints.len(),
        });
    }
    (
        CorpusStore::from_flat_parts(entries, fingerprints, signatures),
        ids,
    )
}

/// The alignment invariant: the signature view is positionally 1:1
/// with the fingerprints — same length, and the signature at i
/// carries the marker byte of the fingerprint at i.
fn assert_aligned(store: &CorpusStore) {
    let fingerprints = store.fingerprints();
    let signatures = store.signatures();
    assert_eq!(
        signatures.len(),
        fingerprints.len(),
        "view length must equal the fingerprint population"
    );
    for index in 0..fingerprints.len() {
        let Some(fingerprint) = fingerprints.get(index) else {
            continue;
        };
        let Some(signature) = signatures.get(index) else {
            // The deliberate-failure pattern: an assertion message
            // with mismatched operands, never a raw panic.
            assert_eq!(
                index,
                usize::MAX,
                "index {index}: signature missing while fingerprint exists"
            );
            continue;
        };
        let expected_marker = u64::from(fingerprint.hash[0]) << 8;
        assert_eq!(
            signature[0], expected_marker,
            "index {index}: signature marker {:x} must match fingerprint marker {:x}",
            signature[0], expected_marker
        );
    }
}

/// Replacing a file at the beginning, middle, and end of a
/// multi-segment population — with different record counts, so the
/// splices grow and shrink inside segments — keeps every remaining
/// fingerprint aligned with its signature.
#[test]
fn upserts_across_a_multi_segment_population_stay_aligned() -> Result<(), String> {
    let (store, ids) = three_segment_store();
    let mut store = store;
    assert_aligned(&store);
    assert_eq!(store.fingerprint_count(), 6, "three files of two records");

    // Beginning: a.rs grows from two records to three.
    store.upsert(
        id_at(&ids, 0)?,
        PathBuf::from("a.rs"),
        cached(marker_at(0), 3, id_at(&ids, 0)?),
    );
    assert_eq!(store.fingerprint_count(), 7, "beginning file +1 record");
    assert_aligned(&store);

    // Middle: b.rs shrinks from two records to one.
    store.upsert(
        id_at(&ids, 1)?,
        PathBuf::from("b.rs"),
        cached(marker_at(1), 1, id_at(&ids, 1)?),
    );
    assert_eq!(store.fingerprint_count(), 6, "middle file -1 record");
    assert_aligned(&store);

    // End: c.rs grows from two records to five.
    store.upsert(
        id_at(&ids, 2)?,
        PathBuf::from("c.rs"),
        cached(marker_at(2), 5, id_at(&ids, 2)?),
    );
    assert_eq!(store.fingerprint_count(), 9, "end file +3 records");
    assert_aligned(&store);
    Ok(())
}

/// Removing a file at each position drains exactly its records and
/// leaves the surviving population aligned.
#[test]
fn removals_across_a_multi_segment_population_stay_aligned() -> Result<(), String> {
    let (store, ids) = three_segment_store();
    let mut store = store;

    assert!(store.remove(id_at(&ids, 0)?), "beginning file present");
    assert_eq!(
        store.fingerprint_count(),
        4,
        "beginning file's two records drained"
    );
    assert_aligned(&store);

    assert!(store.remove(id_at(&ids, 1)?), "middle file present");
    assert_eq!(
        store.fingerprint_count(),
        2,
        "middle file's two records drained"
    );
    assert_aligned(&store);

    assert!(store.remove(id_at(&ids, 2)?), "end file present");
    assert_eq!(
        store.fingerprint_count(),
        0,
        "end file's two records drained"
    );
    assert_aligned(&store);

    assert!(
        !store.remove(id_at(&ids, 2)?),
        "a second removal finds nothing"
    );
    Ok(())
}

/// A file that never existed splices into sort position between
/// live segments — the mid-segment split path.
#[test]
fn a_new_file_between_segments_splices_mid_population() -> Result<(), String> {
    let (store, ids) = three_segment_store();
    let mut store = store;
    let (mut registry, _existing) = registry_files(3);
    let new_id = registry.register(PathBuf::from("bb.rs"));
    assert_ne!(new_id, id_at(&ids, 0)?, "fresh id for the new file");

    store.upsert(new_id, PathBuf::from("bb.rs"), cached(b'd', 4, new_id));
    assert_eq!(store.fingerprint_count(), 10, "new file adds four records");
    assert_aligned(&store);

    // Removing it again restores the original population exactly.
    assert!(store.remove(new_id));
    assert_eq!(store.fingerprint_count(), 6);
    assert_aligned(&store);
    Ok(())
}
