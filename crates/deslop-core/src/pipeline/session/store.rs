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
        if count == 0 {
            return;
        }
        let mut cursor = 0_usize;
        for segment in &mut self.signatures {
            let end = cursor.saturating_add(segment.len());
            if offset < end {
                let within = offset.saturating_sub(cursor);
                let stop = within.saturating_add(count).min(segment.len());
                drop(segment.drain(within..stop));
                return;
            }
            cursor = end;
        }
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
        let mut cursor = 0_usize;
        for segment in &mut self.signatures {
            let end = cursor.saturating_add(segment.len());
            if offset <= end {
                let within = offset.saturating_sub(cursor).min(segment.len());
                let tail = segment.split_off(within);
                segment.extend(incoming);
                segment.extend(tail);
                return;
            }
            cursor = end;
        }
        self.signatures.push(incoming);
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
