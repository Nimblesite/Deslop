//! Bounded per-range memoisation for [`ParseCache`]
//! ([PERF-FLUTTER-TODO-CORPUS], [PERF-FLUTTER-TODO-MEMORY]): the capped
//! get-or-compute cells behind the filter queries a corpus-scale report
//! asks repeatedly per member range. Split from the parent module,
//! which owns the cache type and the CST/LRU machinery.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::super::body_shape::OwnedShapeToken;
use super::super::{calls::CallShape, polymorphic::OwnedSubject};
use super::{locked, CallSequence, ParseCache, Snippet, SnippetKey};

/// Most cells each cache may retain. Beyond the cap a value is
/// recomputed on demand — results are identical, only reuse ends, so
/// the resident cost of memoisation stays bounded on corpus-scale runs
/// ([PERF-FLUTTER-TODO-MEMORY]).
const CALL_SHAPE_MEMO_MAX: usize = 131_072;
/// Cap for the call-sequence cells.
const CALL_SEQUENCE_MEMO_MAX: usize = 65_536;
/// Cap for the signature-only body streams.
const SIGNATURE_SHAPE_MEMO_MAX: usize = 65_536;
/// Cap for the polymorphic subject cells. Cells are small (a digest,
/// a name, bases), so the cap is set against the corpus member
/// population rather than a fraction of it.
const SUBJECT_MEMO_MAX: usize = 524_288;
/// Cap for the function body-digest cells.
const BODY_DIGEST_MEMO_MAX: usize = 262_144;

/// Shared get-or-compute for one memo map: returns the memoised value,
/// computing and (within the cap) storing it on a miss. `None` results
/// are stored too — "no enclosing call" is as expensive to rederive as
/// the value itself.
fn memo_entry<T>(
    map: &Mutex<HashMap<SnippetKey, Option<Arc<T>>>>,
    key: SnippetKey,
    cap: usize,
    compute: impl FnOnce() -> Option<T>,
) -> Option<Arc<T>> {
    if let Some(hit) = locked(map).get(&key) {
        return hit.clone();
    }
    let computed = compute().map(Arc::new);
    let mut map = locked(map);
    if map.len() < cap {
        let _replaced = map.insert(key, computed.clone());
    }
    computed
}

impl ParseCache {
    /// The enclosing-call [`CallShape`] for `snippet`'s range,
    /// memoised ([PERF-FLUTTER-TODO-CORPUS]).
    pub(crate) fn call_shape(
        &self,
        snippet: &Snippet<'_>,
        compute: impl FnOnce() -> Option<CallShape>,
    ) -> Option<Arc<CallShape>> {
        memo_entry(
            &self.call_shapes,
            (snippet.file_id, snippet.range.start, snippet.range.end),
            CALL_SHAPE_MEMO_MAX,
            compute,
        )
    }

    /// The fused call-sequence cell for `snippet`'s range, memoised
    /// ([PERF-FLUTTER-TODO-CORPUS]).
    pub(crate) fn call_sequence(
        &self,
        snippet: &Snippet<'_>,
        compute: impl FnOnce() -> Option<CallSequence>,
    ) -> Option<Arc<CallSequence>> {
        memo_entry(
            &self.call_sequences,
            (snippet.file_id, snippet.range.start, snippet.range.end),
            CALL_SEQUENCE_MEMO_MAX,
            compute,
        )
    }

    /// The signature-only body stream for `snippet`'s range, memoised
    /// ([PERF-FLUTTER-TODO-CORPUS]).
    pub(crate) fn signature_shape(
        &self,
        snippet: &Snippet<'_>,
        compute: impl FnOnce() -> Option<Vec<OwnedShapeToken>>,
    ) -> Option<Arc<Vec<OwnedShapeToken>>> {
        memo_entry(
            &self.signature_shapes,
            (snippet.file_id, snippet.range.start, snippet.range.end),
            SIGNATURE_SHAPE_MEMO_MAX,
            compute,
        )
    }

    /// The polymorphic subject for `snippet`'s range, memoised
    /// ([PERF-FLUTTER-TODO-CORPUS]).
    pub(crate) fn subject(
        &self,
        snippet: &Snippet<'_>,
        compute: impl FnOnce() -> Option<OwnedSubject>,
    ) -> Option<Arc<OwnedSubject>> {
        memo_entry(
            &self.subjects,
            (snippet.file_id, snippet.range.start, snippet.range.end),
            SUBJECT_MEMO_MAX,
            compute,
        )
    }

    /// The body-shape digest for the function covering `(file, start,
    /// end)`, memoised by the function's own range so sibling members
    /// of one function share the walk.
    pub(crate) fn body_digest(
        &self,
        key: SnippetKey,
        compute: impl FnOnce() -> [u8; 32],
    ) -> [u8; 32] {
        if let Some(hit) = locked(&self.body_digests).get(&key) {
            return *hit;
        }
        // Computed outside the lock, like every other cell: the digest
        // walks a whole function body, and holding the memo across it
        // would serialise every worker in the sharded noise split.
        let digest = compute();
        let mut map = locked(&self.body_digests);
        if map.len() < BODY_DIGEST_MEMO_MAX {
            let _previous = map.insert(key, digest);
        }
        digest
    }
}
