//! Shared-subtree structural overlap ([FUSION-SHARED-SUBTREE], gh #408).
//!
//! `pair.rs` documents `structural` as "the best-achievable subtree
//! overlap", but the candidate layer wrote a literal `0.0` for every
//! cross-bucket pair — while the unchanged statements inside a Type-3
//! near-miss are Merkle-identical, which is exactly why fragment views
//! of the same clone survive. This module measures that overlap.
//!
//! The measure is ordered tree alignment: `1 - TED / max(nodes)`,
//! where `TED` is the Zhang–Shasha tree edit distance over the
//! normalised kinds with unit insert/delete/relabel costs. A
//! one-statement Type-3 insertion costs exactly the inserted subtree,
//! so the genuine near-miss measures high while two unrelated
//! functions that merely share statement vocabulary measure low — a
//! multiset of shared subtree hashes cannot tell those apart, because
//! the discriminating information is in the *order and nesting* of the
//! matches, which is precisely what an alignment scores and a multiset
//! discards.
//!
//! Endpoints past [`ALIGNMENT_MAX_NODES`] fall back to greedy maximal
//! shared-Merkle-subtree coverage — a conservative lower bound on the
//! aligned overlap. The bound converges to the alignment as trees
//! grow: its error is the root-to-edit spine, whose share of the tree
//! vanishes at exactly the sizes the fallback covers. A lower bound
//! can suppress a rescue, never manufacture one.

use std::{collections::HashMap, rc::Rc};

use crate::{
    ast::NormalizedNode,
    fingerprint::{collect_fingerprints, Fingerprint},
    pair::{CandidatePair, SHARED_SUBTREE_MIN_JACCARD, SHARED_SUBTREE_MIN_NODE_COUNT},
    state::FileId,
    tokens::resolve_range_nodes,
};

/// Largest endpoint (in nodes) measured by exact tree alignment. The
/// Zhang–Shasha DP is quadratic in nodes; past this size the greedy
/// coverage bound takes over, where its spine error is already
/// negligible.
pub const ALIGNMENT_MAX_NODES: usize = 512;

/// Smallest shared subtree creditable by the large-tree coverage
/// fallback. Normalisation interns single leaves down to their kind
/// (`__ident__` matches `__ident__` everywhere), so leaf-level matches
/// measure the language's grammar, not the code.
pub const SHARED_SUBTREE_MIN_CREDIT_NODES: usize = 3;

/// Measures shared-subtree overlap onto every candidate pair the fused
/// threshold would otherwise drop despite corroborating token evidence
/// ([FUSION-SHARED-SUBTREE]). Only those pairs are measured: aligning
/// two subtrees for all candidates would repeat the admission-cost
/// mistake [FUSION-CONTENT-GATE] deliberately avoids, and a pair that
/// already survives needs no rescue.
pub fn apply_shared_subtree_rescue(
    pairs: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
) {
    let mut measurer = OverlapMeasurer::new(trees);
    let mut measured = 0_usize;
    for pair in pairs.iter_mut().filter(|pair| rescue_eligible(pair)) {
        let (Some(left), Some(right)) =
            (fingerprints.get(pair.left), fingerprints.get(pair.right))
        else {
            continue;
        };
        pair.shared_subtree_overlap = measurer.overlap(left, right);
        tracing::debug!(
            left_nodes = left.node_count,
            right_nodes = right.node_count,
            token_jaccard = pair.score.token_jaccard,
            overlap = pair.shared_subtree_overlap,
            "shared-subtree overlap measured"
        );
        measured = measured.saturating_add(1);
    }
    tracing::debug!(measured, "shared-subtree rescue overlaps measured");
}

/// True for a pair worth measuring: dropped below its fused floor on a
/// zero structural anchor, yet carrying the token corroboration and
/// endpoint substance the rescue route requires.
fn rescue_eligible(pair: &CandidatePair) -> bool {
    let score = pair.score.finite();
    score.structural <= 0.0
        && score.bounded_fused() < pair.fused_min_score
        && score.token_jaccard >= SHARED_SUBTREE_MIN_JACCARD
        && pair.endpoint_node_counts.0 >= SHARED_SUBTREE_MIN_NODE_COUNT
}

/// Measures shared-subtree overlap between fingerprint endpoints over
/// one corpus, memoising per-endpoint views and per-pair results so an
/// endpoint appearing in many pairs is walked once.
#[derive(Debug)]
pub struct OverlapMeasurer<'corpus> {
    /// `FileId → normalised root` for the corpus under measurement.
    tree_index: HashMap<FileId, &'corpus NormalizedNode>,
    /// Per-endpoint resolved state. `None` records an unresolvable
    /// range so it is not re-walked per pair.
    endpoints: HashMap<EndpointKey, Option<Rc<EndpointView>>>,
    /// Per-pair measured overlap, keyed order-insensitively.
    pair_results: HashMap<(EndpointKey, EndpointKey), f64>,
}

/// Identity of one endpoint's resolved range.
type EndpointKey = (FileId, usize, usize);

/// One endpoint's resolved measurement state.
#[derive(Debug)]
struct EndpointView {
    /// Post-order `(kind, leftmost-leaf index)` sequence under a
    /// synthetic window root, for the alignment.
    postorder: Vec<PostNode>,
    /// Total nodes excluding the synthetic root.
    total: usize,
    /// Creditable subtrees for the large-tree fallback, largest first.
    entries: Vec<Fingerprint>,
    /// Hash → multiplicity over `entries`.
    counts: HashMap<[u8; 32], usize>,
}

/// One node of the post-order sequence.
#[derive(Debug, Clone, Copy)]
struct PostNode {
    /// Normalised kind.
    kind: &'static str,
    /// 1-based post-order index of this node's leftmost leaf.
    leftmost: usize,
}

impl<'corpus> OverlapMeasurer<'corpus> {
    /// Builds a measurer over the corpus trees.
    #[must_use]
    pub fn new(trees: &'corpus [NormalizedNode]) -> Self {
        Self {
            tree_index: trees.iter().map(|tree| (tree.file_id, tree)).collect(),
            endpoints: HashMap::new(),
            pair_results: HashMap::new(),
        }
    }

    /// Shared-subtree overlap between two endpoints in `[0, 1]`.
    ///
    /// `1.0` requires Merkle equality of the endpoints themselves; a
    /// non-equal pair is bounded below `1.0` because an alignment of
    /// unequal trees costs at least one edit. `0.0` when either
    /// endpoint's byte range does not resolve to a node or sibling
    /// window in its tree — exactly the pairs the old literal `0.0`
    /// described honestly.
    pub fn overlap(&mut self, left: &Fingerprint, right: &Fingerprint) -> f64 {
        if left.hash == right.hash {
            return 1.0;
        }
        let pair_key = ordered_key(endpoint_key(left), endpoint_key(right));
        if let Some(&cached) = self.pair_results.get(&pair_key) {
            return cached;
        }
        let result = self.measure(left, right);
        let _previous = self.pair_results.insert(pair_key, result);
        result
    }

    /// Measures one uncached, non-equal pair.
    fn measure(&mut self, left: &Fingerprint, right: &Fingerprint) -> f64 {
        let (Some(left_view), Some(right_view)) = (self.view(left), self.view(right)) else {
            return 0.0;
        };
        let larger = left_view.total.max(right_view.total);
        if larger == 0 {
            return 0.0;
        }
        let shared = if larger > ALIGNMENT_MAX_NODES {
            credit_shared_nodes(&left_view, &right_view)
        } else {
            aligned_shared_nodes(&left_view, &right_view)
        };
        (lossless_count(shared) / lossless_count(larger)).clamp(0.0, 1.0)
    }

    /// Returns (building on first use) the endpoint's resolved view.
    fn view(&mut self, endpoint: &Fingerprint) -> Option<Rc<EndpointView>> {
        let key = endpoint_key(endpoint);
        if let Some(cached) = self.endpoints.get(&key) {
            return cached.clone();
        }
        let built = build_view(&self.tree_index, endpoint).map(Rc::new);
        let _previous = self.endpoints.insert(key, built.clone());
        built
    }
}

/// The endpoint's cache identity.
fn endpoint_key(endpoint: &Fingerprint) -> EndpointKey {
    (
        endpoint.file_id,
        endpoint.byte_range.start,
        endpoint.byte_range.end,
    )
}

/// Order-insensitive pair cache key.
fn ordered_key(left: EndpointKey, right: EndpointKey) -> (EndpointKey, EndpointKey) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

/// Resolves the endpoint's nodes and builds both measurement inputs.
/// Resolution reuses [`resolve_range_nodes`] — the same resolver the
/// token stream and content walks use — so every signal sees the same
/// code, including synthetic sibling windows.
fn build_view(
    tree_index: &HashMap<FileId, &NormalizedNode>,
    endpoint: &Fingerprint,
) -> Option<EndpointView> {
    let root = tree_index.get(&endpoint.file_id)?;
    let members = resolve_range_nodes(root, endpoint.byte_range.start, endpoint.byte_range.end)?;
    let mut postorder: Vec<PostNode> = Vec::new();
    let mut entries: Vec<Fingerprint> = Vec::new();
    for member in &members {
        push_postorder(member, &mut postorder);
        entries.extend(collect_fingerprints(member, SHARED_SUBTREE_MIN_CREDIT_NODES));
    }
    let total = postorder.len();
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
    let mut counts: HashMap<[u8; 32], usize> = HashMap::new();
    for entry in &entries {
        let slot = counts.entry(entry.hash).or_insert(0);
        *slot = slot.saturating_add(1);
    }
    Some(EndpointView {
        postorder,
        total,
        entries,
        counts,
    })
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

/// Appends `node`'s subtree to `out` in post-order, recording each
/// node's leftmost-leaf index. Iterative so a deep tree cannot
/// overflow the stack (matching `fingerprint::hash_and_collect`).
fn push_postorder(node: &NormalizedNode, out: &mut Vec<PostNode>) {
    let mut stack = vec![WalkFrame {
        node,
        next_child: 0,
        leftmost: None,
    }];
    while let Some(frame) = stack.last_mut() {
        if let Some(child) = frame.node.children.get(frame.next_child) {
            frame.next_child = frame.next_child.saturating_add(1);
            stack.push(WalkFrame {
                node: child,
                next_child: 0,
                leftmost: None,
            });
            continue;
        }
        let position = out.len().saturating_add(1);
        let leftmost = frame.leftmost.unwrap_or(position);
        out.push(PostNode {
            kind: frame.node.kind,
            leftmost,
        });
        let _finished = stack.pop();
        if let Some(parent) = stack.last_mut() {
            if parent.leftmost.is_none() {
                parent.leftmost = Some(leftmost);
            }
        }
    }
}

/// Shared node count under the optimal ordered alignment:
/// `max(total) - TED`, floored at zero. With unit costs the distance
/// is the node mass the alignment could not match, so this is the
/// aligned analogue of the fallback's credited mass.
fn aligned_shared_nodes(left: &EndpointView, right: &EndpointView) -> usize {
    let distance = tree_edit_distance(&left.postorder, &right.postorder);
    left.total.max(right.total).saturating_sub(distance)
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
    let mut forest = Grid::new(
        left_root.saturating_sub(span.left_leaf).saturating_add(2),
        right_root.saturating_sub(span.right_leaf).saturating_add(2),
    );
    seed_edit_borders(&mut forest);
    for left_index in span.left_leaf..=left_root {
        for right_index in span.right_leaf..=right_root {
            fill_cell(span, left_index, right_index, &mut forest, tree_dist);
        }
    }
}

/// Seeds the first row and column with pure insert/delete costs.
fn seed_edit_borders(forest: &mut Grid) {
    for row in 1..forest.rows() {
        forest.set(row, 0, row);
    }
    for column in 1..forest.columns() {
        forest.set(0, column, column);
    }
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
    let row = left_index.saturating_sub(span.left_leaf).saturating_add(1);
    let column = right_index
        .saturating_sub(span.right_leaf)
        .saturating_add(1);
    let delete = forest.get(row.saturating_sub(1), column).saturating_add(1);
    let insert = forest.get(row, column.saturating_sub(1)).saturating_add(1);
    let whole_trees = leftmost(span.left, left_index) == span.left_leaf
        && leftmost(span.right, right_index) == span.right_leaf;
    let substitute = if whole_trees {
        let relabel =
            usize::from(kind_at(span.left, left_index) != kind_at(span.right, right_index));
        forest
            .get(row.saturating_sub(1), column.saturating_sub(1))
            .saturating_add(relabel)
    } else {
        let left_prefix = leftmost(span.left, left_index).saturating_sub(span.left_leaf);
        let right_prefix = leftmost(span.right, right_index).saturating_sub(span.right_leaf);
        forest
            .get(left_prefix, right_prefix)
            .saturating_add(tree_dist.get(left_index, right_index))
    };
    let best = delete.min(insert).min(substitute);
    forest.set(row, column, best);
    if whole_trees {
        tree_dist.set(left_index, right_index, best);
    }
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

/// Large-tree fallback: greedy-maximal shared-Merkle-subtree node
/// credit. Largest left subtrees first, one right-multiset consumption
/// per credit, nested-in-credited skipped. A conservative lower bound
/// on [`aligned_shared_nodes`] — node mass matched under a bijection
/// of identical subtrees is achievable by an alignment.
fn credit_shared_nodes(left: &EndpointView, right: &EndpointView) -> usize {
    let mut remaining = right.counts.clone();
    let mut taken: Vec<(usize, usize)> = Vec::new();
    let mut credit = 0_usize;
    for entry in &left.entries {
        let start = entry.byte_range.start;
        let end = entry.byte_range.end;
        if taken
            .iter()
            .any(|(taken_start, taken_end)| *taken_start <= start && end <= *taken_end)
        {
            continue;
        }
        let Some(count) = remaining.get_mut(&entry.hash) else {
            continue;
        };
        if *count == 0 {
            continue;
        }
        *count = count.saturating_sub(1);
        credit = credit.saturating_add(entry.node_count);
        taken.push((start, end));
    }
    credit
}

/// Lossless small-count conversion for the coverage divisor.
fn lossless_count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}
