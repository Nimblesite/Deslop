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
//!
//! Measurement is memoised by the ordered pair of endpoint Merkle
//! hashes ([FUSION-SHARED-SUBTREE-MEMO]): hash equality pins the whole
//! normalised structure — the same premise as the `1.0` short-circuit —
//! so a corpus holding many byte-offset copies of one window costs one
//! alignment per *distinct structural pair*, not one per byte-range
//! combination. The rescue path additionally refuses the quadratic
//! alignment outright when a sound constant-per-node upper bound already
//! proves the pair cannot clear the admission floor
//! ([FUSION-SHARED-SUBTREE-BOUND]).

use std::{collections::HashMap, rc::Rc};

/// Zhang–Shasha ordered tree alignment ([FUSION-SHARED-SUBTREE]).
mod alignment;
/// Large-tree greedy coverage fallback ([FUSION-SHARED-SUBTREE]).
mod credit;
/// Rescue application over the candidate set ([FUSION-SHARED-SUBTREE]).
mod rescue;
/// Rescue-pass gate counters ([PERF-FLUTTER-TODO-OBSERVABILITY]).
mod tally;
/// Endpoint view construction ([FUSION-SHARED-SUBTREE]).
mod view;

/// Measurement unit tests ([FUSION-SHARED-SUBTREE]).
#[cfg(test)]
mod tests;

pub use rescue::apply_shared_subtree_rescue;

use alignment::aligned_shared_nodes;
use view::{build_view, EndpointView};

use crate::{
    ast::NormalizedNode, fingerprint::Fingerprint, observe::bump, pair::SHARED_SUBTREE_MIN_OVERLAP,
    state::FileId,
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

/// Aggregate measurement counters for one [`OverlapMeasurer`]
/// ([FUSION-SHARED-SUBTREE-MEMO], [PIPELINE-OBSERVABILITY-STAGES]).
/// Snapshot via [`OverlapMeasurer::stats`]; the rescue and cluster
/// stages log them so cache effectiveness and alignment volume are
/// readable from any run.
#[derive(Debug, Default, Clone, Copy)]
pub struct MeasureStats {
    /// Pairs answered `1.0` by Merkle equality of the endpoints.
    pub hash_equal: u64,
    /// Pairs answered from the exact-overlap memo.
    pub exact_hits: u64,
    /// Rescue queries answered from the below-floor bound memo.
    pub bound_hits: u64,
    /// Rescue queries whose freshly computed kind-multiset bound proved
    /// the pair cannot clear the floor, skipping the alignment
    /// ([FUSION-SHARED-SUBTREE-BOUND]).
    pub bound_skips: u64,
    /// Exact Zhang–Shasha alignments computed.
    pub alignments: u64,
    /// Greedy large-tree credit fallbacks computed.
    pub credit_fallbacks: u64,
    /// Pairs with an unresolvable endpoint, reported `0.0` uncached.
    pub unresolved: u64,
}

/// Measures shared-subtree overlap between fingerprint endpoints over
/// one corpus, memoising per-endpoint views and per-structural-pair
/// results so an endpoint appearing in many pairs is walked once and a
/// structure appearing at many byte offsets is aligned once
/// ([FUSION-SHARED-SUBTREE-MEMO]).
#[derive(Debug)]
pub struct OverlapMeasurer<'corpus> {
    /// `FileId → normalised root` for the corpus under measurement.
    tree_index: HashMap<FileId, &'corpus NormalizedNode>,
    /// Per-endpoint resolved state. `None` records an unresolvable
    /// range so it is not re-walked per pair.
    endpoints: HashMap<EndpointKey, Option<Rc<EndpointView>>>,
    /// Exact measured overlap per structural pair.
    exact_results: HashMap<PairKey, f64>,
    /// Below-floor upper bounds per structural pair, usable only by the
    /// rescue's floor comparison ([FUSION-SHARED-SUBTREE-BOUND]) —
    /// never as an exact value.
    bound_results: HashMap<PairKey, f64>,
    /// Aggregate counters ([PIPELINE-OBSERVABILITY-STAGES]).
    stats: MeasureStats,
}

/// Identity of one endpoint's resolved range, for the view memo.
type EndpointKey = (FileId, usize, usize);

/// Memo key for a measured pair: the ordered Merkle hashes of the two
/// endpoints ([FUSION-SHARED-SUBTREE-MEMO]). Hash equality pins the
/// whole normalised structure — the premise the `1.0` short-circuit
/// already stands on — so every byte-offset copy of one structural
/// pair shares this key, and the measurement runs once per *structure*
/// rather than once per byte-range combination
/// (`a_fleet_of_identical_windows_costs_one_alignment`).
type PairKey = ([u8; 32], [u8; 32]);

impl<'corpus> OverlapMeasurer<'corpus> {
    /// Builds a measurer over the corpus trees.
    #[must_use]
    pub fn new(trees: &'corpus [NormalizedNode]) -> Self {
        Self {
            tree_index: trees.iter().map(|tree| (tree.file_id, tree)).collect(),
            endpoints: HashMap::new(),
            exact_results: HashMap::new(),
            bound_results: HashMap::new(),
            stats: MeasureStats::default(),
        }
    }

    /// Snapshot of the aggregate measurement counters.
    #[must_use]
    pub const fn stats(&self) -> MeasureStats {
        self.stats
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
            bump(&mut self.stats.hash_equal);
            return 1.0;
        }
        let key = pair_key(left, right);
        if let Some(&cached) = self.exact_results.get(&key) {
            bump(&mut self.stats.exact_hits);
            return cached;
        }
        let Some((left_view, right_view)) = self.view_pair(left, right) else {
            return 0.0;
        };
        let result = self.measure_views(&left_view, &right_view);
        let _previous = self.exact_results.insert(key, result);
        result
    }

    /// Overlap for the rescue's floor comparison
    /// ([FUSION-SHARED-SUBTREE-BOUND]). Identical to [`Self::overlap`]
    /// whenever the pair could clear `SHARED_SUBTREE_MIN_OVERLAP`; when
    /// a sound upper bound already proves it cannot, the bound itself —
    /// strictly below the floor — is returned without running the
    /// alignment. The admission decision is identical by construction;
    /// the value differs only on pairs the rescue then drops, where it
    /// is never rendered.
    pub fn rescue_overlap(&mut self, left: &Fingerprint, right: &Fingerprint) -> f64 {
        if left.hash == right.hash {
            bump(&mut self.stats.hash_equal);
            return 1.0;
        }
        let key = pair_key(left, right);
        if let Some(&cached) = self.exact_results.get(&key) {
            bump(&mut self.stats.exact_hits);
            return cached;
        }
        if let Some(&bound) = self.bound_results.get(&key) {
            bump(&mut self.stats.bound_hits);
            return bound;
        }
        let Some((left_view, right_view)) = self.view_pair(left, right) else {
            return 0.0;
        };
        let bound = kind_bound_ratio(&left_view, &right_view);
        if bound < SHARED_SUBTREE_MIN_OVERLAP {
            bump(&mut self.stats.bound_skips);
            let _previous = self.bound_results.insert(key, bound);
            return bound;
        }
        let result = self.measure_views(&left_view, &right_view);
        let _previous = self.exact_results.insert(key, result);
        result
    }

    /// Resolves both endpoint views, counting a pair with either side
    /// unresolvable. Unresolvable pairs are deliberately not memoised
    /// under the pair key: resolvability is a property of the byte
    /// range, not of the structure the key describes.
    fn view_pair(
        &mut self,
        left: &Fingerprint,
        right: &Fingerprint,
    ) -> Option<(Rc<EndpointView>, Rc<EndpointView>)> {
        let views = self.view(left).zip(self.view(right));
        if views.is_none() {
            bump(&mut self.stats.unresolved);
        }
        views
    }

    /// Measures one resolved, non-equal pair.
    fn measure_views(&mut self, left: &EndpointView, right: &EndpointView) -> f64 {
        let larger = left.total.max(right.total);
        if larger == 0 {
            return 0.0;
        }
        let shared = if larger > ALIGNMENT_MAX_NODES {
            bump(&mut self.stats.credit_fallbacks);
            credit::credit_shared_nodes(left, right)
        } else {
            bump(&mut self.stats.alignments);
            aligned_shared_nodes(left, right)
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

/// The endpoint's view-memo identity.
fn endpoint_key(endpoint: &Fingerprint) -> EndpointKey {
    (
        endpoint.file_id,
        endpoint.byte_range.start,
        endpoint.byte_range.end,
    )
}

/// Order-insensitive memo key for a measured pair
/// ([FUSION-SHARED-SUBTREE-MEMO]).
fn pair_key(left: &Fingerprint, right: &Fingerprint) -> PairKey {
    if left.hash <= right.hash {
        (left.hash, right.hash)
    } else {
        (right.hash, left.hash)
    }
}

/// Sound upper bound on the alignment's shared-node count
/// ([FUSION-SHARED-SUBTREE-BOUND]). Any edit script maps some set `M`
/// of node pairs; its cost is `deletes + inserts + relabels =
/// larger + smaller − 2|M| + relabels`, so the shared mass
/// `larger − TED` never exceeds the kind-preserving part of `M` — which
/// is bounded by the smaller endpoint and by the kind-multiset
/// intersection. Both bounds are constant per node, so refusing an
/// alignment here can never refuse a pair the alignment would admit.
fn kind_shared_upper_bound(left: &EndpointView, right: &EndpointView) -> usize {
    let smaller = left.total.min(right.total);
    smaller.min(kind_intersection(&left.kind_counts, &right.kind_counts))
}

/// Multiset-intersection cardinality of two kind-count maps.
fn kind_intersection(
    left: &HashMap<&'static str, usize>,
    right: &HashMap<&'static str, usize>,
) -> usize {
    let (small, large) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    small
        .iter()
        .map(|(kind, count)| (*count).min(large.get(kind).copied().unwrap_or(0)))
        .fold(0_usize, usize::saturating_add)
}

/// The upper bound as an overlap ratio against the larger endpoint —
/// directly comparable to `SHARED_SUBTREE_MIN_OVERLAP`
/// ([FUSION-SHARED-SUBTREE-BOUND]).
fn kind_bound_ratio(left: &EndpointView, right: &EndpointView) -> f64 {
    let larger = left.total.max(right.total);
    if larger == 0 {
        return 0.0;
    }
    lossless_count(kind_shared_upper_bound(left, right)) / lossless_count(larger)
}

/// Lossless small-count conversion for the coverage divisor.
fn lossless_count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}
