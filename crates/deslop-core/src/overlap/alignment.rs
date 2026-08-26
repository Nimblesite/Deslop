//! Ordered tree alignment for [FUSION-SHARED-SUBTREE].
//!
//! Zhang–Shasha tree edit distance over post-order sequences of
//! normalised node kinds, with unit insert/delete/relabel costs. Kept
//! separate from the measurement policy in [`super`] so the algorithm
//! can be asserted on its own terms: the DP is where a subtle error
//! would silently change every `structural` in every report, which
//! `tests` pins against an independent textbook recurrence.
//!
//! [PERF-FLUTTER-TODO-RESCUE] This is the pipeline's hot loop — around
//! two thirds of a corpus-scale run's CPU, reached from both the rescue
//! and the per-cluster signal measurement. The two grids are therefore
//! owned by a long-lived [`Aligner`] and reused across every alignment
//! a worker performs, rather than allocated per call and per keyroot
//! pair: the textbook formulation allocates one forest grid for each of
//! the |keyroots| × |keyroots| sub-problems, which on Flutter meant
//! billions of short-lived allocations that computed nothing. Cells are
//! `u32` (a distance cannot exceed the node cap) addressed through a
//! fixed row stride, so the inner loop is a base-plus-offset read
//! instead of a bounds-checked multiply.
//!
//! None of that changes a value. The reused grids are re-seeded before
//! each sub-problem and every interior cell is written before it is
//! read, so the distance is identical to the freshly-allocated form.

/// Alignment arithmetic pinned against the textbook recurrence.
#[cfg(test)]
mod tests;

/// One node of the post-order sequence.
#[derive(Debug, Clone, Copy)]
pub(super) struct PostNode {
    /// Normalised kind.
    pub(super) kind: &'static str,
    /// 1-based post-order index of this node's leftmost leaf.
    pub(super) leftmost: usize,
}

/// Reusable scratch for the ordered tree alignment.
///
/// One per worker, carried alongside the measurer's memos: the buffers
/// grow to the largest pair a worker has measured and are then reused,
/// so a corpus-scale pass allocates a bounded amount however many
/// alignments it runs.
#[derive(Debug, Default)]
pub(super) struct Aligner {
    /// Subtree-distance grid, row stride `right.len() + 1`.
    tree_dist: Vec<u32>,
    /// Forest-distance grid for one keyroot pair, row stride
    /// `right.len() + 2` — one allocation for every sub-problem.
    forest: Vec<u32>,
    /// Left keyroots, ascending, 1-based.
    left_keyroots: Vec<usize>,
    /// Right keyroots, ascending, 1-based.
    right_keyroots: Vec<usize>,
    /// Last post-order position seen per leftmost-leaf index.
    latest: Vec<usize>,
}

impl Aligner {
    /// Shared node count under the optimal ordered alignment:
    /// `max(total) - TED`, floored at zero. With unit costs the distance
    /// is the node mass the alignment could not match, so this is the
    /// aligned analogue of the fallback's credited mass.
    pub(super) fn shared_nodes(
        &mut self,
        left: &super::EndpointView,
        right: &super::EndpointView,
    ) -> usize {
        let distance = self.distance(left.postorder(), right.postorder());
        left.total().max(right.total()).saturating_sub(distance)
    }

    /// Zhang–Shasha tree edit distance over post-order sequences with
    /// unit insert/delete/relabel costs. Standard keyroot decomposition.
    pub(super) fn distance(&mut self, left: &[PostNode], right: &[PostNode]) -> usize {
        self.reset(left, right);
        for left_position in 0..self.left_keyroots.len() {
            for right_position in 0..self.right_keyroots.len() {
                let Some(span) = self.span(left, right, (left_position, right_position)) else {
                    continue;
                };
                let Self {
                    forest, tree_dist, ..
                } = self;
                forest_distance(span, forest, tree_dist);
            }
        }
        let last = left
            .len()
            .saturating_mul(right.len().saturating_add(1))
            .saturating_add(right.len());
        usize::try_from(read(&self.tree_dist, last)).unwrap_or(0)
    }

    /// Clears both grids to the size this pair needs and recomputes the
    /// two keyroot lists.
    ///
    /// `tree_dist` must start zeroed: the decomposition reads a subtree
    /// pair's cell before writing it whenever the sub-problem is not
    /// itself a whole-tree pair, and a stale value carried over from the
    /// previous alignment would be spliced in as if it belonged here.
    fn reset(&mut self, left: &[PostNode], right: &[PostNode]) {
        let tree_cells = left
            .len()
            .saturating_add(1)
            .saturating_mul(right.len().saturating_add(1));
        self.tree_dist.clear();
        self.tree_dist.resize(tree_cells, 0);
        let forest_cells = left
            .len()
            .saturating_add(2)
            .saturating_mul(right.len().saturating_add(2));
        self.forest.clear();
        self.forest.resize(forest_cells, 0);
        let Self {
            latest,
            left_keyroots,
            right_keyroots,
            ..
        } = self;
        keyroots(left, latest, left_keyroots);
        keyroots(right, latest, right_keyroots);
    }

    /// The context for one keyroot pair, or `None` when either keyroot
    /// index is out of range.
    fn span<'seq>(
        &self,
        left: &'seq [PostNode],
        right: &'seq [PostNode],
        (left_position, right_position): (usize, usize),
    ) -> Option<ForestSpan<'seq>> {
        let left_root = self.left_keyroots.get(left_position).copied()?;
        let right_root = self.right_keyroots.get(right_position).copied()?;
        Some(ForestSpan {
            left,
            right,
            left_leaf: leftmost(left, left_root),
            right_leaf: leftmost(right, right_root),
            left_root,
            right_root,
            forest_stride: right.len().saturating_add(2),
            tree_stride: right.len().saturating_add(1),
        })
    }
}

/// Shared node count under the optimal ordered alignment, with scratch
/// of its own.
///
/// The measurement path calls [`Aligner::shared_nodes`] on the measurer's
/// long-lived aligner; the assertions in [`super::tests`] hold a view pair
/// and no measurer, and want the value rather than the plumbing. Same
/// arithmetic, one throwaway pair of grids.
#[cfg(test)]
pub(super) fn aligned_shared_nodes(
    left: &super::EndpointView,
    right: &super::EndpointView,
) -> usize {
    Aligner::default().shared_nodes(left, right)
}

/// 1-based post-order indices whose leftmost leaf is not shared with a
/// later node — the Zhang–Shasha keyroots, ascending.
///
/// A leftmost-leaf index is itself a post-order position, so the "last
/// node per leftmost leaf" table is a flat array rather than a hash map:
/// same keyroots, no hashing in a function called twice per alignment.
fn keyroots(nodes: &[PostNode], latest: &mut Vec<usize>, out: &mut Vec<usize>) {
    latest.clear();
    latest.resize(nodes.len().saturating_add(1), 0);
    for (index, node) in nodes.iter().enumerate() {
        if let Some(slot) = latest.get_mut(node.leftmost) {
            *slot = index.saturating_add(1);
        }
    }
    out.clear();
    out.extend(latest.iter().copied().filter(|&position| position != 0));
    out.sort_unstable();
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
    /// Left keyroot (1-based).
    left_root: usize,
    /// Right keyroot (1-based).
    right_root: usize,
    /// Row stride of the forest grid.
    forest_stride: usize,
    /// Row stride of the subtree-distance grid.
    tree_stride: usize,
}

/// One row of the keyroot pair's forest grid, resolved once so the
/// inner loop reads offsets instead of recomputing them per cell.
#[derive(Clone, Copy)]
struct Row {
    /// Grid offset of this row's column zero.
    base: usize,
    /// Grid offset of the previous row's column zero.
    above: usize,
    /// Whether the left node at this row roots a whole subtree.
    left_whole: bool,
    /// Grid offset of the row a non-whole splice reads its corner from.
    corner_base: usize,
    /// Leftmost leaf of the right keyroot, the whole-subtree test's
    /// right-hand side.
    right_leaf: usize,
    /// Offset of this row's first subtree-distance cell.
    subtree_base: usize,
    /// Left node's kind.
    left_kind: &'static str,
}

/// Fills `tree_dist` for the subtree pair rooted at the two keyroots.
fn forest_distance(span: ForestSpan<'_>, forest: &mut [u32], tree_dist: &mut [u32]) {
    seed(span, forest);
    for left_index in span.left_leaf..=span.left_root {
        let left_leftmost = leftmost(span.left, left_index);
        let base = left_index
            .saturating_sub(span.left_leaf)
            .saturating_add(1)
            .saturating_mul(span.forest_stride);
        fill_row(span, row_at(span, left_index, left_leftmost, base), forest, tree_dist);
    }
}

/// Resolves everything one row needs before its cells are walked, so
/// the inner loop does no multiplication and no sequence lookup.
fn row_at(span: ForestSpan<'_>, left_index: usize, left_leftmost: usize, base: usize) -> Row {
    Row {
        base,
        above: base.saturating_sub(span.forest_stride),
        left_whole: left_leftmost == span.left_leaf,
        corner_base: left_leftmost
            .saturating_sub(span.left_leaf)
            .saturating_mul(span.forest_stride),
        right_leaf: span.right_leaf,
        subtree_base: left_index
            .saturating_mul(span.tree_stride)
            .saturating_add(span.right_leaf),
        left_kind: kind_at(span.left, left_index),
    }
}

/// Re-seeds the forest grid's pure insert/delete borders for one
/// keyroot pair. Every interior cell is written before it is read, so
/// only the borders carry over from the previous sub-problem.
fn seed(span: ForestSpan<'_>, forest: &mut [u32]) {
    write(forest, 0, 0);
    let rows = span.left_root.saturating_sub(span.left_leaf).saturating_add(2);
    for row in 1..rows {
        let slot = row.saturating_mul(span.forest_stride);
        write(forest, slot, small(row));
    }
    let columns = span
        .right_root
        .saturating_sub(span.right_leaf)
        .saturating_add(2);
    for column in 1..columns {
        write(forest, column, small(column));
    }
}

/// Fills one row of the keyroot pair's forest grid.
///
/// The row is walked as slices rather than grid coordinates: the cells
/// above it, the cells written before it, its own cells, and its
/// subtree-distance run are each resolved once here, so a cell costs
/// three slice reads and no arithmetic on offsets. The cell to the
/// left is the value just produced, so it is carried in `previous`
/// rather than read back out of the grid at all.
fn fill_row(span: ForestSpan<'_>, row: Row, forest: &mut [u32], tree_dist: &mut [u32]) {
    let Some(nodes) = span
        .right
        .get(span.right_leaf.saturating_sub(1)..span.right_root)
    else {
        return;
    };
    let (earlier, current) = forest.split_at_mut(row.base);
    let (Some(above), Some(subtree)) = (
        earlier.get(row.above..),
        tree_dist.get_mut(row.subtree_base..),
    ) else {
        return;
    };
    let mut previous = read(current, 0);
    for (offset, node) in nodes.iter().enumerate() {
        previous = cell(row, Cursor { offset, node, previous }, Grids { above, earlier, subtree });
        write(current, offset.saturating_add(1), previous);
    }
}

/// One cell's position in a row, and the value just written to its left.
struct Cursor<'seq> {
    /// Zero-based offset from the row's first measured column.
    offset: usize,
    /// The right sequence's node at this column.
    node: &'seq PostNode,
    /// The cell immediately to the left, already computed.
    previous: u32,
}

/// The three runs of cells one row reads.
struct Grids<'grid> {
    /// The row above, from its column zero.
    above: &'grid [u32],
    /// Every cell written before this row — where a splice's corner sits.
    earlier: &'grid [u32],
    /// This row's subtree distances, from the keyroot's leftmost leaf.
    subtree: &'grid mut [u32],
}

/// Computes one forest-distance cell, recording a subtree distance when
/// both prefixes are whole subtrees.
///
/// The three Zhang–Shasha options: delete the left node, insert the
/// right one, or — the third — relabel two whole subtrees against each
/// other, or splice in an already-computed subtree distance.
fn cell(row: Row, cursor: Cursor<'_>, grids: Grids<'_>) -> u32 {
    let whole = row.left_whole && cursor.node.leftmost == row.right_leaf;
    let substitute = if whole {
        read(grids.above, cursor.offset)
            .saturating_add(u32::from(row.left_kind != cursor.node.kind))
    } else {
        let right_prefix = cursor.node.leftmost.saturating_sub(row.right_leaf);
        read(grids.earlier, row.corner_base.saturating_add(right_prefix))
            .saturating_add(read(grids.subtree, cursor.offset))
    };
    let best = read(grids.above, cursor.offset.saturating_add(1))
        .saturating_add(1)
        .min(cursor.previous.saturating_add(1))
        .min(substitute);
    if whole {
        write(grids.subtree, cursor.offset, best);
    }
    best
}

/// Cell value; [`u32::MAX`] out of range, so a min-fold can never elect
/// an access the algorithm's own indexing never makes.
fn read(cells: &[u32], slot: usize) -> u32 {
    cells.get(slot).copied().unwrap_or(u32::MAX)
}

/// Writes a cell, ignoring out-of-range writes.
fn write(cells: &mut [u32], slot: usize, value: u32) {
    if let Some(cell) = cells.get_mut(slot) {
        *cell = value;
    }
}

/// A grid coordinate as a cell value. Coordinates are bounded by the
/// node cap, far below [`u32::MAX`].
fn small(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
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
