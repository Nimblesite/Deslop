//! [PIPELINE-CLUSTER-EXACT-SCOPE-MATCHED] Whether a view's width is
//! matched by one of its copies.
//!
//! A view is matched when some copy holds, among its own top-level nodes,
//! the kind of the view's first node and the kind of its last, and
//! unmatched when it has copies and none does. A view with no copy of its
//! own — it reached the component only through a view of its own region —
//! is unproven: no copy says anything about its width. A window
//! that opens on a type declared just above a copied function, against a
//! copy that is only the function, opens on a kind no copy has: its extra
//! width is whatever the author wrote next to the duplication. A run of
//! same-kind nodes — statements, dictionary entries, functions — is
//! always matched, so the rule never decides between two readings of a
//! homogeneous run.

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
};

use super::scope::DeclarationScopes;
use crate::{fingerprint::Fingerprint, pair::FusedCluster};

/// What a view's copies say about its width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Width {
    /// A copy holds the kinds of both the view's end nodes.
    Matched,
    /// The view has copies, and none holds both end kinds.
    Unmatched,
    /// No copy, or no resolvable nodes: nothing is known.
    Unproven,
}

/// Width evidence for one component, worked out on first ask.
pub(super) struct WidthEvidence<'run, 'corpus, L: BuildHasher> {
    /// Every fingerprint, indexed like the component's members.
    fingerprints: &'run [Fingerprint],
    /// Resolves a view to the nodes it covers.
    scopes: &'run DeclarationScopes<'corpus, L>,
    /// Each member's copies ([`copies_by_member`]).
    copies: HashMap<usize, Vec<usize>>,
    /// Views already decided.
    decided: HashMap<usize, Width>,
}

impl<'run, 'corpus, L: BuildHasher> WidthEvidence<'run, 'corpus, L> {
    /// Starts one component's evidence with nothing decided.
    pub(super) fn new(
        fused: &FusedCluster,
        fingerprints: &'run [Fingerprint],
        scopes: &'run DeclarationScopes<'corpus, L>,
    ) -> Self {
        Self {
            copies: copies_by_member(fused, fingerprints),
            fingerprints,
            scopes,
            decided: HashMap::new(),
        }
    }

    /// What `view`'s copies say about its width.
    pub(super) fn width(&mut self, view: usize) -> Width {
        if let Some(known) = self.decided.get(&view) {
            return *known;
        }
        let width = self.decide(view);
        let _previous = self.decided.insert(view, width);
        width
    }

    /// Reads `view`'s end kinds and looks for a copy that has both.
    fn decide(&self, view: usize) -> Width {
        let (Some(ends), Some(copies)) = (self.end_kinds(view), self.copies.get(&view)) else {
            return Width::Unproven;
        };
        let matched = copies.iter().any(|copy| {
            self.top_kinds(*copy)
                .is_some_and(|kinds| ends.iter().all(|end| kinds.contains(end)))
        });
        if matched {
            Width::Matched
        } else {
            Width::Unmatched
        }
    }

    /// The kinds of `view`'s first and last top-level node.
    fn end_kinds(&self, view: usize) -> Option<[&'static str; 2]> {
        let nodes = self.scopes.top_nodes(self.fingerprints.get(view)?)?;
        Some([nodes.first()?.kind, nodes.last()?.kind])
    }

    /// The kinds of every top-level node `view` covers.
    fn top_kinds(&self, view: usize) -> Option<HashSet<&'static str>> {
        let nodes = self.scopes.top_nodes(self.fingerprints.get(view)?)?;
        Some(nodes.iter().map(|node| node.kind).collect())
    }
}

/// Every member's copies: the other end of each admitted edge that is not
/// a second view of the same region — a member of another file, or of
/// the same file without overlapping it.
fn copies_by_member(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
) -> HashMap<usize, Vec<usize>> {
    let mut copies: HashMap<usize, Vec<usize>> = HashMap::new();
    for edge in &fused.edges {
        let (Some(left), Some(right)) = (fingerprints.get(edge.left), fingerprints.get(edge.right))
        else {
            continue;
        };
        if left.file_id == right.file_id && left.byte_range.overlaps(right.byte_range) {
            continue;
        }
        copies.entry(edge.left).or_default().push(edge.right);
        copies.entry(edge.right).or_default().push(edge.left);
    }
    copies
}
