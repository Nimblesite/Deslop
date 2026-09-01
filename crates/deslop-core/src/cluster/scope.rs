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

    /// The full byte range of the function-like declaration whose range
    /// **equals** the occurrence's range, when the grammar names one.
    ///
    /// An occurrence aligned like this sits exactly on an authored
    /// function — modifier through closing brace — so it names the
    /// function the author wrote rather than an arbitrary window. Under
    /// [PIPELINE-CLUSTER-EXACT-SCOPE] a view that is the declaration is
    /// treated as the enclosing authored scope (root cause of gh #486);
    /// `None` marks windows, wrappers and whole files.
    ///
    /// This is one side of a mirrored bucket: the sibling file's
    /// collapse runs over the same fingerprints, so an exact function
    /// view elects the same alignment in every occurrence of the pair.
    pub(super) fn aligned_function(&self, member: &Fingerprint) -> Option<ByteRange> {
        let tree = self.trees.get(&member.file_id)?;
        let language = self.languages.get(&member.file_id)?;
        let kinds = function_kinds(language);
        (!kinds.is_empty())
            .then(|| aligned_function_at(tree, member.byte_range, kinds))
            .flatten()
    }

    /// Whether the occurrence's range strictly contains a function-like
    /// declaration — that is, it crosses that declaration's boundary
    /// without sitting on it.
    ///
    /// A whole-file or class-body view crosses the boundary of every
    /// function it spans; an exact function view crosses nothing (a
    /// nested function inside it is also a boundary it straddles, which
    /// conservatively keeps the width contest for that view).
    pub(super) fn crosses_function_boundary(&self, member: &Fingerprint) -> bool {
        let Some(tree) = self.trees.get(&member.file_id) else {
            return false;
        };
        let Some(language) = self.languages.get(&member.file_id) else {
            return false;
        };
        let kinds = function_kinds(language);
        !kinds.is_empty() && crosses_function_boundary_at(tree, member.byte_range, kinds)
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

/// Whether any descendant of `node` whose kind is in `kinds` lies
/// strictly inside `range` — contained by it without coinciding.
fn crosses_function_boundary_at(node: &NormalizedNode, range: ByteRange, kinds: &[&str]) -> bool {
    if node.byte_range.start >= range.end || node.byte_range.end <= range.start {
        return false;
    }
    let strictly_inside = node.byte_range.start >= range.start
        && node.byte_range.end <= range.end
        && (node.byte_range.start > range.start || node.byte_range.end < range.end);
    if kinds.contains(&node.kind) && strictly_inside {
        return true;
    }
    node.children
        .iter()
        .any(|child| crosses_function_boundary_at(child, range, kinds))
}
