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

/// Zhang–Shasha ordered tree alignment ([FUSION-SHARED-SUBTREE]).
mod alignment;

/// Measurement unit tests ([FUSION-SHARED-SUBTREE]).
#[cfg(test)]
mod tests;
use alignment::{aligned_shared_nodes, PostNode};

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
///
/// The unit is nodes of the *normalised* tree, so
/// [PIPELINE-NORMALIZE-AST-OPERATOR] moved what the number reaches
/// without anyone changing it: operator tokens now survive as leaves, and
/// an operator-dense expression counts around half as many nodes again.
/// At 512 that silently pulled `ts-mixed-band`'s ninety-term expression —
/// 558 nodes, a consistent rename plus one redundant paren, the case
/// [FUSION-SHARED-SUBTREE] exists to rescue — onto the conservative bound,
/// which scored it under the admission floor and reported nothing
/// (`without_embeddings_the_mid_band_pair_is_visible_without_saturating`).
/// The cap must reach the largest endpoint the admission path is expected
/// to rescue, so it is set above the largest such pinned case with room to
/// spare rather than trimmed to it.
pub const ALIGNMENT_MAX_NODES: usize = 768;

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
    let mut rescued_pairs = 0_usize;
    for pair in pairs.iter_mut() {
        if !rescue_eligible(pair) || !crosses_files(pair, fingerprints) {
            continue;
        }
        if measure_onto(pair, fingerprints, &mut measurer) {
            rescued_pairs = rescued_pairs.saturating_add(1);
        }
    }
    tracing::debug!(rescued_pairs, "shared-subtree rescue overlaps measured");
}

/// Measures one eligible pair, returning whether both endpoints
/// resolved.
fn measure_onto(
    pair: &mut CandidatePair,
    fingerprints: &[Fingerprint],
    measurer: &mut OverlapMeasurer<'_>,
) -> bool {
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return false;
    };
    pair.shared_subtree_overlap = measurer.overlap(left, right);
    tracing::debug!(
        left_nodes = left.node_count,
        right_nodes = right.node_count,
        token_jaccard = pair.score.token_jaccard,
        overlap = pair.shared_subtree_overlap,
        "shared-subtree overlap measured"
    );
    true
}

/// True when the pair's endpoints live in different files.
///
/// The rescue is deliberately cross-file only. Every clone this route
/// exists to recover is a copy *between* files ([FUSION-SHARED-SUBTREE],
/// gh #408), and admitting same-file pairs on shape overlap is the
/// #197 in-file sibling-family shape, which the report already spends a
/// dedicated proof suppressing. It is also what keeps a single-file
/// corpus intact: same-file rescues union that file's subtrees into one
/// transitive component, and the same-file overlap collapse then
/// reduces it to a single logical location, which is dropped below
/// `MIN_REPORTABLE_MEMBERS` — so the file's real duplication
/// disappeared entirely rather than being reported
/// (`issue_119_role_gate_exercised`).
fn crosses_files(pair: &CandidatePair, fingerprints: &[Fingerprint]) -> bool {
    match (fingerprints.get(pair.left), fingerprints.get(pair.right)) {
        (Some(left), Some(right)) => left.file_id != right.file_id,
        _ => false,
    }
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
}

impl EndpointView {
    /// Builds a view over a flat run of leaves under the synthetic
    /// window root — the minimal shape for asserting the alignment's
    /// arithmetic directly, without a parser in the way.
    #[cfg(test)]
    fn from_flat_leaves(kinds: &[&'static str]) -> Self {
        let mut postorder: Vec<PostNode> = kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| PostNode {
                kind,
                leftmost: index.saturating_add(1),
            })
            .collect();
        let total = postorder.len();
        postorder.push(PostNode {
            kind: "__window__",
            leftmost: 1,
        });
        Self {
            postorder,
            total,
            entries: Vec::new(),
        }
    }

    /// Post-order sequence, including the synthetic window root.
    fn postorder(&self) -> &[PostNode] {
        &self.postorder
    }

    /// Node total, excluding the synthetic window root.
    const fn total(&self) -> usize {
        self.total
    }
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
        entries.extend(collect_fingerprints(
            member,
            SHARED_SUBTREE_MIN_CREDIT_NODES,
        ));
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
    Some(EndpointView {
        postorder,
        total,
        entries,
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

/// Large-tree fallback: greedy-maximal shared-Merkle-subtree node
/// credit. Largest left subtrees first, each credit consuming one
/// concrete right-side occurrence, nested-in-credited spans skipped on
/// **both** endpoints. A conservative lower bound on
/// [`aligned_shared_nodes`] — node mass matched under a bijection of
/// disjoint identical subtrees is achievable by an alignment. The
/// bijection needs both sides tracked: consuming bare hash counts on
/// the right let a disjoint left copy re-claim nodes nested inside an
/// already-credited right subtree, counting them twice and overshooting
/// the alignment this bound stands in for
/// (`the_fallback_never_credits_a_nested_right_subtree_twice`).
///
/// Left entries arrive largest-first, so every candidate span is no
/// larger than the spans already credited on its side; a strict
/// container has strictly more nodes than its subtree, so a later
/// candidate can never contain a credited span and the nested-inside
/// test alone keeps each side's credited spans disjoint.
fn credit_shared_nodes(left: &EndpointView, right: &EndpointView) -> usize {
    let mut open_right: HashMap<[u8; 32], Vec<(usize, usize)>> = HashMap::new();
    for entry in &right.entries {
        open_right
            .entry(entry.hash)
            .or_default()
            .push((entry.byte_range.start, entry.byte_range.end));
    }
    let mut left_taken: Vec<(usize, usize)> = Vec::new();
    let mut right_taken: Vec<(usize, usize)> = Vec::new();
    let mut credit = 0_usize;
    for entry in &left.entries {
        let span = (entry.byte_range.start, entry.byte_range.end);
        if nested_in_credited(span, &left_taken) {
            continue;
        }
        let Some(claimed) = claim_right_occurrence(entry.hash, &mut open_right, &right_taken)
        else {
            continue;
        };
        credit = credit.saturating_add(entry.node_count);
        left_taken.push(span);
        right_taken.push(claimed);
    }
    credit
}

/// True when `span` nests inside any already-credited span.
fn nested_in_credited(span: (usize, usize), taken: &[(usize, usize)]) -> bool {
    let (start, end) = span;
    taken
        .iter()
        .any(|(taken_start, taken_end)| *taken_start <= start && end <= *taken_end)
}

/// Consumes and returns one right-side occurrence of `hash` that is not
/// nested inside an already-credited right span. Identical hashes have
/// identical node counts, so any open occurrence is an equally-sized
/// witness and the first open one serves.
fn claim_right_occurrence(
    hash: [u8; 32],
    open_right: &mut HashMap<[u8; 32], Vec<(usize, usize)>>,
    right_taken: &[(usize, usize)],
) -> Option<(usize, usize)> {
    let candidates = open_right.get_mut(&hash)?;
    let position = candidates
        .iter()
        .position(|candidate| !nested_in_credited(*candidate, right_taken))?;
    Some(candidates.swap_remove(position))
}

/// Lossless small-count conversion for the coverage divisor.
fn lossless_count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}
