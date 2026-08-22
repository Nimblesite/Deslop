//! [PIPELINE-CLUSTER-EXACT-SCOPE] Which authored declaration an
//! occurrence sits inside.
//!
//! The same-file overlap collapse ranks two views of one region by the
//! cross-file evidence each carries ([`super::cross_file_edge_strengths`]).
//! That comparison is only meaningful between views of comparable
//! scope. Inside one function a narrower view scores higher *because*
//! it excludes the statements that differ, so ranking by grade there
//! elects whichever window omits the most — the same non-comparability
//! [`crate::cluster::subsume`] already documents between nesting
//! cluster views, one stage earlier and with no content evidence yet
//! measured.
//!
//! Declarations are read off the normalised tree, keyed by the file's
//! language ([`function_kinds`]), so a production name that means
//! "function" in one grammar cannot classify a node in another.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    ast::{ByteRange, NormalizedNode},
    cluster_filters::function_kinds,
    fingerprint::Fingerprint,
    state::FileId,
};

/// The authored declaration enclosing an occurrence, resolved against
/// the normalised trees the corpus already holds.
pub(super) struct DeclarationScopes<'trees, L: BuildHasher> {
    /// Normalised root per file.
    trees: HashMap<FileId, &'trees NormalizedNode>,
    /// Language per file, for the declaration productions to look for.
    languages: &'trees HashMap<FileId, &'static str, L>,
}

impl<'trees, L: BuildHasher> DeclarationScopes<'trees, L> {
    /// Indexes `trees` by file so each lookup is one map hit plus a
    /// descent, rather than a scan of the corpus.
    pub(super) fn new(
        trees: &'trees [NormalizedNode],
        languages: &'trees HashMap<FileId, &'static str, L>,
    ) -> Self {
        Self {
            trees: trees.iter().map(|tree| (tree.file_id, tree)).collect(),
            languages,
        }
    }

    /// The byte range of the smallest function-like declaration that
    /// **strictly** encloses `member`, when the grammar names one.
    ///
    /// Strictly, because a view that *is* the declaration is not inside
    /// it: an exact whole-function occurrence and a window nested in it
    /// are views of different scopes, and the wider one is the
    /// declaration itself.
    ///
    /// `None` where no such production encloses the member — a run of
    /// top-level bindings, a class body, a whole file. Those views span
    /// genuinely different amounts of authored code, so their measured
    /// grades are comparable and the strength contest stands (#339).
    pub(super) fn enclosing(&self, member: &Fingerprint) -> Option<ByteRange> {
        let tree = self.trees.get(&member.file_id)?;
        let language = self.languages.get(&member.file_id)?;
        let kinds = function_kinds(language);
        (!kinds.is_empty())
            .then(|| smallest_enclosing(tree, member.byte_range, kinds))
            .flatten()
    }
}

/// The smallest descendant of `node` whose kind is in `kinds` and whose
/// range strictly encloses `range`.
fn smallest_enclosing(
    node: &NormalizedNode,
    range: ByteRange,
    kinds: &[&str],
) -> Option<ByteRange> {
    if !strictly_encloses(node.byte_range, range) {
        return None;
    }
    let deeper = node
        .children
        .iter()
        .find_map(|child| smallest_enclosing(child, range, kinds));
    deeper.or_else(|| kinds.contains(&node.kind).then_some(node.byte_range))
}

/// True when `outer` covers `inner` and is wider on at least one side.
fn strictly_encloses(outer: ByteRange, inner: ByteRange) -> bool {
    outer.start <= inner.start
        && inner.end <= outer.end
        && (outer.start < inner.start || inner.end < outer.end)
}
