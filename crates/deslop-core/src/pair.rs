//! Candidate pair scoring and transitive-closure clustering.
//!
//! Implements the non-embedding half of [FUSION-STRATEGY-MAX-SUM]:
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
/// Jaccard floor for explicit cross-language audit candidates. This is
/// lower than the default LSH-only floor because cross-language AST
/// vocabularies differ, and the mode is opt-in for ports/generated
/// clients rather than normal same-language refactoring.
pub const CROSS_LANGUAGE_MIN_JACCARD: f64 = 0.10;

/// Per-pair score breakdown in `[0, 1]`. See
/// [FUSION-STRATEGY-MAX-SUM] for the semantics. Three slots are reserved
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
    /// # QUARANTINED — #343, `[FUSION-STRATEGY-MAX-SUM]`
    ///
    /// RESTORING THIS OR CALLING THIS FUNCTION IS ILLEGAL.
    /// NO CODE IS ALLOWED TO CALL THIS AND THIS MUST ALWAYS
    /// PANIC AND NOTHING ELSE. POINT CALL SITES ELSEWHERE.
    ///
    /// The deleted body summed the three axes and clamped:
    /// `(structural + token_jaccard + embedding_cos).clamp(0.0, 1.0)`.
    /// The axes are three correlated views of one normalised tree, so the
    /// sum manufactures confidence none of them carries alone: a cluster
    /// at `structural 0.00, token 0.30, embedding 0.94` clamps to a flat
    /// `1.000` — rendered indistinguishable from a byte-proven verbatim
    /// copy — and slips past [FUSION-CONTENT-GATE], whose corners engage
    /// only at `structural >= 0.99` / `token >= 0.95`. The same clamp
    /// inflated pair admission: axes that are individually weak summed
    /// past `FUSED_THRESHOLD` and dragged unproven pairs into transitive
    /// closure. Use [`PairScore::bounded_fused`].
    /// Pinned by `issue_343_sum_clamp_saturation.rs`.
    ///
    /// # Panics
    ///
    /// Always. The quarantine mandates the panic: any caller reaching this
    /// function is reproducing the #343 false positive.
    #[allow(
        dead_code,
        clippy::panic,
        reason = "[FUSION-STRATEGY-MAX-SUM] #343 accuracy quarantine. CLAUDE.md mandates \
                  replacing code that causes false positives with a panic, which the \
                  workspace `panic = \"deny\"` and `-D dead-code` gates would otherwise \
                  reject. The no-suppressions rule yields to the quarantine rule here by \
                  explicit instruction; this allow is legal only on quarantined code."
    )]
    #[must_use]
    pub fn fused(self) -> f64 {
        panic!(
            "QUARANTINED #343: PairScore::fused summed three correlated signals and \
             clamped, saturating mid-band clusters to a confidence of 1.0 that no \
             single axis earned and no byte-identical pair backs. \
             Use PairScore::bounded_fused. \
             Pinned by issue_343_sum_clamp_saturation.rs. \
             structural={} token_jaccard={} embedding_cos={}",
            self.structural, self.token_jaccard, self.embedding_cos
        )
    }

    /// [FUSION-STRATEGY-MAX-SUM] Bounded fusion over the three signal
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
    /// Node count of the smaller endpoint — used as the LSH-only
    /// information-content floor.
    pub min_node_count: usize,
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
#[derive(Debug, Clone)]
pub struct FusedCluster {
    /// Members of the cluster, sorted ascending by fingerprint index.
    pub members: Vec<usize>,
    /// Mean pair score across the pairs that entered this cluster.
    pub mean_score: PairScore,
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
            "pair survival outcome",
        );
    }
}

/// Applies the compound "survives clustering?" decision to a single pair.
fn survival_decision(pair: &CandidatePair) -> PairSurvival {
    if pair.score.bounded_fused() < pair.fused_min_score {
        return PairSurvival::DroppedBelowFused;
    }
    let lsh_only = pair.score.structural <= 0.0 && pair.score.embedding_cos <= 0.0;
    if lsh_only && pair.score.token_jaccard < pair.lsh_only_min_jaccard {
        return PairSurvival::DroppedLshOnlyJaccard;
    }
    if lsh_only && pair.min_node_count < LSH_ONLY_MIN_NODE_COUNT {
        return PairSurvival::DroppedLshOnlyNodeCount;
    }
    PairSurvival::Survived
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
    build_clusters(&mut parents, &surviving)
}

/// Groups members by root and attaches aggregate scores.
fn build_clusters(
    parents: &mut BTreeMap<usize, usize>,
    surviving: &[&CandidatePair],
) -> Vec<FusedCluster> {
    let mut groups: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let members: Vec<usize> = parents.keys().copied().collect();
    for member in members {
        let root = find(parents, member);
        let _inserted = groups.entry(root).or_default().insert(member);
    }
    let mut totals: BTreeMap<usize, ClusterTotals> = BTreeMap::new();
    for pair in surviving {
        let root = find(parents, pair.left);
        let entry = totals.entry(root).or_default();
        entry.add(pair.score);
    }
    groups
        .into_iter()
        .map(|(root, members)| build_cluster(root, members, &totals))
        .collect()
}

/// Running totals per cluster root. Kept in one struct so the
/// per-cluster mean score stays symmetric across all three signals.
#[derive(Debug, Default, Clone, Copy)]
struct ClusterTotals {
    /// Sum of structural scores across pairs in the cluster.
    structural: f64,
    /// Sum of token-Jaccard scores.
    token_jaccard: f64,
    /// Sum of embedding cosines.
    embedding_cos: f64,
    /// Number of pairs folded into the totals.
    count: u32,
}

impl ClusterTotals {
    /// Folds a single pair's score into the running totals.
    fn add(&mut self, score: PairScore) {
        self.structural += score.structural;
        self.token_jaccard += score.token_jaccard;
        self.embedding_cos += score.embedding_cos;
        self.count = self.count.saturating_add(1);
    }

    /// Returns the per-signal mean. When no pairs were folded in the
    /// numerators are already zero, so dividing by one preserves the
    /// zero score without a separate branch.
    fn mean(self) -> PairScore {
        let divisor = f64::from(self.count.max(1));
        PairScore {
            structural: self.structural / divisor,
            token_jaccard: self.token_jaccard / divisor,
            embedding_cos: self.embedding_cos / divisor,
        }
    }
}

/// Builds one [`FusedCluster`] from a connected-component membership set
/// and the precomputed pair-score totals.
fn build_cluster(
    root: usize,
    members: BTreeSet<usize>,
    totals: &BTreeMap<usize, ClusterTotals>,
) -> FusedCluster {
    let ordered_members: Vec<usize> = members.into_iter().collect();
    let mean_score = totals.get(&root).copied().unwrap_or_default().mean();
    FusedCluster {
        members: ordered_members,
        mean_score,
    }
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
