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
    /// Number of bytes spanned by this range. Returns 0 if `end <= start`.
    #[must_use]
    pub const fn len(self) -> usize {
        if self.end > self.start {
            self.end.saturating_sub(self.start)
        } else {
            0
        }
    }

    /// Returns `true` when this range spans no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
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
