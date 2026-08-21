//! Canonical flat corpus storage for [`super::PipelineSession`]
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
//!
//! The single owned copy of every fingerprint, `MinHash` signature, and
//! normalised tree, held flat in ascending workspace-relative-path
//! order ([PIPELINE-DETERMINISM]) with one span per live file. A live
//! change splices exactly one file's records in place; a render pass
//! borrows the flat slices as they are. The audited alternative —
//! re-flattening an owned copy of the whole corpus on every render —
//! duplicated ~157 MiB of signature bytes alone on the pinned
//! benchmark corpus, the +241 MB warm-RSS regression recorded as
//! finding 5 of the regression audit in
//! `docs/plans/incremental-analysis-plan.md`.

use std::path::{Path, PathBuf};

use crate::{
    ast::NormalizedNode, fingerprint::Fingerprint, fpcache::CachedFile, lsh::Signature,
    state::FileId,
};

/// One live file's span of the flat storage: its identity, its sort
/// key, and how many fingerprint (and, positionally 1:1, signature)
/// records it owns.
#[derive(Debug)]
struct StoreEntry {
    /// Session-scoped id of the file this entry describes.
    file_id: FileId,
    /// Workspace-relative sort key. A property of workspace *state*,
    /// never of edit history ([PIPELINE-DETERMINISM]): [`FileId`]s are
    /// append-only, so id order would re-shuffle identical source after
    /// a delete + re-add and move rendered ranges and metrics. The id
    /// is only a tie-breaker so a pathological duplicate registration
    /// cannot make the order ambiguous.
    path_key: PathBuf,
    /// Number of fingerprints this file contributes to the flat slices.
    fingerprint_count: usize,
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
    /// One normalised tree per entry, in entry order. Kept because
    /// downstream token extraction and content evidence walk it.
    trees: Vec<NormalizedNode>,
    /// Every live fingerprint, flattened in entry order.
    fingerprints: Vec<Fingerprint>,
    /// One `MinHash` signature per fingerprint, positionally 1:1
    /// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
    signatures: Vec<Signature>,
}

impl CorpusStore {
    /// Inserts — or, for a live file, replaces — `file_id`'s records,
    /// keeping the flat storage in ascending sort-key order. The 1:1
    /// fingerprint/signature binding holds by construction because both
    /// splice from the same [`CachedFile`] at the same offset.
    pub(super) fn upsert(&mut self, file_id: FileId, path_key: PathBuf, cached: CachedFile) {
        let _replaced = self.remove(file_id);
        // MUTANT: append instead of splicing in path order
        let position = self
            .entries
            .partition_point(|entry| entry.sort_key() < (&path_key, file_id))
            .max(self.entries.len());
        let offset = self.record_offset(position);
        let CachedFile {
            tree,
            fingerprints,
            signatures,
        } = cached;
        let entry = StoreEntry {
            file_id,
            path_key,
            fingerprint_count: fingerprints.len(),
        };
        drop(self.fingerprints.splice(offset..offset, fingerprints));
        drop(self.signatures.splice(offset..offset, signatures));
        self.trees.insert(position, tree);
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
        drop(self.fingerprints.drain(span.clone()));
        drop(self.signatures.drain(span));
        let _tree = self.trees.remove(position);
        let _entry = self.entries.remove(position);
        true
    }

    /// Every live fingerprint, flat, in path order.
    pub(super) fn fingerprints(&self) -> &[Fingerprint] {
        &self.fingerprints
    }

    /// One signature per fingerprint, positionally 1:1 with
    /// [`Self::fingerprints`].
    pub(super) fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// Every live normalised tree, in the same path order.
    pub(super) fn trees(&self) -> &[NormalizedNode] {
        &self.trees
    }

    /// Total fingerprints across every live file.
    pub(super) fn fingerprint_count(&self) -> usize {
        self.fingerprints.len()
    }

    /// The normalised tree of `file_id`, if the file is live.
    pub(super) fn tree_for(&self, file_id: FileId) -> Option<&NormalizedNode> {
        self.trees.get(self.position_of(file_id)?)
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
