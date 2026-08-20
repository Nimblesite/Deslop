//! Ordered tree alignment for [FUSION-SHARED-SUBTREE].
//!
//! Zhang–Shasha tree edit distance over post-order sequences of
//! normalised node kinds, with unit insert/delete/relabel costs. Kept
//! separate from the measurement policy in [`super`] so the algorithm
//! can be asserted on its own terms: the DP is where a subtle error
//! would silently change every `structural` in every report.

use std::collections::HashMap;

/// One node of the post-order sequence.
#[derive(Debug, Clone, Copy)]
pub(super) struct PostNode {
    /// Normalised kind.
    pub(super) kind: &'static str,
    /// 1-based post-order index of this node's leftmost leaf.
    pub(super) leftmost: usize,
}

/// Shared node count under the optimal ordered alignment:
/// `max(total) - TED`, floored at zero. With unit costs the distance
/// is the node mass the alignment could not match, so this is the
/// aligned analogue of the fallback's credited mass.
pub(super) fn aligned_shared_nodes(
    left: &super::EndpointView,
    right: &super::EndpointView,
) -> usize {
    let distance = tree_edit_distance(left.postorder(), right.postorder());
    left.total().max(right.total()).saturating_sub(distance)
}

/// Zhang–Shasha tree edit distance over post-order sequences with unit
/// insert/delete/relabel costs. Standard keyroot decomposition; the
/// forest-distance grid is allocated per keyroot pair at the exact
/// size it needs, which the [`ALIGNMENT_MAX_NODES`] cap keeps small.
fn tree_edit_distance(left: &[PostNode], right: &[PostNode]) -> usize {
    let mut tree_dist = Grid::new(left.len().saturating_add(1), right.len().saturating_add(1));
    for &left_root in &keyroots(left) {
        for &right_root in &keyroots(right) {
            forest_distance(left, right, left_root, right_root, &mut tree_dist);
        }
    }
    tree_dist.get(left.len(), right.len())
}

/// 1-based post-order indices whose leftmost leaf is not shared with a
/// later node — the Zhang–Shasha keyroots, ascending.
fn keyroots(nodes: &[PostNode]) -> Vec<usize> {
    let mut latest: HashMap<usize, usize> = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        let position = index.saturating_add(1);
        let _previous = latest.insert(node.leftmost, position);
    }
    let mut roots: Vec<usize> = latest.into_values().collect();
    roots.sort_unstable();
    roots
}

/// The fixed context one keyroot-pair DP works inside.
#[derive(Clone, Copy)]
struct ForestSpan<'seq> {
    /// Left post-order sequence.
    left: &'seq [PostNode],
    /// Right post-order sequence.
    right: &'seq [PostNode],
    /// Leftmost leaf of the left keyroot (1-based).
    left_leaf: usize,
    /// Leftmost leaf of the right keyroot (1-based).
    right_leaf: usize,
}

/// Fills `tree_dist` for the subtree pair rooted at the two keyroots.
fn forest_distance(
    left: &[PostNode],
    right: &[PostNode],
    left_root: usize,
    right_root: usize,
    tree_dist: &mut Grid,
) {
    let span = ForestSpan {
        left,
        right,
        left_leaf: leftmost(left, left_root),
        right_leaf: leftmost(right, right_root),
    };
    let mut forest = seeded_grid(span, left_root, right_root);
    for left_index in span.left_leaf..=left_root {
        for right_index in span.right_leaf..=right_root {
            fill_cell(span, left_index, right_index, &mut forest, tree_dist);
        }
    }
}

/// The keyroot pair's forest grid, sized to the two subtrees and seeded
/// with the pure insert/delete borders.
fn seeded_grid(span: ForestSpan<'_>, left_root: usize, right_root: usize) -> Grid {
    let mut forest = Grid::new(
        left_root.saturating_sub(span.left_leaf).saturating_add(2),
        right_root.saturating_sub(span.right_leaf).saturating_add(2),
    );
    for row in 1..forest.rows() {
        forest.set(row, 0, row);
    }
    for column in 1..forest.columns() {
        forest.set(0, column, column);
    }
    forest
}

/// Computes one forest-distance cell, recording a tree distance when
/// both prefixes are whole subtrees.
fn fill_cell(
    span: ForestSpan<'_>,
    left_index: usize,
    right_index: usize,
    forest: &mut Grid,
    tree_dist: &mut Grid,
) {
    let (row, column) = (
        left_index.saturating_sub(span.left_leaf).saturating_add(1),
        right_index
            .saturating_sub(span.right_leaf)
            .saturating_add(1),
    );
    let whole_trees = leftmost(span.left, left_index) == span.left_leaf
        && leftmost(span.right, right_index) == span.right_leaf;
    let best = forest
        .get(row.saturating_sub(1), column)
        .saturating_add(1)
        .min(forest.get(row, column.saturating_sub(1)).saturating_add(1))
        .min(substitute_cost(
            span,
            (left_index, right_index),
            (row, column),
            whole_trees,
            forest,
            tree_dist,
        ));
    forest.set(row, column, best);
    if whole_trees {
        tree_dist.set(left_index, right_index, best);
    }
}

/// The third Zhang–Shasha option: relabel two whole subtrees against
/// each other, or splice in an already-computed tree distance.
fn substitute_cost(
    span: ForestSpan<'_>,
    (left_index, right_index): (usize, usize),
    (row, column): (usize, usize),
    whole_trees: bool,
    forest: &Grid,
    tree_dist: &Grid,
) -> usize {
    if whole_trees {
        let relabel =
            usize::from(kind_at(span.left, left_index) != kind_at(span.right, right_index));
        return forest
            .get(row.saturating_sub(1), column.saturating_sub(1))
            .saturating_add(relabel);
    }
    let left_prefix = leftmost(span.left, left_index).saturating_sub(span.left_leaf);
    let right_prefix = leftmost(span.right, right_index).saturating_sub(span.right_leaf);
    forest
        .get(left_prefix, right_prefix)
        .saturating_add(tree_dist.get(left_index, right_index))
}

/// Leftmost-leaf index of the 1-based post-order position.
fn leftmost(nodes: &[PostNode], position: usize) -> usize {
    nodes
        .get(position.saturating_sub(1))
        .map_or(0, |node| node.leftmost)
}

/// Kind at the 1-based post-order position.
fn kind_at(nodes: &[PostNode], position: usize) -> &'static str {
    nodes
        .get(position.saturating_sub(1))
        .map_or("", |node| node.kind)
}

/// Dense `usize` matrix with checked access. Out-of-range reads return
/// `usize::MAX` so a min-fold can never elect them; out-of-range
/// writes are ignored. Every in-algorithm access is in range by
/// construction — the checked forms exist for the lint contract, and
/// the alignment's unit tests pin the arithmetic.
struct Grid {
    /// Column count.
    columns: usize,
    /// Row-major cells.
    cells: Vec<usize>,
}

impl Grid {
    /// Zero-filled `rows × columns` grid.
    fn new(rows: usize, columns: usize) -> Self {
        Self {
            columns,
            cells: vec![0; rows.saturating_mul(columns)],
        }
    }

    /// Row count.
    fn rows(&self) -> usize {
        self.cells.len().checked_div(self.columns).unwrap_or(0)
    }

    /// Column count.
    fn columns(&self) -> usize {
        self.columns
    }

    /// Cell value; `usize::MAX` out of range.
    fn get(&self, row: usize, column: usize) -> usize {
        self.cells
            .get(row.saturating_mul(self.columns).saturating_add(column))
            .copied()
            .unwrap_or(usize::MAX)
    }

    /// Writes a cell, ignoring out-of-range writes.
    fn set(&mut self, row: usize, column: usize, value: usize) {
        let index = row.saturating_mul(self.columns).saturating_add(column);
        if let Some(cell) = self.cells.get_mut(index) {
            *cell = value;
        }
    }
}
