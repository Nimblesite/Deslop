//! Bounded per-range memoisation for [`ParseCache`]
//! ([PERF-FLUTTER-TODO-CORPUS], [PERF-FLUTTER-TODO-MEMORY]): the capped
//! get-or-compute cells behind the filter queries a corpus-scale report
//! asks repeatedly per member range. Split from the parent module,
//! which owns the cache type and the CST/LRU machinery.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::super::body_shape::OwnedShapeToken;
use super::super::{calls::CallShape, polymorphic::OwnedSubject};
use super::{CallSequence, ParseCache, Snippet, SnippetKey};

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
    map: &RefCell<HashMap<SnippetKey, Option<Rc<T>>>>,
    key: SnippetKey,
    cap: usize,
    compute: impl FnOnce() -> Option<T>,
) -> Option<Rc<T>> {
    if let Some(hit) = map.borrow().get(&key) {
        return hit.clone();
    }
    let computed = compute().map(Rc::new);
    let mut map = map.borrow_mut();
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
    ) -> Option<Rc<CallShape>> {
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
    ) -> Option<Rc<CallSequence>> {
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
    ) -> Option<Rc<Vec<OwnedShapeToken>>> {
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
    ) -> Option<Rc<OwnedSubject>> {
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
        let mut map = self.body_digests.borrow_mut();
        if let Some(hit) = map.get(&key) {
            return *hit;
        }
        let digest = compute();
        if map.len() < BODY_DIGEST_MEMO_MAX {
            let _previous = map.insert(key, digest);
        }
        digest
    }
}
