//! Normalised AST representation shared across languages.
//!
//! Implements [PIPELINE-NORMALIZE-AST]: every language parser emits a
//! `NormalizedNode` tree with identical shape and semantics so that downstream
//! pipeline stages ([PIPELINE-FINGERPRINT-MERKLE],
//! [PIPELINE-CLUSTER-EXACT]) are completely language-agnostic.
//!
//! ## Invariants
//!
//! - `kind` is a stable, `'static` string. Two subtrees with the same logical
//!   structure and node kinds hash identically across runs.
//! - `byte_range` is an absolute byte offset into the original source file.
//!   This is the canonical location — line numbers are never stored, only
//!   computed at render time.
//! - Identifier, literal, comment, and whitespace nodes are collapsed by the
//!   per-language normaliser so Type-2 clones (renamed variables) produce
//!   identical trees.

use crate::state::FileId;

/// Half-open byte range `[start, end)` into a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    /// Inclusive start offset in bytes.
    pub start: usize,
    /// Exclusive end offset in bytes.
    pub end: usize,
}

impl ByteRange {
    /// Number of bytes spanned by this range. Returns 0 when
    /// `end <= start` (saturating semantics).
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns `true` when this range spans no bytes. Paired with
    /// [`Self::len`] so clippy's `len_without_is_empty` passes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.end <= self.start
    }
}

/// A normalised AST subtree. See the module docs for invariants.
#[derive(Debug, Clone)]
pub struct NormalizedNode {
    /// Structural node kind. Interned as `&'static str` by the language
    /// parser so equality is a pointer comparison in practice.
    pub kind: &'static str,
    /// Ordered children. Order matters for fingerprinting.
    pub children: Vec<NormalizedNode>,
    /// Byte range within the source file.
    pub byte_range: ByteRange,
    /// File this subtree belongs to.
    pub file_id: FileId,
}

impl NormalizedNode {
    /// Descends to the smallest subtree whose byte range contains
    /// `range` — the merge engine's occurrence-narrowing primitive
    /// ([AUTOFIX-MERGE]).
    ///
    /// `range` is clipped to this subtree's extent before the descent,
    /// because callers name a region of the *file* and a file is wider
    /// than its tree. Normalisation drops comments and the trailing
    /// newline, so the root spans only the code it kept
    /// ([PIPELINE-NORMALIZE-AST]); a whole-file request of
    /// `0..source.len()` is therefore contained by no node at all, and
    /// answering `None` makes the merge engine refuse a plan it should
    /// have produced. Clipping asks the question the caller meant: the
    /// smallest subtree covering the code inside `range`.
    ///
    /// A range lying wholly outside the tree clips to nothing and still
    /// yields `None`.
    #[must_use]
    pub fn smallest_covering(&self, range: ByteRange) -> Option<&Self> {
        let clipped = ByteRange {
            start: range.start.max(self.byte_range.start),
            end: range.end.min(self.byte_range.end),
        };
        (!clipped.is_empty()).then(|| self.covering(clipped))?
    }

    /// Strict-containment descent, once `range` is known to lie inside
    /// this subtree.
    fn covering(&self, range: ByteRange) -> Option<&Self> {
        if self.byte_range.start > range.start || self.byte_range.end < range.end {
            return None;
        }
        self.children
            .iter()
            .find_map(|child| child.covering(range))
            .or(Some(self))
    }

    /// Total number of nodes in this subtree, including `self`. Computed
    /// bottom-up by the caller and cached alongside the fingerprint.
    #[must_use]
    pub fn subtree_node_count(&self) -> usize {
        self.children
            .iter()
            .map(Self::subtree_node_count)
            .fold(1_usize, usize::saturating_add)
    }
}

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;

    #[test]
    fn byte_range_len_and_is_empty_agree_across_the_boundary() {
        let spanning = ByteRange { start: 2, end: 7 };
        assert_eq!(spanning.len(), 5);
        assert!(!spanning.is_empty());
        let zero_width = ByteRange { start: 4, end: 4 };
        assert_eq!(zero_width.len(), 0);
        assert!(zero_width.is_empty());
        // Saturating semantics: inverted ranges clamp to empty.
        let inverted = ByteRange { start: 9, end: 3 };
        assert_eq!(inverted.len(), 0);
        assert!(inverted.is_empty());
    }
}
