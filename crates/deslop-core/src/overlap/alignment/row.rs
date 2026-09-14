//! One row of the Zhang–Shasha forest-distance recurrence.

use super::{kind_at, read, ForestSpan, PostNode};

/// Row offsets and invariant left-node state.
#[derive(Clone, Copy)]
struct Row {
    /// Grid offset of this row's column zero.
    base: usize,
    /// Grid offset of the previous row's column zero.
    above: usize,
    /// Whether the left node at this row roots a whole subtree.
    left_whole: bool,
    /// Grid offset of a splice cell's upper-left forest corner.
    corner_base: usize,
    /// Leftmost leaf of the right keyroot.
    right_leaf: usize,
    /// Offset of this row's first subtree-distance cell.
    subtree_base: usize,
    /// Left node's kind.
    left_kind: &'static str,
}

impl Row {
    /// Resolves every invariant needed by a row's cells.
    fn at(span: ForestSpan<'_>, left_index: usize, left_leftmost: usize, base: usize) -> Self {
        Self {
            base,
            above: base.saturating_sub(span.forest_stride),
            left_whole: left_leftmost == span.left_leaf,
            corner_base: corner_base(span, left_leftmost),
            right_leaf: span.right_leaf,
            subtree_base: subtree_base(span, left_index),
            left_kind: kind_at(span.left, left_index),
        }
    }
}

/// Equal-length slices walked by one row.
struct Cells<'seq, 'scratch> {
    /// Right-side nodes, in post-order.
    nodes: &'seq [PostNode],
    /// Previous-row windows of two cells.
    above: &'scratch [u32],
    /// Forest cells before this row, for splice corners.
    earlier: &'scratch [u32],
    /// Writable interior cells in this row.
    current: &'scratch mut [u32],
    /// Writable subtree-distance cells aligned with `nodes`.
    subtree: &'scratch mut [u32],
}

/// Fills one forest-grid row from pre-resolved slices.
#[cfg_attr(feature = "profile-internals", inline(never))]
pub(super) fn fill(
    span: ForestSpan<'_>,
    left_index: usize,
    left_leftmost: usize,
    base: usize,
    forest: &mut [u32],
    tree_dist: &mut [u32],
) {
    let row = Row::at(span, left_index, left_leftmost, base);
    let Some((previous, cells)) = resolve_cells(span, row, forest, tree_dist) else {
        return;
    };
    fold_cells(row, cells, previous);
}

/// Row offset of a non-whole splice's upper-left corner.
fn corner_base(span: ForestSpan<'_>, left_leftmost: usize) -> usize {
    left_leftmost
        .saturating_sub(span.left_leaf)
        .saturating_mul(span.forest_stride)
}

/// First subtree-distance slot written by a row.
fn subtree_base(span: ForestSpan<'_>, left_index: usize) -> usize {
    left_index
        .saturating_mul(span.tree_stride)
        .saturating_add(span.right_leaf)
}

/// Resolves and trims the equal-length slices walked by a row.
fn resolve_cells<'seq, 'scratch>(
    span: ForestSpan<'seq>,
    row: Row,
    forest: &'scratch mut [u32],
    tree_dist: &'scratch mut [u32],
) -> Option<(u32, Cells<'seq, 'scratch>)> {
    let nodes = span
        .right
        .get(span.right_leaf.saturating_sub(1)..span.right_root)?;
    let (earlier, current) = forest.split_at_mut(row.base);
    let above = earlier.get(row.above..)?;
    let subtree = tree_dist.get_mut(row.subtree_base..)?;
    trim_cells(nodes, above, earlier, current, subtree)
}

/// Trims scratch slices and separates the seeded first cell.
fn trim_cells<'seq, 'scratch>(
    nodes: &'seq [PostNode],
    above: &'scratch [u32],
    earlier: &'scratch [u32],
    current: &'scratch mut [u32],
    subtree: &'scratch mut [u32],
) -> Option<(u32, Cells<'seq, 'scratch>)> {
    let columns = nodes.len();
    let above = above.get(..columns.saturating_add(1))?;
    let current = current.get_mut(..columns.saturating_add(1))?;
    let subtree = subtree.get_mut(..columns)?;
    let (first, current) = current.split_first_mut()?;
    Some((*first, cells(nodes, above, earlier, current, subtree)))
}

/// Bundles validated slices for the recurrence walkers.
fn cells<'seq, 'scratch>(
    nodes: &'seq [PostNode],
    above: &'scratch [u32],
    earlier: &'scratch [u32],
    current: &'scratch mut [u32],
    subtree: &'scratch mut [u32],
) -> Cells<'seq, 'scratch> {
    Cells {
        nodes,
        above,
        earlier,
        current,
        subtree,
    }
}

/// Selects the monomorphised recurrence for this row shape.
fn fold_cells(row: Row, cells: Cells<'_, '_>, previous: u32) {
    if row.left_whole {
        fold_whole(row, cells, previous);
    } else {
        fold_spliced(row, cells, previous);
    }
}

/// Walks a row that may complete whole-subtree pairs.
fn fold_whole(row: Row, cells: Cells<'_, '_>, previous: u32) {
    let Cells {
        nodes,
        above,
        earlier,
        current,
        subtree,
    } = cells;
    fold_row(
        nodes,
        above,
        current,
        subtree,
        previous,
        |node, prior, pair, cell| whole_cell(row, node, prior, pair, earlier, cell),
    );
}

/// Walks a row whose cells all splice completed subtree distances.
fn fold_spliced(row: Row, cells: Cells<'_, '_>, previous: u32) {
    let Cells {
        nodes,
        above,
        earlier,
        current,
        subtree,
    } = cells;
    fold_row(
        nodes,
        above,
        current,
        subtree,
        previous,
        |node, prior, pair, cell| splice_cell(row, node, prior, pair, earlier, *cell),
    );
}

/// Walks equal-length row slices with one cell evaluator.
fn fold_row(
    nodes: &[PostNode],
    above: &[u32],
    current: &mut [u32],
    subtree: &mut [u32],
    mut previous: u32,
    mut evaluate: impl FnMut(&PostNode, u32, &[u32], &mut u32) -> u32,
) {
    let rows = nodes.iter().zip(above.windows(2)).zip(current).zip(subtree);
    for (((node, above), current), subtree) in rows {
        previous = evaluate(node, previous, above, subtree);
        *current = previous;
    }
}

/// Computes a cell that may complete a whole subtree pair.
fn whole_cell(
    row: Row,
    node: &PostNode,
    previous: u32,
    above: &[u32],
    earlier: &[u32],
    subtree: &mut u32,
) -> u32 {
    if node.leftmost != row.right_leaf {
        return splice_cell(row, node, previous, above, earlier, *subtree);
    }
    let Some((&above_left, _above_right)) = above.split_first() else {
        return previous;
    };
    let substitute = above_left.saturating_add(u32::from(row.left_kind != node.kind));
    let best = choose(previous, above, substitute);
    *subtree = best;
    best
}

/// Computes a splice cell whose prefixes are not both whole subtrees.
fn splice_cell(
    row: Row,
    node: &PostNode,
    previous: u32,
    above: &[u32],
    earlier: &[u32],
    subtree: u32,
) -> u32 {
    let right_prefix = node.leftmost.saturating_sub(row.right_leaf);
    let corner = row.corner_base.saturating_add(right_prefix);
    let substitute = read(earlier, corner).saturating_add(subtree);
    choose(previous, above, substitute)
}

/// Minimum delete, insert, or substitute cost for one cell.
fn choose(previous: u32, above: &[u32], substitute: u32) -> u32 {
    let [_, above_right] = above else {
        return previous;
    };
    above_right
        .saturating_add(1)
        .min(previous.saturating_add(1))
        .min(substitute)
}
