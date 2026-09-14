//! Exact constant-time paths for one-node keyroot spans.

use super::{kind_at, small, write, ForestSpan, PostNode};

/// Writes exact distances when either keyroot span is a one-node tree.
///
/// A singleton against an `n`-node tree costs `n - 1` when its kind can be
/// retained anywhere in that tree, otherwise `n` including one relabel. Every
/// leftmost-spine subtree cell is persisted because later keyroot spans splice
/// those cells exactly as they splice general forest-DP results.
pub(super) fn write_distances(span: ForestSpan<'_>, tree_dist: &mut [u32]) -> bool {
    if span.left_leaf == span.left_root {
        return write_left(span, tree_dist);
    }
    if span.right_leaf == span.right_root {
        return write_right(span, tree_dist);
    }
    false
}

/// Writes a singleton-left span against every right leftmost-spine subtree.
fn write_left(span: ForestSpan<'_>, tree_dist: &mut [u32]) -> bool {
    let Some(tree) = span_nodes(span.right, span.right_leaf, span.right_root) else {
        return false;
    };
    write_against_tree(
        kind_at(span.left, span.left_root),
        tree,
        tree_dist,
        |position| left_slot(span, position),
    );
    true
}

/// Writes every left leftmost-spine subtree against a singleton right span.
fn write_right(span: ForestSpan<'_>, tree_dist: &mut [u32]) -> bool {
    let Some(tree) = span_nodes(span.left, span.left_leaf, span.left_root) else {
        return false;
    };
    write_against_tree(
        kind_at(span.right, span.right_root),
        tree,
        tree_dist,
        |position| right_slot(span, position),
    );
    true
}

/// Post-order nodes covered by one keyroot span.
fn span_nodes(nodes: &[PostNode], leaf: usize, root: usize) -> Option<&[PostNode]> {
    nodes.get(leaf.saturating_sub(1)..root)
}

/// Tree-distance slot for a singleton-left subtree pair.
fn left_slot(span: ForestSpan<'_>, position: usize) -> usize {
    span.left_root
        .saturating_mul(span.tree_stride)
        .saturating_add(span.right_leaf)
        .saturating_add(position)
}

/// Tree-distance slot for a singleton-right subtree pair.
fn right_slot(span: ForestSpan<'_>, position: usize) -> usize {
    span.left_leaf
        .saturating_add(position)
        .saturating_mul(span.tree_stride)
        .saturating_add(span.right_root)
}

/// Writes singleton-to-tree distance cells along the tree's leftmost spine.
fn write_against_tree(
    singleton_kind: &'static str,
    tree: &[PostNode],
    tree_dist: &mut [u32],
    slot: impl Fn(usize) -> usize,
) {
    let Some(leftmost) = tree.first().map(|node| node.leftmost) else {
        return;
    };
    let mut matched = false;
    for (position, node) in tree.iter().enumerate() {
        matched |= node.kind == singleton_kind;
        if node.leftmost == leftmost {
            let distance = position.saturating_add(usize::from(!matched));
            write(tree_dist, slot(position), small(distance));
        }
    }
}
