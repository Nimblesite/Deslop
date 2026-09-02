//! [PIPELINE-CLUSTER-EXACT-SCOPE] Which authored declaration an
//! occurrence sits inside.
//!
//! The same-file overlap collapse selects its representative by authored
//! scope and width only ([`super::collapse_overlapping_per_file`]): an
//! enclosing view inside the same authored declaration stays, otherwise
//! the wider byte range wins. Pair grades never enter the selection — a
//! bridge that should not connect two components must fail pair
//! admission ([PIPELINE-CLUSTER-CLOSURE]).
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
pub(crate) struct DeclarationScopes<'trees, L: BuildHasher> {
    /// Normalised root per file.
    trees: HashMap<FileId, &'trees NormalizedNode>,
    /// Language per file, for the declaration productions to look for.
    languages: &'trees HashMap<FileId, &'static str, L>,
}

impl<'trees, L: BuildHasher> DeclarationScopes<'trees, L> {
    /// Indexes `trees` by file so each lookup is one map hit plus a
    /// descent, rather than a scan of the corpus.
    pub(crate) fn new(
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
    pub(crate) fn enclosing(&self, member: &Fingerprint) -> Option<ByteRange> {
        let tree = self.trees.get(&member.file_id)?;
        let language = self.languages.get(&member.file_id)?;
        let kinds = function_kinds(language);
        (!kinds.is_empty())
            .then(|| smallest_enclosing(tree, member.byte_range, kinds))
            .flatten()
    }

    /// The byte range of the function-like declaration whose range
    /// **equals** the occurrence's range, when the grammar names one.
    ///
    /// Such an occurrence is the function the author wrote — modifier
    /// through closing brace — rather than a window over it. Under
    /// [PIPELINE-CLUSTER-EXACT-SCOPE] a view that is the declaration is
    /// the enclosing authored scope, and under
    /// [PIPELINE-CLUSTER-EXACT-SCOPE-STRADDLE] it outranks any view that
    /// cuts through that declaration. `None` marks windows, wrappers and
    /// whole files.
    pub(crate) fn aligned_function(&self, member: &Fingerprint) -> Option<ByteRange> {
        let tree = self.trees.get(&member.file_id)?;
        let language = self.languages.get(&member.file_id)?;
        let kinds = function_kinds(language);
        (!kinds.is_empty())
            .then(|| aligned_function_at(tree, member.byte_range, kinds))
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
    if !node.byte_range.strictly_encloses(range) {
        return None;
    }
    let deeper = node
        .children
        .iter()
        .find_map(|child| smallest_enclosing(child, range, kinds));
    deeper.or_else(|| kinds.contains(&node.kind).then_some(node.byte_range))
}

/// The range of the deepest descendant of `node` whose kind is in
/// `kinds` and whose range **equals** `range`.
fn aligned_function_at(
    node: &NormalizedNode,
    range: ByteRange,
    kinds: &[&str],
) -> Option<ByteRange> {
    if node.byte_range.start > range.start || node.byte_range.end < range.end {
        return None;
    }
    let deeper = node
        .children
        .iter()
        .find_map(|child| aligned_function_at(child, range, kinds));
    deeper.or_else(|| {
        (kinds.contains(&node.kind) && node.byte_range == range).then_some(node.byte_range)
    })
}
