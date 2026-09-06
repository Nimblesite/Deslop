//! Range-bounded searches over a tree-sitter tree, shared by every
//! cluster filter that gathers the nodes of one kind inside a member's
//! reported bytes ([CLONE-NOISE]).

use tree_sitter::Node;

use super::node_intersects_range;
use crate::ast::{named_children, ByteRange};

/// Returns true when `node` lies wholly inside `range`.
pub(super) fn node_enclosed_by_range(node: Node<'_>, range: ByteRange) -> bool {
    node.start_byte() >= range.start && node.end_byte() <= range.end
}

/// How much of a hit must lie inside the searched range.
#[derive(Clone, Copy, Debug)]
enum Bound {
    /// The hit lies wholly inside the range.
    Enclosed,
    /// The hit merely overlaps the range.
    Intersecting,
}

/// Whether a hit found inside another hit is reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nested {
    /// Descent stops at a hit, so nested hits are never double-counted.
    Skip,
    /// Descent continues below a hit, so nested hits are reported too.
    Include,
}

/// A depth-first search for the named nodes whose kind satisfies
/// `is_kind`, bounded to one byte range.
#[derive(Debug)]
pub(super) struct KindSearch<IsKind> {
    /// Byte range the search is bounded to.
    range: ByteRange,
    /// How much of a hit must lie inside `range`.
    bound: Bound,
    /// Whether hits nested inside other hits are reported.
    nested: Nested,
    /// Node-kind predicate a hit must satisfy.
    is_kind: IsKind,
}

impl<IsKind: Fn(&str) -> bool> KindSearch<IsKind> {
    /// Searches for nodes lying wholly inside `range`, stopping at each hit.
    pub(super) fn enclosed(range: ByteRange, is_kind: IsKind) -> Self {
        Self {
            range,
            bound: Bound::Enclosed,
            nested: Nested::Skip,
            is_kind,
        }
    }

    /// Searches for nodes overlapping `range`, stopping at each hit.
    pub(super) fn intersecting(range: ByteRange, is_kind: IsKind) -> Self {
        Self {
            range,
            bound: Bound::Intersecting,
            nested: Nested::Skip,
            is_kind,
        }
    }

    /// Reports hits nested inside other hits as well.
    pub(super) fn with_nested_hits(self) -> Self {
        Self {
            nested: Nested::Include,
            ..self
        }
    }

    /// Every hit under `root`, in source order.
    pub(super) fn nodes<'tree>(&self, root: Node<'tree>) -> Vec<Node<'tree>> {
        let mut out = Vec::new();
        self.collect(root, &mut out);
        out
    }

    /// The hit under `root`, when there is exactly one.
    pub(super) fn sole_node<'tree>(&self, root: Node<'tree>) -> Option<Node<'tree>> {
        let nodes = self.nodes(root);
        let [node] = nodes.as_slice() else {
            return None;
        };
        Some(*node)
    }

    /// Appends the hits under `node` to `out`, pruning subtrees that do
    /// not touch `range`.
    fn collect<'tree>(&self, node: Node<'tree>, out: &mut Vec<Node<'tree>>) {
        if !node_intersects_range(node, self.range) {
            return;
        }
        if self.is_hit(node) {
            out.push(node);
            if self.nested == Nested::Skip {
                return;
            }
        }
        for child in named_children(node) {
            self.collect(child, out);
        }
    }

    /// True when `node` satisfies the kind predicate within the bound.
    fn is_hit(&self, node: Node<'_>) -> bool {
        let inside = match self.bound {
            Bound::Enclosed => node_enclosed_by_range(node, self.range),
            Bound::Intersecting => true,
        };
        inside && (self.is_kind)(node.kind())
    }
}
