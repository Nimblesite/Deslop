//! Candidate pair scoring and transitive-closure clustering.
//!
//! Implements the non-embedding half of [FUSION-STRATEGY-BOUNDED-MAX]:
//!
//! 1. Take the union of structural-hash bucket members and token-LSH band
//!    collisions as the candidate pair set.
//! 2. Score each pair on two signals in `[0, 1]`: `structural_sim` (1.0 for
//!    members of the same Merkle bucket, else the best-achievable subtree
//!    overlap which is 0.0 for cross-bucket token-only candidates) and
//!    `token_jaccard` estimated from the `MinHash` signatures.
//! 3. Apply the fused threshold and form clusters via transitive closure.
//!
//! Pair scores go into the final report (see [PRINCIPLES-AUDIENCE-AGENT])
//! so agent consumers can tell **why** each cluster was flagged.

use std::collections::{BTreeMap, BTreeSet};

/// Candidate-pair construction helpers kept separate from closure clustering.
mod candidates;
pub use candidates::{candidate_pairs, candidate_pairs_for_language_policy};

/// Minimum fused score required before a pair enters a cluster. The
/// threshold is calibrated against a unit-bounded fused confidence:
/// exact structural matches saturate at 1.0, and Type-3 candidates
/// discovered by LSH alone need `token_jaccard` ≥
/// [`LSH_ONLY_MIN_JACCARD`] *and* the fused threshold below, which
/// together keep LSH-only noise out of clusters. The literature
/// ([TECH-TOKEN-SOURCERERCC]) treats Jaccard ≥ 0.7 as a typical Type-3
/// cutoff; we go higher for LSH-only because those pairs have no
/// structural anchor.
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

/// Per-pair score breakdown in `[0, 1]`. See
/// [FUSION-STRATEGY-BOUNDED-MAX] for the semantics. Three slots are reserved
/// from v1 so the embedding pass in P5 is additive, not a schema bump:
/// the ensemble-LLM 2025 finding is that sum/max fusion (never average)
/// gives the biggest gain.
#[derive(Debug, Clone, Copy)]
pub struct PairScore {
    /// 1.0 when the pair shares an exact Merkle bucket, else 0.0.
    pub structural: f64,
    /// Estimated k-gram Jaccard in `[0, 1]`.
    pub token_jaccard: f64,
    /// Cosine similarity from the embedding pass, in `[0, 1]`.
    pub embedding_cos: f64,
}

impl PairScore {
    /// [FUSION-STRATEGY-BOUNDED-MAX] Bounded fusion over the three signal
    /// axes: the strongest single axis, never their sum.
    ///
    /// The axes are correlated views of one normalised tree, so combining
    /// them may sharpen a verdict but must never exceed the best evidence
    /// — a confidence above every individual axis would be manufactured,
    /// not measured. Non-finite axes contribute nothing. At render time
    /// [FUSION-CONTENT-GATE] (`buckets.rs`) re-scores shape-saturating
    /// clusters as `max(embedding_cos, shape × content)` — the same bound
    /// with measured content evidence in place of this function's
    /// implicit 1.0 — making the gate the definition of the rendered
    /// confidence rather than a correction of it.
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
    /// Computed signal breakdown.
    pub score: PairScore,
}

/// A cluster discovered via transitive closure of surviving candidate pairs.
///
/// Carries membership only. Signals are **not** aggregated here: the
/// pairs that glued a component together are discovery evidence, and
/// averaging them once diluted byte-proven pairs with every weaker edge
/// in the component. The rendered signal breakdown is measured between
/// the rendered occurrences in `crate::cluster` instead.
#[derive(Debug, Clone)]
pub struct FusedCluster {
    /// Members of the cluster, sorted ascending by fingerprint index.
    pub members: Vec<usize>,
}

/// Reason one candidate pair did or did not enter transitive closure.
enum PairSurvival {
    /// Pair entered the fused cluster graph.
    Survived,
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
    if score.bounded_fused() < pair.fused_min_score {
        return PairSurvival::DroppedBelowFused;
    }
    if score.structural <= 0.0 && !endpoints_are_size_coherent(pair.endpoint_node_counts) {
        return PairSurvival::DroppedSizeMismatch;
    }
    let lsh_only = score.structural <= 0.0 && score.embedding_cos <= 0.0;
    if lsh_only && score.token_jaccard < pair.lsh_only_min_jaccard {
        return PairSurvival::DroppedLshOnlyJaccard;
    }
    if lsh_only && pair.lsh_only_node_floor < LSH_ONLY_MIN_NODE_COUNT {
        return PairSurvival::DroppedLshOnlyNodeCount;
    }
    PairSurvival::Survived
}

/// True when two endpoints are close enough in size to describe the same
/// code ([PAIR-SIZE-COHERENCE]). See [`MAX_ENDPOINT_NODE_RATIO`].
fn endpoints_are_size_coherent((smaller, larger): (usize, usize)) -> bool {
    larger <= smaller.saturating_mul(MAX_ENDPOINT_NODE_RATIO)
}

/// Filters `pairs` by the fused threshold and returns the connected
/// components as [`FusedCluster`]s. Members inside each cluster are sorted
/// ascending so the final output is deterministic.
#[must_use]
pub fn cluster_by_transitive_closure(pairs: &[CandidatePair]) -> Vec<FusedCluster> {
    let (stats, surviving) = SurvivalStats::collect(pairs);
    stats.log(pairs.len());
    if surviving.is_empty() {
        return Vec::new();
    }
    let mut parents: BTreeMap<usize, usize> = BTreeMap::new();
    for pair in &surviving {
        let _left = ensure_root(&mut parents, pair.left);
        let _right = ensure_root(&mut parents, pair.right);
        union(&mut parents, pair.left, pair.right);
    }
    build_clusters(&mut parents)
}

/// Groups members by union-find root into membership-only clusters.
fn build_clusters(parents: &mut BTreeMap<usize, usize>) -> Vec<FusedCluster> {
    let mut groups: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let members: Vec<usize> = parents.keys().copied().collect();
    for member in members {
        let root = find(parents, member);
        let _inserted = groups.entry(root).or_default().insert(member);
    }
    groups
        .into_values()
        .map(|members| FusedCluster {
            members: members.into_iter().collect(),
        })
        .collect()
}

/// Ensures `id` has a parent entry (itself) in the union-find.
fn ensure_root(parents: &mut BTreeMap<usize, usize>, id: usize) -> usize {
    *parents.entry(id).or_insert(id)
}

/// Iterative union-find with path compression. Iterative so the
/// recursion depth cannot overflow the stack on corpora with long
/// equivalence chains (≥17K fingerprints observed on real C# repos).
fn find(parents: &mut BTreeMap<usize, usize>, id: usize) -> usize {
    let mut current = id;
    let mut path: Vec<usize> = Vec::new();
    loop {
        let parent = parents.get(&current).copied().unwrap_or(current);
        if parent == current {
            break;
        }
        path.push(current);
        current = parent;
    }
    for node in path {
        let _previous = parents.insert(node, current);
    }
    current
}

/// Union-find union.
fn union(parents: &mut BTreeMap<usize, usize>, a: usize, b: usize) {
    let root_a = find(parents, a);
    let root_b = find(parents, b);
    if root_a == root_b {
        return;
    }
    let _previous = parents.insert(root_a, root_b);
}
