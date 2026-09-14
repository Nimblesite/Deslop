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
    ///
    /// The tree is re-parsed from the held source on demand: the store
    /// deliberately retains no normalised trees
    /// ([PERF-FLUTTER-TODO-MEMORY]) and the merge path asks for one
    /// subtree of one file at a time, so a single-file parse costs
    /// microseconds where retaining every corpus tree would cost
    /// gigabytes.
    pub fn subtree_at_range(&self, file_id: FileId, range: ByteRange) -> Option<NormalizedNode> {
        let source = self.sources.get(&file_id).map(Vec::as_slice)?;
        let language = self.file_languages.get(&file_id).copied()?;
        let parser = self.parsers.iter().find(|parser| parser.id() == language)?;
        let tree = parser.parse_and_normalize(source, file_id).ok()?;
        tree.smallest_covering(range).cloned()
    }
}
