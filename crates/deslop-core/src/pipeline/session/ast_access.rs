//! In-process AST access for the mechanical merge engine
//! ([AUTOFIX-MERGE]).
//!
//! The anti-unification path needs each occurrence's normalised
//! subtree and the raw source bytes behind it. Both live in the
//! session's in-memory state (the corpus store / `sources`) and are
//! exposed here as borrows — **never serialised to the wire**
//! ([AUTOFIX-MERGE-CODE-ACTION] in-process rule).

use crate::{
    ast::{ByteRange, NormalizedNode},
    state::FileId,
};

use super::PipelineSession;

impl PipelineSession {
    /// Raw source bytes of `file_id` as held by the current
    /// generation, or `None` for unknown files ([AUTOFIX-MERGE]).
    #[must_use]
    pub fn source_bytes_for(&self, file_id: FileId) -> Option<&[u8]> {
        self.sources.get(&file_id).map(Vec::as_slice)
    }

    /// Smallest normalised subtree of `file_id` covering `range`, or
    /// `None` when the file is unknown or the range lies outside the
    /// parse root ([AUTOFIX-MERGE]).
    #[must_use]
    pub fn subtree_at_range(&self, file_id: FileId, range: ByteRange) -> Option<&NormalizedNode> {
        self.store
            .tree_for(file_id)
            .and_then(|tree| tree.smallest_covering(range))
    }
}
