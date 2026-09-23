//! Shared-subtree structural overlap ([FUSED-SHARED-SUBTREE], gh #408).
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
//! hashes ([FUSED-SHARED-SUBTREE-MEMO]): hash equality pins the whole
//! normalised structure — the same premise as the `1.0` short-circuit —
//! so a corpus holding many byte-offset copies of one window costs one
//! alignment per *distinct structural pair*, not one per byte-range
//! combination. The rescue path additionally refuses the quadratic
//! alignment outright when a sound constant-per-node upper bound already
//! proves the pair cannot clear the admission floor
//! ([FUSED-SHARED-SUBTREE-BOUND]).

use std::{collections::HashMap, sync::Arc};

/// Zhang–Shasha ordered tree alignment ([FUSED-SHARED-SUBTREE]).
mod alignment;
/// Deterministic exact-alignment benchmark workloads.
#[cfg(feature = "benchmark")]
pub mod benchmark;
/// Sound count and kind upper bounds.
mod bounds;
/// The aligned core two endpoints share ([FUSED-SHARED-SUBTREE-CORE]).
mod core;
pub(crate) use core::judge_core;
/// Large-tree greedy coverage fallback ([FUSED-SHARED-SUBTREE]).
mod credit;
/// Rescue application over the candidate set ([FUSED-SHARED-SUBTREE]).
mod rescue;
pub(crate) use rescue::RescueContext;
/// Deterministic tree shapes shared by the measurement tests.
#[cfg(test)]
mod shapes;
/// Aggregate measurement counters.
mod stats;
/// Ordered-subsequence admission bound
/// ([FUSED-SHARED-SUBTREE-BOUND-ORDER]).
mod subsequence;
pub use stats::MeasureStats;
/// Rescue-pass gate counters ([PERF-FLUTTER-TODO-OBSERVABILITY]).
mod tally;
/// Endpoint view construction ([FUSED-SHARED-SUBTREE]).
mod view;

/// Measurement unit tests ([FUSED-SHARED-SUBTREE]).
#[cfg(test)]
mod tests;

use alignment::Aligner;
pub(crate) use bounds::endpoint_count_bound;
#[cfg(test)]
use bounds::kind_shared_upper_bound;
use bounds::{kind_bound_ratio, lossless_count};
pub use rescue::apply_shared_subtree_rescue;
use view::{build_view, EndpointView};

/// Most endpoint views one measurer retains
/// ([PERF-FLUTTER-TODO-MEMORY]). Star-shaped bucket members reuse one
/// endpoint across many pairs, which is what the memo buys; a corpus-scale
/// rescue population holds millions of *distinct* endpoints, and retaining
/// every view was a large share of the stage's memory. At the cap a new
/// generation replaces the old one, retaining locality without unbounded
/// residence.
const ENDPOINT_VIEW_MEMO_MAX: usize = 1_024;

/// Most exact-overlap results one measurer retains. The memo exists so a
/// structural pair appearing at many byte offsets costs one alignment;
/// a new generation replaces a full cache so later pairs can also be
/// reused with bounded residence.
const EXACT_RESULT_MEMO_MAX: usize = 16_384;

/// Most below-floor bounds one measurer retains, for the same reason as
/// [`EXACT_RESULT_MEMO_MAX`].
const BOUND_RESULT_MEMO_MAX: usize = 16_384;

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
/// [FUSED-SHARED-SUBTREE] exists to rescue — onto the conservative bound,
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

/// Measures shared-subtree overlap between fingerprint endpoints over
/// one corpus, memoising per-endpoint views and per-structural-pair
/// results so an endpoint appearing in many pairs is walked once and a
/// structure appearing at many byte offsets is aligned once
/// ([FUSED-SHARED-SUBTREE-MEMO]).
#[derive(Debug)]
pub struct OverlapMeasurer<'corpus> {
    /// `FileId → normalised root` for the corpus under measurement.
    tree_index: HashMap<FileId, &'corpus NormalizedNode>,
    /// Per-endpoint resolved state. `None` records an unresolvable
    /// range so it is not re-walked per pair.
    endpoints: HashMap<EndpointKey, Option<Arc<EndpointView>>>,
    /// Per-endpoint subtrees for the core pairing
    /// ([FUSED-SHARED-SUBTREE-CORE]), retained like `endpoints`.
    cores: HashMap<EndpointKey, Option<Arc<core::Resolved<'corpus>>>>,
    /// Exact measured overlap per structural pair.
    exact_results: HashMap<PairKey, f64>,
    /// Below-floor upper bounds per structural pair, usable only by the
    /// rescue's floor comparison ([FUSED-SHARED-SUBTREE-BOUND]) —
    /// never as an exact value.
    bound_results: HashMap<PairKey, f64>,
    /// Aggregate counters ([PIPELINE-OBSERVABILITY-STAGES]).
    stats: MeasureStats,
    /// Reusable alignment scratch ([PERF-FLUTTER-TODO-RESCUE]). Owned
    /// by the measurer so every alignment a worker runs reuses one pair
    /// of grids instead of allocating per keyroot pair.
    aligner: Aligner,
    /// Reusable ordered-bound row, for the same reason as `aligner`
    /// ([FUSED-SHARED-SUBTREE-BOUND-ORDER]).
    order_row: subsequence::Row,
}

/// Identity of one endpoint's resolved range, for the view memo.
type EndpointKey = (FileId, usize, usize, [u8; 32], usize);

/// Memo key for a measured pair: the ordered Merkle hashes of the two
/// endpoints ([FUSED-SHARED-SUBTREE-MEMO]). Hash equality pins the
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
            cores: HashMap::new(),
            exact_results: HashMap::new(),
            bound_results: HashMap::new(),
            stats: MeasureStats::default(),
            aligner: Aligner::default(),
            order_row: subsequence::Row::default(),
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
        // Views before the memo, always: the memo key is the hash pair,
        // but resolvability is a property of each byte range in its own
        // tree, so an unresolvable pair must answer `0.0` whether or
        // not a resolvable copy of the same structural pair was
        // measured first
        // (`an_unresolvable_copy_still_scores_its_unequal_pairs_zero`).
        let Some((left_view, right_view)) = self.view_pair(left, right) else {
            return 0.0;
        };
        let key = pair_key(left, right);
        if let Some(&cached) = self.exact_results.get(&key) {
            bump(&mut self.stats.exact_hits);
            return cached;
        }
        let result = self.measure_views(&left_view, &right_view);
        self.remember_exact(key, result);
        result
    }

    /// Overlap for the rescue's floor comparison
    /// ([FUSED-SHARED-SUBTREE-BOUND]). Identical to [`Self::overlap`]
    /// whenever the pair could clear `SHARED_SUBTREE_MIN_OVERLAP`; when
    /// a sound upper bound already proves it cannot, the bound itself —
    /// strictly below the floor — is returned without running the
    /// alignment. The admission decision is identical by construction;
    /// the value differs only on pairs the rescue then drops, where it
    /// is never rendered.
    pub fn rescue_overlap(&mut self, left: &Fingerprint, right: &Fingerprint) -> f64 {
        self.bounded_overlap(left, right, SHARED_SUBTREE_MIN_OVERLAP)
    }

    /// A sound upper bound below `floor`, and the exact overlap otherwise.
    /// Below the rescue floor an exact answer is required because cached
    /// rescue bounds are only reusable at or above that floor.
    pub fn bounded_overlap(&mut self, left: &Fingerprint, right: &Fingerprint, floor: f64) -> f64 {
        if floor < SHARED_SUBTREE_MIN_OVERLAP {
            return self.overlap(left, right);
        }
        if let Some(result) = self.trivial_bounded_overlap(left, right, floor) {
            return result;
        }
        // Bounds that need node kinds still resolve views before their
        // memo, so an unresolvable range cannot inherit a view result.
        let Some((left_view, right_view)) = self.view_pair(left, right) else {
            return 0.0;
        };
        let key = pair_key(left, right);
        if let Some(cached) = self.cached_bounded_overlap(key) {
            return cached;
        }
        self.measure_bounded_views(&left_view, &right_view, key, floor)
    }

    /// Merkle equality or a node-count bound can answer without a view.
    fn trivial_bounded_overlap(
        &mut self,
        left: &Fingerprint,
        right: &Fingerprint,
        floor: f64,
    ) -> Option<f64> {
        if left.hash == right.hash {
            bump(&mut self.stats.hash_equal);
            return Some(1.0);
        }
        let bound = endpoint_count_bound(left, right);
        if bound < floor {
            let key = pair_key(left, right);
            if let Some(&cached) = self.bound_results.get(&key) {
                bump(&mut self.stats.bound_hits);
                return Some(cached);
            }
            bump(&mut self.stats.bound_skips);
            return Some(self.remember_bound(key, bound));
        }
        None
    }

    /// Exact results and reusable below-rescue bounds share one lookup.
    fn cached_bounded_overlap(&mut self, key: PairKey) -> Option<f64> {
        if let Some(&cached) = self.exact_results.get(&key) {
            bump(&mut self.stats.exact_hits);
            return Some(cached);
        }
        if let Some(&bound) = self.bound_results.get(&key) {
            bump(&mut self.stats.bound_hits);
            return Some(bound);
        }
        None
    }

    /// Applies two sound bounds before paying for an exact alignment.
    fn measure_bounded_views(
        &mut self,
        left_view: &EndpointView,
        right_view: &EndpointView,
        key: PairKey,
        floor: f64,
    ) -> f64 {
        let bound = kind_bound_ratio(left_view, right_view);
        if bound < floor {
            bump(&mut self.stats.bound_skips);
            return self.remember_bound(key, bound);
        }
        self.measure_ordered_views(left_view, right_view, key, floor)
    }

    /// [FUSED-SHARED-SUBTREE-BOUND-ORDER] The ordered bound is tighter
    /// than the kind multiset and still bounds large-tree credit fallback.
    fn measure_ordered_views(
        &mut self,
        left_view: &EndpointView,
        right_view: &EndpointView,
        key: PairKey,
        floor: f64,
    ) -> f64 {
        let ordered = self.order_bound_ratio(left_view, right_view);
        if ordered < floor {
            bump(&mut self.stats.order_skips);
            return self.remember_bound(key, ordered);
        }
        let result = self.measure_views(left_view, right_view);
        self.remember_exact(key, result);
        result
    }

    /// Rotates a bounded exact-result memo so later structural pairs
    /// still reuse their alignments ([FUSED-SHARED-SUBTREE-MEMO]).
    fn remember_exact(&mut self, key: PairKey, result: f64) {
        if self.exact_results.len() == EXACT_RESULT_MEMO_MAX {
            self.exact_results.clear();
        }
        let _previous = self.exact_results.insert(key, result);
    }

    /// Records a below-floor bound under `key` (within the memo cap)
    /// and returns it. Every caller returns the value it just proved,
    /// so the memo write and the answer never drift apart.
    fn remember_bound(&mut self, key: PairKey, bound: f64) -> f64 {
        if bound < SHARED_SUBTREE_MIN_OVERLAP && self.bound_results.len() < BOUND_RESULT_MEMO_MAX {
            let _previous = self.bound_results.insert(key, bound);
        }
        bound
    }

    /// The ordered-subsequence upper bound as an overlap ratio against
    /// the larger endpoint ([FUSED-SHARED-SUBTREE-BOUND-ORDER]).
    fn order_bound_ratio(&mut self, left: &EndpointView, right: &EndpointView) -> f64 {
        let larger = left.total.max(right.total);
        if larger == 0 {
            return 0.0;
        }
        let shared = subsequence::common_subsequence_len(
            left.postorder(),
            left.total,
            &right.kind_positions,
            &mut self.order_row,
        );
        lossless_count(shared) / lossless_count(larger)
    }

    /// Whether this endpoint's byte range resolves to a measurable
    /// view, resolving (and memoising) it on first ask. Resolvability
    /// partitions equal-hash members for the grouped signal
    /// measurement ([FUSED-PAIR-SIGNALS]): an equal-hash pair is
    /// `1.0` by short-circuit either way, but a *different*-hash pair
    /// with an unresolvable side is `0.0`, so one representative may
    /// only stand for members that resolve the same way.
    pub fn resolvable(&mut self, endpoint: &Fingerprint) -> bool {
        self.view(endpoint).is_some()
    }

    /// Resolves both endpoint views, counting a pair with either side
    /// unresolvable. Unresolvable pairs are deliberately not memoised
    /// under the pair key: resolvability is a property of the byte
    /// range, not of the structure the key describes.
    fn view_pair(
        &mut self,
        left: &Fingerprint,
        right: &Fingerprint,
    ) -> Option<(Arc<EndpointView>, Arc<EndpointView>)> {
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
            self.aligner.shared_nodes(left, right)
        };
        (lossless_count(shared) / lossless_count(larger)).clamp(0.0, 1.0)
    }

    /// Returns (building on first use) the endpoint's resolved view.
    /// Retention is bounded by [`ENDPOINT_VIEW_MEMO_MAX`]; a full cache
    /// rotates before admitting a later endpoint.
    fn view(&mut self, endpoint: &Fingerprint) -> Option<Arc<EndpointView>> {
        let key = endpoint_key(endpoint);
        if let Some(cached) = self.endpoints.get(&key) {
            return cached.clone();
        }
        let built = build_view(&self.tree_index, endpoint).map(Arc::new);
        retain_endpoint(&mut self.endpoints, key, built)
    }
}

/// Keeps one generation of endpoint state bounded while allowing later
/// recovery families to reuse their own views ([FUSED-SHARED-SUBTREE-MEMO]).
fn retain_endpoint<T>(
    memo: &mut HashMap<EndpointKey, Option<Arc<T>>>,
    key: EndpointKey,
    built: Option<Arc<T>>,
) -> Option<Arc<T>> {
    if memo.len() == ENDPOINT_VIEW_MEMO_MAX {
        memo.clear();
    }
    let _previous = memo.insert(key, built.clone());
    built
}

/// The endpoint's full identity. A wrapper and its child can have the same
/// file and byte range but different structures and node counts.
fn endpoint_key(endpoint: &Fingerprint) -> EndpointKey {
    (
        endpoint.file_id,
        endpoint.byte_range.start,
        endpoint.byte_range.end,
        endpoint.hash,
        endpoint.node_count,
    )
}

/// Order-insensitive memo key for a measured pair
/// ([FUSED-SHARED-SUBTREE-MEMO]).
fn pair_key(left: &Fingerprint, right: &Fingerprint) -> PairKey {
    if left.hash <= right.hash {
        (left.hash, right.hash)
    } else {
        (right.hash, left.hash)
    }
}
