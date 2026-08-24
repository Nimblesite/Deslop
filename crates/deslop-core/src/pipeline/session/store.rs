//! Canonical flat corpus storage for [`super::PipelineSession`]
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
//!
//! The single owned copy of every fingerprint, `MinHash` signature, and
//! normalised tree, held flat in ascending workspace-relative-path
//! order ([PIPELINE-DETERMINISM]) with one span per live file. A live
//! change splices exactly one file's records in place; a render pass
//! borrows the flat slices as they are. The audited alternative —
//! re-flattening an owned copy of the whole corpus on every render —
//! duplicated ~157 MiB of signature bytes alone on the pinned tokio
//! benchmark corpus, a +241 MB warm-RSS regression.
//!
//! The path order is not cosmetic: a render borrows these slices as
//! they are, so a splice that appends a changed file's records instead
//! of inserting them at the file's sort position renders its occurrence
//! — and the `summary` line built from it — in edit-arrival order.
//! Pinned by `deslop/tests/live_session_equivalence.rs`.

use std::path::{Path, PathBuf};

use crate::{
    fingerprint::Fingerprint, lsh::Signature,
    state::FileId,
};

/// One live file's span of the flat storage: its identity, its sort
/// key, and how many fingerprint (and, positionally 1:1, signature)
/// records it owns.
#[derive(Debug)]
pub(super) struct StoreEntry {
    /// Session-scoped id of the file this entry describes.
    pub(super) file_id: FileId,
    /// Workspace-relative sort key. A property of workspace *state*,
    /// never of edit history ([PIPELINE-DETERMINISM]): [`FileId`]s are
    /// append-only, so id order would re-shuffle identical source after
    /// a delete + re-add and move rendered ranges and metrics. The id
    /// is only a tie-breaker so a pathological duplicate registration
    /// cannot make the order ambiguous.
    pub(super) path_key: PathBuf,
    /// Number of fingerprints this file contributes to the flat slices.
    pub(super) fingerprint_count: usize,
}

impl StoreEntry {
    /// This entry's `(path key, id)` sort key.
    fn sort_key(&self) -> (&Path, FileId) {
        (&self.path_key, self.file_id)
    }
}

/// Flat, path-ordered corpus storage with per-file spans.
///
/// Entry presence here is the single source of truth for "which files
/// currently contribute fingerprints to the corpus."
#[derive(Debug, Default)]
pub(super) struct CorpusStore {
    /// Per-file spans in ascending [`StoreEntry::sort_key`] order.
    entries: Vec<StoreEntry>,
    /// Every live fingerprint, flattened in entry order.
    fingerprints: Vec<Fingerprint>,
    /// One `MinHash` signature per fingerprint, positionally 1:1
    /// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]), stored as contiguous
    /// segments — one per corpus-build shard, in shard (sorted-path)
    /// order — so a parallel build never merges the multi-GB
    /// population into one vector
    /// ([PERF-FLUTTER-TODO-MEMORY]).
    signatures: Vec<Vec<Signature>>,
}

impl CorpusStore {
    /// Takes the corpus loop's finished flat vectors as this store's
    /// body. The vectors are already in canonical order and exact-sized
    /// ([PERF-FLUTTER-TODO-MEMORY]); `files` presizes the entry list,
    /// which the change pass appends to.
    pub(super) fn from_flat_parts(
        entries: Vec<StoreEntry>,
        fingerprints: Vec<Fingerprint>,
        signatures: Vec<Vec<Signature>>,
    ) -> Self {
        Self {
            entries,
            fingerprints,
            signatures,
        }
    }

    /// Inserts — or, for a live file, replaces — `file_id`'s records,
    /// keeping the flat storage in ascending sort-key order. The 1:1
    /// fingerprint/signature binding holds by construction because both
    /// splice from the same [`CachedFile`] at the same offset.
    pub(super) fn upsert(
        &mut self,
        file_id: FileId,
        path_key: PathBuf,
        cached: crate::fpcache::CachedFile,
    ) {
        let _replaced = self.remove(file_id);
        let position = self
            .entries
            .partition_point(|entry| entry.sort_key() < (&path_key, file_id));
        let offset = self.record_offset(position);
        // The normalised tree is dropped here — the store holds no
        // trees ([PERF-FLUTTER-TODO-MEMORY]); measurement stages
        // re-materialise from sources.
        let crate::fpcache::CachedFile {
            tree: _tree,
            fingerprints,
            signatures,
        } = cached;
        let entry = StoreEntry {
            file_id,
            path_key,
            fingerprint_count: fingerprints.len(),
        };
        drop(self.fingerprints.splice(offset..offset, fingerprints));
        // An incremental upsert splices into the file's owning segment;
        // a file whose segment is unknown (first live edit on a
        // segment store) appends a fresh one in sort position.
        self.splice_signatures(offset, signatures);
        self.entries.insert(position, entry);
    }

    /// Removes `file_id`'s records, closing its span in every flat
    /// vector. Returns whether the file was present.
    pub(super) fn remove(&mut self, file_id: FileId) -> bool {
        let Some(position) = self.position_of(file_id) else {
            return false;
        };
        let offset = self.record_offset(position);
        let count = self
            .entries
            .get(position)
            .map_or(0, |entry| entry.fingerprint_count);
        let span = offset..offset.saturating_add(count);
        drop(self.fingerprints.drain(span));
        self.drain_signatures(offset, count);
        let _entry = self.entries.remove(position);
        true
    }

    /// Every live fingerprint, flat, in path order.
    pub(super) fn fingerprints(&self) -> &[Fingerprint] {
        &self.fingerprints
    }

    /// The signature population as an indexed view over the segments.
    /// The view borrows this store.
    pub(super) fn signatures(&self) -> crate::lsh::SignatureIndex<'_> {
        crate::lsh::SignatureIndex::from_segments(
            self.signatures.iter().map(std::vec::Vec::as_slice),
        )
    }

    /// Drops `count` signatures at flat `offset` — a contiguous run
    /// inside one segment, because a file's records never straddle
    /// segments.
    fn drain_signatures(&mut self, offset: usize, count: usize) {
        if let Some((segment, within)) = self.segment_mut_holding(offset) {
            let stop = within.saturating_add(count).min(segment.len());
            drop(segment.drain(within..stop));
        }
    }

    /// The segment containing flat `offset`, and the offset within it.
    /// A file's records are contiguous inside one segment — upserts and
    /// removals always land on file boundaries.
    fn segment_mut_holding(&mut self, offset: usize) -> Option<(&mut Vec<Signature>, usize)> {
        let mut cursor = 0_usize;
        for segment in &mut self.signatures {
            let end = cursor.saturating_add(segment.len());
            if offset < end {
                return Some((segment, offset.saturating_sub(cursor)));
            }
            cursor = end;
        }
        None
    }

    /// Total fingerprints across every live file.
    pub(super) fn fingerprint_count(&self) -> usize {
        self.fingerprints.len()
    }

    /// Splices a file's signatures at the flat `offset`. A file's
    /// records are contiguous inside one segment (upserts always land
    /// on file boundaries), so the owning segment splits, receives the
    /// records, and re-appends its tail — one pass, index space intact.
    fn splice_signatures(&mut self, offset: usize, incoming: Vec<Signature>) {
        if incoming.is_empty() {
            return;
        }
        match self.segment_mut_holding(offset) {
            Some((segment, within)) => {
                let tail = segment.split_off(within);
                segment.extend(incoming);
                segment.extend(tail);
            }
            // Past every segment: a brand-new file at the end of the
            // sort order takes a fresh segment.
            None => self.signatures.push(incoming),
        }
    }

    /// Entry position of `file_id`, if the file is live.
    fn position_of(&self, file_id: FileId) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.file_id == file_id)
    }

    /// Index of `position`'s first record in the flat vectors: the sum
    /// of every earlier entry's span. The append position — the common
    /// case when a full pass feeds files already in ascending order —
    /// is answered without walking the entries.
    fn record_offset(&self, position: usize) -> usize {
        if position == self.entries.len() {
            return self.fingerprints.len();
        }
        self.entries
            .iter()
            .take(position)
            .fold(0_usize, |total, entry| {
                total.saturating_add(entry.fingerprint_count)
            })
    }
}

/// Workspace-relative sort key for a live file ([PIPELINE-DETERMINISM]).
/// Falls back to the absolute path for a file outside the scan root —
/// a function of workspace state, never of registration history.
pub(super) fn relative_path_key(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod segment_tests {
    //! [PERF-FLUTTER-TODO-MEMORY] The signature population lives in
    //! per-shard segments; incremental upserts and removals splice and
    //! drain inside them. These pins mutate files at the beginning,
    //! middle, and end of a multi-segment population and assert the
    //! 1:1 fingerprint/signature positional alignment after every step
    //! (`docs/performance-branch-review.md`, "segmented-store remove/
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

    fn records(marker: u8, count: usize, file_id: FileId) -> (Vec<Fingerprint>, Vec<crate::lsh::Signature>) {
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
                    index, usize::MAX,
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
        store.upsert(id_at(&ids, 0)?, PathBuf::from("a.rs"), cached(marker_at(0), 3, id_at(&ids, 0)?));
        assert_eq!(store.fingerprint_count(), 7, "beginning file +1 record");
        assert_aligned(&store);

        // Middle: b.rs shrinks from two records to one.
        store.upsert(id_at(&ids, 1)?, PathBuf::from("b.rs"), cached(marker_at(1), 1, id_at(&ids, 1)?));
        assert_eq!(store.fingerprint_count(), 6, "middle file -1 record");
        assert_aligned(&store);

        // End: c.rs grows from two records to five.
        store.upsert(id_at(&ids, 2)?, PathBuf::from("c.rs"), cached(marker_at(2), 5, id_at(&ids, 2)?));
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
        assert_eq!(store.fingerprint_count(), 4, "beginning file's two records drained");
        assert_aligned(&store);

        assert!(store.remove(id_at(&ids, 1)?), "middle file present");
        assert_eq!(store.fingerprint_count(), 2, "middle file's two records drained");
        assert_aligned(&store);

        assert!(store.remove(id_at(&ids, 2)?), "end file present");
        assert_eq!(store.fingerprint_count(), 0, "end file's two records drained");
        assert_aligned(&store);

        assert!(!store.remove(id_at(&ids, 2)?), "a second removal finds nothing");
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
}
