//! Endpoint view construction for [FUSED-SHARED-SUBTREE].
//!
//! An [`EndpointView`] is one endpoint's resolved measurement state:
//! the post-order kind sequence the alignment consumes, the creditable
//! subtrees the large-tree fallback consumes, and the kind multiset the
//! admission bound consumes ([FUSED-SHARED-SUBTREE-BOUND]). Built once
//! per endpoint and memoised by the measurer.

use std::collections::HashMap;

use super::{alignment::PostNode, subsequence::KindPositions, SHARED_SUBTREE_MIN_CREDIT_NODES};
use crate::{
    ast::NormalizedNode,
    fingerprint::{collect_fingerprints, Fingerprint},
    state::FileId,
    tokens::resolve_range_nodes,
};

/// One endpoint's resolved measurement state.
#[derive(Debug)]
pub(super) struct EndpointView {
    /// Post-order `(kind, leftmost-leaf index)` sequence under a
    /// synthetic window root, for the alignment.
    pub(super) postorder: Vec<PostNode>,
    /// Total nodes excluding the synthetic root.
    pub(super) total: usize,
    /// Creditable subtrees (≥ [`super::SHARED_SUBTREE_MIN_CREDIT_NODES`]
    /// nodes), largest-first — the large-tree fallback's population.
    /// Always built: any endpoint may pair with one past
    /// [`super::ALIGNMENT_MAX_NODES`], and the fallback reads both
    /// sides' entries.
    pub(super) entries: Vec<Fingerprint>,
    /// Node-kind multiset, excluding the synthetic root, for the
    /// admission upper bound ([FUSED-SHARED-SUBTREE-BOUND]).
    pub(super) kind_counts: HashMap<&'static str, usize>,
    /// Post-order positions by kind, excluding the synthetic root, for
    /// the ordered admission bound
    /// ([FUSED-SHARED-SUBTREE-BOUND-ORDER]). Built with the view so an
    /// endpoint appearing in many pairs is indexed once.
    pub(super) kind_positions: KindPositions,
}

impl EndpointView {
    /// Builds a view over a flat run of leaves under the synthetic
    /// window root — the minimal shape for asserting the alignment's
    /// arithmetic directly, without a parser in the way.
    #[cfg(test)]
    pub(super) fn from_flat_leaves(kinds: &[&'static str]) -> Self {
        let mut postorder: Vec<PostNode> = kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| PostNode {
                kind,
                leftmost: index.saturating_add(1),
            })
            .collect();
        let total = postorder.len();
        let kind_counts = count_kinds(&postorder);
        let kind_positions = KindPositions::new(&postorder, total);
        postorder.push(PostNode {
            kind: "__window__",
            leftmost: 1,
        });
        Self {
            postorder,
            total,
            entries: Vec::new(),
            kind_counts,
            kind_positions,
        }
    }

    /// Post-order sequence, including the synthetic window root.
    pub(super) fn postorder(&self) -> &[PostNode] {
        &self.postorder
    }

    /// Node total, excluding the synthetic window root.
    pub(super) const fn total(&self) -> usize {
        self.total
    }
}

/// Resolves the endpoint's nodes and builds every measurement input.
/// Resolution reuses [`resolve_range_nodes`] — the same resolver the
/// token stream and content walks use — so every signal sees the same
/// code, including synthetic sibling windows.
pub(super) fn build_view(
    tree_index: &HashMap<FileId, &NormalizedNode>,
    endpoint: &Fingerprint,
) -> Option<EndpointView> {
    let root = tree_index.get(&endpoint.file_id)?;
    let members = resolve_range_nodes(root, endpoint.byte_range.start, endpoint.byte_range.end)?;
    let mut postorder: Vec<PostNode> = Vec::new();
    for member in &members {
        push_postorder(member, &mut postorder);
    }
    let total = postorder.len();
    let kind_counts = count_kinds(&postorder);
    let kind_positions = KindPositions::new(&postorder, total);
    let entries = creditable_entries(&members);
    // Synthetic window root: aligns the members as ordered siblings so
    // a multi-node sibling window is one tree for the alignment. It
    // matches its counterpart at zero cost, so the distance is exactly
    // the forest distance.
    postorder.push(PostNode {
        kind: "__window__",
        leftmost: 1,
    });
    Some(EndpointView {
        postorder,
        total,
        entries,
        kind_counts,
        kind_positions,
    })
}

/// The creditable-subtree collection for a resolved endpoint. Built for
/// **every** endpoint: the large-tree fallback is selected by the
/// *pair's* larger side, so an endpoint at or under
/// [`super::ALIGNMENT_MAX_NODES`] still needs its entries when it pairs
/// with one past the cap — gating on the individual endpoint's size
/// starved `credit_shared_nodes` of the small side and silently dropped
/// real rescues (`a_small_endpoint_still_gets_credit_against_a_large_one`).
fn creditable_entries(members: &[&NormalizedNode]) -> Vec<Fingerprint> {
    let mut entries = Vec::new();
    for member in members {
        entries.extend(collect_fingerprints(
            member,
            SHARED_SUBTREE_MIN_CREDIT_NODES,
        ));
    }
    // Byte order, widest first at a tie: the fallback walks both
    // endpoints left to right and never looks backwards, so it needs
    // its candidates in position order, and it should be offered a
    // container before the subtrees nested inside it
    // ([FUSED-SHARED-SUBTREE]).
    entries.sort_by(super::credit::credit_order);
    entries
}

/// Kind-multiset of a post-order sequence ([FUSED-SHARED-SUBTREE-BOUND]).
/// Called before the synthetic root is appended so the multiset holds
/// exactly the window's own nodes.
fn count_kinds(postorder: &[PostNode]) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for node in postorder {
        let slot = counts.entry(node.kind).or_insert(0);
        *slot = slot.saturating_add(1);
    }
    counts
}

/// One in-progress frame of the iterative post-order walk.
struct WalkFrame<'tree> {
    /// Node being expanded.
    node: &'tree NormalizedNode,
    /// Next child to descend into.
    next_child: usize,
    /// Leftmost-leaf index inherited from the first child.
    leftmost: Option<usize>,
}

impl<'tree> WalkFrame<'tree> {
    /// Opens a frame over `node` with no children walked yet.
    const fn new(node: &'tree NormalizedNode) -> Self {
        Self {
            node,
            next_child: 0,
            leftmost: None,
        }
    }
}

/// Appends `node`'s subtree to `out` in post-order, recording each
/// node's leftmost-leaf index. Iterative so a deep tree cannot
/// overflow the stack (matching `fingerprint::hash_and_collect`).
fn push_postorder(node: &NormalizedNode, out: &mut Vec<PostNode>) {
    let mut stack = vec![WalkFrame::new(node)];
    while let Some(frame) = stack.last_mut() {
        if let Some(child) = frame.node.children.get(frame.next_child) {
            frame.next_child = frame.next_child.saturating_add(1);
            stack.push(WalkFrame::new(child));
            continue;
        }
        close_frame(&mut stack, out);
    }
}

/// Emits the top frame's node and folds its leftmost leaf into its
/// parent, which inherits it from its first child.
fn close_frame(stack: &mut Vec<WalkFrame<'_>>, out: &mut Vec<PostNode>) {
    let Some(frame) = stack.pop() else {
        return;
    };
    let leftmost = frame
        .leftmost
        .unwrap_or_else(|| out.len().saturating_add(1));
    out.push(PostNode {
        kind: frame.node.kind,
        leftmost,
    });
    if let Some(parent) = stack.last_mut() {
        if parent.leftmost.is_none() {
            parent.leftmost = Some(leftmost);
        }
    }
}
