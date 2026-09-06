//! Candidate pair scoring and transitive-closure clustering.
//!
//! Implements the non-embedding half of [FUSED-STRATEGY-BOUNDED-MAX]:
//!
//! 1. Take the union of structural-hash bucket members and token-LSH band
//!    collisions as the candidate pair set.
//! 2. Score each pair on the evidence available before rescue: exact Merkle
//!    evidence `H`, token Jaccard `J`, and embedding cosine `E`.
//! 3. Apply the pair-specific fused threshold or the separate compound
//!    shared-subtree rescue, then form clusters via transitive closure.
//!
//! Pair scores go into the final report (see [PRINCIPLES-AUDIENCE-AGENT])
//! so agent consumers can tell **why** each cluster was flagged.

use crate::fingerprint::Fingerprint;

/// Candidate-pair construction helpers kept separate from closure clustering.
mod candidates;
pub use candidates::{candidate_pairs, candidate_pairs_for_language_policy, LshPairs};

/// Pair-content admission guard applied before transitive closure.
mod content_gate;
mod echo;
pub(crate) use content_gate::apply_pair_content_gate;
pub(crate) use echo::ExactClones;

/// Transitive-closure clustering over surviving pairs.
mod closure;
pub use closure::cluster_by_transitive_closure;

#[cfg(test)]
mod gate_parity_tests;

/// Minimum fused score required before a pair enters a cluster. The
/// threshold is calibrated against a unit-bounded fused confidence:
/// exact structural matches saturate at 1.0, and Type-3 candidates
/// discovered by LSH alone need `token_jaccard` ≥
/// [`LSH_ONLY_MIN_JACCARD`] *and* the fused threshold below, which
/// together keep LSH-only noise out of clusters. The `SourcererCC` `0.70`
/// token-bag intersection-over-larger-block operating point is directional
/// context only; this 0.85 bounded maximum is Deslop's corpus-derived bar.
pub const FUSED_THRESHOLD: f64 = 0.85;
/// Additional Jaccard floor applied to pairs that fired only on the LSH
/// path (no structural hash match). Keeps thousands of tiny "same
/// `using` / `namespace` structure" sibling windows from merging into
/// one mega-cluster via transitive closure.
pub const LSH_ONLY_MIN_JACCARD: f64 = 0.90;
/// Minimum node count required at **both endpoints** for an LSH-only pair
/// to survive clustering. Small subtrees have low information content —
/// an 18-node k-gram set is mostly grammar scaffolding (`using`,
/// `namespace`, `method_declaration`), so tens of thousands of such
/// subtrees reach Jaccard ≈ 1.0 purely by accident. Requiring a
/// substantive node count forces LSH-only matches to carry real signal.
pub const LSH_ONLY_MIN_NODE_COUNT: usize = 40;
/// Largest ratio between the two endpoint node counts of a pair carrying
/// no structural anchor ([PAIR-SIZE-COHERENCE]).
///
/// Structural evidence already constrains size: a shared Merkle bucket
/// means the trees match, so a structurally anchored pair is size coherent
/// by construction and this guard leaves it alone. A pair discovered by
/// embedding alone has no such constraint, and an embedding model will
/// happily score a parameter list and a ninety-term arithmetic chain in
/// the same file, over the same identifiers, at cosine 1.00. Admitting
/// that pair grows a cluster a member that duplicates nothing, and the
/// rendered summary then contradicts its own `canonical_node_count` —
/// "3 copies of a 19-node subtree" over an 865-byte, 274-node expression.
///
/// Four is deliberately loose. Type-3 and Type-4 clones do change size as
/// they drift, so the guard fires only where the pair is self
/// contradictory rather than merely uneven. Pinned by
/// `deslop::pair_size_coherence`.
pub const MAX_ENDPOINT_NODE_RATIO: usize = 4;
/// Jaccard floor for explicit cross-language audit candidates. This is
/// lower than the default LSH-only floor because cross-language AST
/// vocabularies differ, and the mode is opt-in for ports/generated
/// clients rather than normal same-language refactoring.
pub const CROSS_LANGUAGE_MIN_JACCARD: f64 = 0.10;
/// Shared-subtree overlap at or above which a below-threshold pair is
/// admitted as a Type-3 near-miss ([FUSED-SHARED-SUBTREE], gh #408).
///
/// A one-statement insertion rehashes every ancestor Merkle node, so
/// the enclosing method pair of a textbook Type-3 clone carries
/// `structural = 0.0` on the anchor axis while the unchanged statements
/// inside it stay Merkle-identical. Measured on the five `*-type3`
/// fixtures, the genuine near-miss pairs cover 0.81–0.88 of the larger
/// method with shared subtrees; the demanding floor plus the token
/// corroboration below keeps accidental shape-vocabulary overlap out.
pub const SHARED_SUBTREE_MIN_OVERLAP: f64 = 0.75;
/// Denominator of the largest endpoint share that may be absent while
/// still reaching [`SHARED_SUBTREE_MIN_OVERLAP`]. At the 0.75 floor,
/// the smaller endpoint must contain at least three quarters of the
/// larger endpoint's nodes.
const SHARED_SUBTREE_MAX_UNSHARED_DENOMINATOR: usize = 4;
/// Token-Jaccard corroboration a shared-subtree near-miss must also
/// carry ([FUSED-SHARED-SUBTREE]). Shared subtrees alone cannot admit
/// a pair: normalisation makes boilerplate scaffolding Merkle-identical
/// across unrelated files, so the overlap must be corroborated by the
/// independent token axis. "Moderate" by design — the exact whole-method
/// Jaccard of a one-statement Type-3 insertion measures 0.74–0.85
/// across the five fixture languages, below the 0.90 LSH-only floor
/// precisely because the near-miss statements dilute the k-gram set.
pub const SHARED_SUBTREE_MIN_JACCARD: f64 = 0.65;
/// Minimum smaller-endpoint node count for the shared-subtree route.
/// Lower than [`LSH_ONLY_MIN_NODE_COUNT`] because this route carries
/// structural corroboration LSH-only pairs lack; still high enough that
/// grammar scaffolding alone cannot reach it (the smallest genuine
/// fixture method, `python-type3`'s `aggregate`, is 31 nodes).
pub const SHARED_SUBTREE_MIN_NODE_COUNT: usize = 30;
/// Raw-content agreement a shared-subtree rescue pair must corroborate
/// ([FUSED-SHARED-SUBTREE]). Corroboration that the endpoints share some
/// raw content — not the routing support floor: the canonical renamed
/// near-miss (`csharp-type3`) measures 0.19 because the extra statement
/// destroys leaf alignment, while the gh #458 stranger measures 0.0436.
/// The floor sits between them; reuse of `CONTENT_SUPPORT_FLOOR` (0.70)
/// gated the anchor-free route and drove it to zero clusters.
pub const RESCUE_MIN_CONTENT_AGREEMENT: f64 = 0.10;
/// Cosine at or above which a measured `embedding_cos` counts as the
/// embedding pass *vouching for* a cluster rather than merely having
/// measured it ([FUSED-PAIR-SIGNALS]).
///
/// A cosine belongs to the pair, not to the pass that surfaced it
/// ([REPAIR-COSINE-MERGE], gh #351), so once embeddings are on every
/// rendered cluster carries one — including clusters the model considers
/// unrelated. The question a consumer must ask is therefore never *is
/// there a cosine* but *is the cosine positive evidence*, and this is
/// the only line that answers it: it is both the operating point at
/// which the ANN pass admits a pair as a candidate at all
/// ([TECH-EMBED-NEURAL]; the `candidates.embedding_min_cosine` lever in
/// `embedding/pairs.rs`) and the line at which
/// [CLONE-BUCKETS-ROUTING] row 2 lets semantic evidence carry a bucket
/// on its own.
///
/// Asking the other question instead is how a bucket came to follow the
/// discovery route. `report_render::route_shape_identical` tested the
/// [FUSED-CONTENT-GATE] escape against
/// `buckets::STRUCTURAL_ONLY_MAX_SUPPORT` — the ceiling *below* which a
/// signal counts as **absent** — so a cosine of 0.05 read as semantic
/// backing strong enough to overrule the measured content evidence. The
/// embeddings-off run has no cosine and is gated; the embeddings-on run
/// has a near-zero one and is not. `csharp-type3` rendered the identical
/// two occurrences as `structural_only` at cosine 0.00 and
/// `nearly_identical` at 0.61. Pinned by
/// `deslop::embedding_route_invariance` (gh #356).
pub const EMBEDDING_SUPPORT_FLOOR: f64 = 0.80;

/// Per-pair score breakdown in `[0, 1]`. Candidate admission stores exact
/// Merkle evidence in `structural`. Explicit pair comparison may instead
/// report measured overlap for the two requested endpoints. Only admission
/// calls [`Self::bounded_fused`] ([FUSED-STRATEGY-BOUNDED-MAX]).
#[derive(Debug, Clone, Copy)]
pub struct PairScore {
    /// Exact Merkle evidence `H` during admission; measured overlap `S`
    /// when comparing two explicitly requested endpoints.
    pub structural: f64,
    /// Estimated k-gram Jaccard in `[0, 1]`.
    pub token_jaccard: f64,
    /// Cosine similarity from the embedding pass, in `[0, 1]`.
    pub embedding_cos: f64,
}

impl PairScore {
    /// [FUSED-STRATEGY-BOUNDED-MAX] Bounded fusion over the three signal
    /// axes: the strongest single axis, never their sum.
    ///
    /// The axes are correlated views of one normalised tree, so combining
    /// them may sharpen a verdict but must never exceed the best evidence
    /// — a confidence above every individual axis would be manufactured,
    /// not measured. Non-finite axes contribute nothing. This is solely
    /// the pre-rescue pair-admission quantity; cluster reports never call
    /// it or render its result.
    #[must_use]
    pub fn bounded_fused(self) -> f64 {
        [self.structural, self.token_jaccard, self.embedding_cos]
            .into_iter()
            .filter(|axis| axis.is_finite())
            .fold(0.0_f64, f64::max)
            .clamp(0.0, 1.0)
    }

    /// Returns the score with every non-finite axis replaced by `0.0` —
    /// absent evidence, which is what a `NaN` axis actually is.
    ///
    /// Predicates written as comparisons are silently false against
    /// `NaN`, so a malformed axis reads as *positive* evidence to any
    /// test of the form `axis <= 0.0`. Normalising before a decision, not
    /// inside each predicate, keeps that from having to be re-remembered
    /// at every call site.
    #[must_use]
    pub fn finite(self) -> Self {
        Self {
            structural: finite_or_zero(self.structural),
            token_jaccard: finite_or_zero(self.token_jaccard),
            embedding_cos: finite_or_zero(self.embedding_cos),
        }
    }
}

/// Returns `value` when finite, `0.0` otherwise.
fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// A candidate clone pair identified by fingerprint indices into the
/// flattened fingerprint list, plus its score and the node counts of
/// both endpoints (needed by `survival_decision` to reject low-information
/// LSH-only matches).
#[derive(Debug, Clone, Copy)]
pub struct CandidatePair {
    /// Lower fingerprint index.
    pub left: usize,
    /// Higher fingerprint index.
    pub right: usize,
    /// Both endpoint node counts as `(smaller, larger)`. The honest
    /// measured sizes, used by the [PAIR-SIZE-COHERENCE] guard.
    pub endpoint_node_counts: (usize, usize),
    /// Node count compared against [`LSH_ONLY_MIN_NODE_COUNT`]. Normally
    /// the smaller endpoint, but explicit cross-language opt-in raises it
    /// to the floor so the information-content guard does not reject a
    /// port audit — which is why it cannot stand in for a measured size.
    pub lsh_only_node_floor: usize,
    /// Token-Jaccard floor for LSH-only candidates. Defaults to the
    /// conservative same-language floor; explicit cross-language opt-in
    /// lowers it to [`CROSS_LANGUAGE_MIN_JACCARD`] so port-audit
    /// comparisons remain available without weakening normal runs.
    pub lsh_only_min_jaccard: f64,
    /// Fused-score floor for this pair. Defaults to [`FUSED_THRESHOLD`];
    /// explicit cross-language audit pairs lower it to
    /// [`CROSS_LANGUAGE_MIN_JACCARD`] so lower-overlap ports can surface.
    pub fused_min_score: f64,
    /// Measured shared-subtree overlap ([FUSED-SHARED-SUBTREE]).
    /// `0.0` until the rescue pass measures it — it is only measured
    /// for pairs that are otherwise dropped below the fused threshold
    /// yet carry the corroborating token evidence, because walking two
    /// subtrees per pair across every candidate would repeat the
    /// admission-cost mistake [FUSED-CONTENT-GATE] deliberately avoids.
    pub shared_subtree_overlap: f64,
    /// Computed signal breakdown.
    pub score: PairScore,
}

/// A cluster discovered via transitive closure of surviving candidate pairs.
///
/// Carries membership plus the surviving discovery edges. Every edge is
/// an admitted pair: it survived either normal admission or the
/// [FUSED-SHARED-SUBTREE] rescue. Edges exist only to define transitive
/// closure; they are never promoted into cluster-level evidence.
#[derive(Debug, Clone)]
pub struct FusedCluster {
    /// Members of the cluster, sorted ascending by fingerprint index.
    pub members: Vec<usize>,
    /// The surviving candidate-pair edges inside this component.
    pub edges: Vec<FusedEdge>,
    /// Index of the shape family this component was admitted out of —
    /// the closure over every pre-gate candidate pair — so the report
    /// can ask the noise filters about the whole family when the
    /// component alone is only a fragment of it
    /// ([CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY]). `None` when no family
    /// table was built.
    pub shape_family: Option<usize>,
}

/// One surviving discovery edge inside a [`FusedCluster`]: the two
/// fingerprint indices it connects. Pair-scoped admission evidence
/// stays on the pair (`PairScore`); the cluster never selects, grades,
/// or ranks an edge by it ([PIPELINE-CLUSTER-CLOSURE]).
#[derive(Debug, Clone, Copy)]
pub struct FusedEdge {
    /// Lower fingerprint index of the surviving pair.
    pub left: usize,
    /// Higher fingerprint index of the surviving pair.
    pub right: usize,
}

/// Reason one candidate pair did or did not enter transitive closure.
enum PairSurvival {
    /// Pair entered the fused cluster graph.
    Survived,
    /// Pair was below the fused threshold but carried shared-subtree
    /// overlap and token corroboration ([FUSED-SHARED-SUBTREE]).
    SurvivedSharedSubtree,
    /// Pair failed the global fused-confidence threshold.
    DroppedBelowFused,
    /// LSH-only pair failed the token-Jaccard floor.
    DroppedLshOnlyJaccard,
    /// LSH-only pair failed the endpoint node-count floor.
    DroppedLshOnlyNodeCount,
    /// Pair had no structural anchor and endpoints too different in size
    /// to describe the same code ([PAIR-SIZE-COHERENCE]).
    DroppedSizeMismatch,
}

/// Counts GH#45 pair survival outcomes for structured observability.
#[derive(Default)]
struct SurvivalStats {
    /// Candidate pairs admitted to transitive closure.
    survived: usize,
    /// Pairs admitted via the shared-subtree route ([FUSED-SHARED-SUBTREE]).
    survived_shared_subtree: usize,
    /// Candidate pairs dropped below [`FUSED_THRESHOLD`].
    dropped_below_fused: usize,
    /// LSH-only pairs dropped below [`LSH_ONLY_MIN_JACCARD`].
    dropped_lsh_only_jaccard: usize,
    /// LSH-only pairs dropped below [`LSH_ONLY_MIN_NODE_COUNT`].
    dropped_lsh_only_node_count: usize,
    /// Pairs dropped past [`MAX_ENDPOINT_NODE_RATIO`].
    dropped_size_mismatch: usize,
}

impl SurvivalStats {
    /// Classifies every pair and returns the surviving subset.
    fn collect(pairs: &[CandidatePair]) -> (Self, Vec<&CandidatePair>) {
        let mut stats = Self::default();
        let mut surviving: Vec<&CandidatePair> = Vec::new();
        for pair in pairs {
            stats.push(pair, &mut surviving);
        }
        (stats, surviving)
    }

    /// Records one pair's outcome.
    fn push<'a>(&mut self, pair: &'a CandidatePair, surviving: &mut Vec<&'a CandidatePair>) {
        match survival_decision(pair) {
            PairSurvival::Survived => {
                self.survived = self.survived.saturating_add(1);
                surviving.push(pair);
            }
            PairSurvival::SurvivedSharedSubtree => {
                self.survived = self.survived.saturating_add(1);
                self.survived_shared_subtree = self.survived_shared_subtree.saturating_add(1);
                surviving.push(pair);
            }
            PairSurvival::DroppedBelowFused => {
                self.dropped_below_fused = self.dropped_below_fused.saturating_add(1);
            }
            PairSurvival::DroppedLshOnlyJaccard => {
                self.dropped_lsh_only_jaccard = self.dropped_lsh_only_jaccard.saturating_add(1);
            }
            PairSurvival::DroppedLshOnlyNodeCount => {
                self.dropped_lsh_only_node_count =
                    self.dropped_lsh_only_node_count.saturating_add(1);
            }
            PairSurvival::DroppedSizeMismatch => {
                self.dropped_size_mismatch = self.dropped_size_mismatch.saturating_add(1);
            }
        }
    }

    /// Emits the structured GH#45 pair-survival summary.
    fn log(self, total: usize) {
        tracing::info!(
            total,
            survived = self.survived,
            survived_shared_subtree = self.survived_shared_subtree,
            dropped_below_fused = self.dropped_below_fused,
            dropped_lsh_only_jaccard = self.dropped_lsh_only_jaccard,
            dropped_lsh_only_node_count = self.dropped_lsh_only_node_count,
            dropped_size_mismatch = self.dropped_size_mismatch,
            "pair survival outcome",
        );
    }
}

/// Applies the compound "survives clustering?" decision to a single pair.
fn survival_decision(pair: &CandidatePair) -> PairSurvival {
    let score = pair.score.finite();
    let rescued = shared_subtree_rescued(pair, score);
    if score.bounded_fused() < pair.fused_min_score && !rescued {
        return PairSurvival::DroppedBelowFused;
    }
    if score.structural <= 0.0 && !endpoints_are_size_coherent(pair.endpoint_node_counts) {
        return PairSurvival::DroppedSizeMismatch;
    }
    let lsh_only = score.structural <= 0.0 && score.embedding_cos <= 0.0 && !rescued;
    if lsh_only && score.token_jaccard < pair.lsh_only_min_jaccard {
        return PairSurvival::DroppedLshOnlyJaccard;
    }
    if lsh_only && pair.lsh_only_node_floor < LSH_ONLY_MIN_NODE_COUNT {
        return PairSurvival::DroppedLshOnlyNodeCount;
    }
    if rescued && score.bounded_fused() < pair.fused_min_score {
        return PairSurvival::SurvivedSharedSubtree;
    }
    PairSurvival::Survived
}

/// The insertion-time half of [`survival_decision`]
/// ([PERF-FLUTTER-TODO-PAIRS]): whether the pair survives when its
/// shared-subtree overlap is still unknown (`0.0`). Used by candidate
/// construction to refuse dead pairs before they are retained — the
/// arithmetic is the same function of the same axes, so a pair kept here
/// is exactly a pair the closure keeps, and a pair refused here is
/// exactly one the closure would drop (unless the rescue can still
/// admit it, which [`rescue_eligible`] covers separately).
pub(crate) fn construction_survives(pair: &CandidatePair) -> bool {
    let score = pair.score.finite();
    if score.bounded_fused() < pair.fused_min_score {
        return false;
    }
    if score.structural <= 0.0 && !endpoints_are_size_coherent(pair.endpoint_node_counts) {
        return false;
    }
    let lsh_only = score.structural <= 0.0 && score.embedding_cos <= 0.0;
    if lsh_only && score.token_jaccard < pair.lsh_only_min_jaccard {
        return false;
    }
    if lsh_only && pair.lsh_only_node_floor < LSH_ONLY_MIN_NODE_COUNT {
        return false;
    }
    true
}

/// True for a pair worth measuring: dropped below its fused floor on a
/// zero structural anchor, yet carrying the token corroboration and
/// endpoint substance the rescue route requires
/// ([FUSED-SHARED-SUBTREE]). Shared with the rescue pass so the
/// construction gate and the measurer can never disagree about which
/// pairs are rescue candidates.
pub(crate) fn rescue_eligible(pair: &CandidatePair) -> bool {
    let score = pair.score.finite();
    score.structural <= 0.0
        && score.bounded_fused() < pair.fused_min_score
        && score.token_jaccard >= SHARED_SUBTREE_MIN_JACCARD
        && pair.endpoint_node_counts.0 >= SHARED_SUBTREE_MIN_NODE_COUNT
        && shared_subtree_can_reach_floor(pair.endpoint_node_counts)
}

/// True for a pair the token axis carries on its own: no structural
/// anchor, no embedding support, Jaccard at the LSH-only floor, and the
/// fused floor cleared on that echo alone. [FUSED-CONTENT-GATE] holds such
/// a pair to the promote floor only when it is *unanchored* — when its
/// shared-subtree alignment has been measured and failed
/// [`SHARED_SUBTREE_MIN_OVERLAP`]. An unmeasured overlap reads as `0.0`,
/// which is no alignment at all, so without a measurement the gate cannot
/// tell a near-identical run of functions (`go-cluster-extent-alignment`:
/// two writers renamed, one literal swapped for a constant) from the
/// whole-file-against-interior-window echo the promote floor exists to
/// refuse (#339). Sizes that cannot reach the overlap floor are left
/// unmeasured: the gate's verdict would be the same.
pub(crate) fn token_carried(pair: &CandidatePair) -> bool {
    let score = pair.score.finite();
    score.structural <= 0.0
        && score.embedding_cos < EMBEDDING_SUPPORT_FLOOR
        && score.token_jaccard >= LSH_ONLY_MIN_JACCARD
        && score.bounded_fused() >= pair.fused_min_score
        && pair.endpoint_node_counts.0 >= SHARED_SUBTREE_MIN_NODE_COUNT
        && shared_subtree_can_reach_floor(pair.endpoint_node_counts)
}

/// True for a pair whose shared-subtree alignment the rescue pass
/// measures: a rescue candidate ([`rescue_eligible`]), or a token-carried
/// pair whose content floor turns on that measurement
/// ([`token_carried`]).
pub(crate) fn alignment_required(pair: &CandidatePair) -> bool {
    rescue_eligible(pair) || token_carried(pair)
}

/// Whether endpoint sizes leave enough nodes for the smaller tree to
/// cover [`SHARED_SUBTREE_MIN_OVERLAP`] of the larger tree. The
/// complement form avoids overflow for corpus-sized node counts.
fn shared_subtree_can_reach_floor((smaller, larger): (usize, usize)) -> bool {
    let maximum_unshared = larger / SHARED_SUBTREE_MAX_UNSHARED_DENOMINATOR;
    smaller >= larger.saturating_sub(maximum_unshared)
}

/// True when the pair's endpoints live in different files.
///
/// The rescue measures every eligible cross-file pair; inside one file
/// it measures only the narrow population
/// [FUSED-SHARED-SUBTREE-SAME-FILE] describes, so this predicate is the
/// scope split rather than the scope itself
/// (`RescueContext::measures`). Admitting same-file pairs on shape
/// overlap alone is the `#197` in-file sibling-family shape, which the
/// report already spends a dedicated proof suppressing, and it is also
/// how a single-file corpus loses its findings: unconstrained same-file
/// rescues union that file's subtrees into one transitive component,
/// and the same-file overlap collapse then reduces it to a single
/// logical location, which is dropped below `MIN_REPORTABLE_MEMBERS`
/// (`issue_119_role_gate_exercised`).
pub(crate) fn crosses_files(left: &Fingerprint, right: &Fingerprint) -> bool {
    left.file_id != right.file_id
}

/// [FUSED-SHARED-SUBTREE] admission: a pair below the fused threshold
/// still enters clustering when its measured shared-subtree overlap
/// clears [`SHARED_SUBTREE_MIN_OVERLAP`], the independent token axis
/// corroborates at [`SHARED_SUBTREE_MIN_JACCARD`], and both endpoints
/// are substantive ([`SHARED_SUBTREE_MIN_NODE_COUNT`]). This is a
/// compound gate over two *independently measured* axes, not sum
/// fusion — neither axis alone admits, and rescue never mutates the
/// pre-rescue fused value ([FUSED-STRATEGY-BOUNDED-MAX]).
fn shared_subtree_rescued(pair: &CandidatePair, score: PairScore) -> bool {
    pair.shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP
        && score.token_jaccard >= SHARED_SUBTREE_MIN_JACCARD
        && pair.endpoint_node_counts.0 >= SHARED_SUBTREE_MIN_NODE_COUNT
}

/// True when two endpoints are close enough in size to describe the same
/// code ([PAIR-SIZE-COHERENCE]). See [`MAX_ENDPOINT_NODE_RATIO`].
fn endpoints_are_size_coherent((smaller, larger): (usize, usize)) -> bool {
    larger <= smaller.saturating_mul(MAX_ENDPOINT_NODE_RATIO)
}
