//! Endpoint view construction for [FUSION-SHARED-SUBTREE].
//!
//! An [`EndpointView`] is one endpoint's resolved measurement state:
//! the post-order kind sequence the alignment consumes, the creditable
//! subtrees the large-tree fallback consumes, and the kind multiset the
//! admission bound consumes ([FUSION-SHARED-SUBTREE-BOUND]). Built once
//! per endpoint and memoised by the measurer.

use std::collections::HashMap;

use super::{alignment::PostNode, SHARED_SUBTREE_MIN_CREDIT_NODES};
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
    /// Creditable subtrees for the large-tree fallback, largest first.
    pub(super) entries: Vec<Fingerprint>,
    /// Node-kind multiset, excluding the synthetic root, for the
    /// admission upper bound ([FUSION-SHARED-SUBTREE-BOUND]).
    pub(super) kind_counts: HashMap<&'static str, usize>,
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
        postorder.push(PostNode {
            kind: "__window__",
            leftmost: 1,
        });
        Self {
            postorder,
            total,
            entries: Vec::new(),
            kind_counts,
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
    let mut entries: Vec<Fingerprint> = Vec::new();
    for member in &members {
        push_postorder(member, &mut postorder);
        entries.extend(collect_fingerprints(
            member,
            SHARED_SUBTREE_MIN_CREDIT_NODES,
        ));
    }
    let total = postorder.len();
    let kind_counts = count_kinds(&postorder);
    // Synthetic window root: aligns the members as ordered siblings so
    // a multi-node sibling window is one tree for the alignment. It
    // matches its counterpart at zero cost, so the distance is exactly
    // the forest distance.
    postorder.push(PostNode {
        kind: "__window__",
        leftmost: 1,
    });
    entries.sort_by(|left, right| {
        right
            .node_count
            .cmp(&left.node_count)
            .then(left.byte_range.start.cmp(&right.byte_range.start))
    });
    Some(EndpointView {
        postorder,
        total,
        entries,
        kind_counts,
    })
}

/// Kind-multiset of a post-order sequence ([FUSION-SHARED-SUBTREE-BOUND]).
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
