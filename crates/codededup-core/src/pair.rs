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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::{
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::{estimate_jaccard, Signature},
};

/// Minimum fused score required before a pair enters a cluster. The
/// threshold is calibrated against the two deterministic signals only —
/// exact structural matches score 2.0 (1.0 structural + 1.0 Jaccard);
/// Type-3 candidates discovered by LSH alone need `token_jaccard` ≥
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
    /// Cosine similarity from the embedding pass, in `[0, 1]`. Populated
    /// in P5 once the `EmbeddingProvider` trait is wired; 0.0 today so
    /// the fused score is a sum of the two deterministic signals.
    pub embedding_cos: f64,
}

impl PairScore {
    /// Max-normalized sum of all three component scores. Each component is
    /// already in `[0, 1]`, so the fused value lives in `[0, 3]`. The
    /// ensemble-LLM 2025 paper is explicit that *averaging* hurts; sum /
    /// max help.
    #[must_use]
    pub fn fused(self) -> f64 {
        self.structural + self.token_jaccard + self.embedding_cos
    }
}

/// A candidate clone pair identified by fingerprint indices into the
/// flattened fingerprint list, plus its score and the node counts of
/// both endpoints (needed by [`is_surviving`] to reject low-information
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

/// Returns candidate pairs unioning:
///
/// - every distinct pair inside each structural (Merkle) hash bucket
///   (`structural = 1.0`),
/// - every LSH band collision (`structural = 0.0`),
/// - every ANN top-k neighbour surfaced by the embedding pass (pair
///   enters with its `embedding_cos` populated).
///
/// Pair scores include the token Jaccard estimate regardless of how the
/// pair was discovered. When `embedding_pairs` is empty (no provider
/// or `--embeddings=off`) the output matches the pre-P5 behaviour
/// exactly.
#[must_use]
pub fn candidate_pairs(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    lsh_pairs: &[(usize, usize)],
    embedding_pairs: &[EmbeddingPair],
) -> Vec<CandidatePair> {
    let mut scores: HashMap<(usize, usize), f64> = HashMap::new();
    let mut cosines: HashMap<(usize, usize), f64> = HashMap::new();
    collect_structural_pairs(fingerprints, &mut scores);
    add_lsh_pairs(lsh_pairs, &mut scores);
    add_embedding_pairs(embedding_pairs, &mut scores, &mut cosines);
    finalise_pairs(fingerprints, signatures, scores, &cosines)
}

/// Populates `scores` with `1.0` for every structural (Merkle-hash) pair.
///
/// Uses a **star topology** per bucket rather than a full N² enumeration:
/// the canonical member of the bucket is paired with every other member,
/// which is `O(n)` per bucket and still produces the same connected
/// component under transitive closure. For a bucket of `2_000` clones this
/// is `2_000` pairs instead of `2_000_000` — critical on large
/// generated-code corpora.
fn collect_structural_pairs(
    fingerprints: &[Fingerprint],
    scores: &mut HashMap<(usize, usize), f64>,
) {
    let mut by_hash: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        by_hash.entry(fingerprint.hash).or_default().push(index);
    }
    for bucket in by_hash.values() {
        let mut sorted = bucket.clone();
        sorted.sort_unstable();
        let Some(canonical) = sorted.first().copied() else {
            continue;
        };
        for other in sorted.iter().skip(1) {
            let key = order(canonical, *other);
            let _previous = scores.insert(key, 1.0_f64);
        }
    }
}

/// Adds LSH-only pairs. Pairs already present (from the structural pass)
/// keep their existing score — structural evidence dominates token
/// evidence when both fire.
fn add_lsh_pairs(lsh_pairs: &[(usize, usize)], scores: &mut HashMap<(usize, usize), f64>) {
    for &(a, b) in lsh_pairs {
        let key = order(a, b);
        let _previous = scores.entry(key).or_insert(0.0_f64);
    }
}

/// Adds embedding ANN pairs. Structural scoring is unchanged; pairs
/// gain an entry in `cosines` that [`finalise_pairs`] folds into the
/// final [`PairScore`]. Pairs already surfaced by the structural or
/// LSH passes still benefit from the embedding cosine being recorded.
fn add_embedding_pairs(
    embedding_pairs: &[EmbeddingPair],
    scores: &mut HashMap<(usize, usize), f64>,
    cosines: &mut HashMap<(usize, usize), f64>,
) {
    for pair in embedding_pairs {
        let key = order(pair.left, pair.right);
        let _previous_score = scores.entry(key).or_insert(0.0_f64);
        let slot = cosines.entry(key).or_insert(pair.cosine);
        if pair.cosine > *slot {
            *slot = pair.cosine;
        }
    }
}

/// Converts raw `(left, right) → structural_score` map into a sorted
/// [`CandidatePair`] list with token Jaccard filled in from the signatures
/// and the minimum endpoint node count attached for downstream filtering.
fn finalise_pairs(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    scores: HashMap<(usize, usize), f64>,
    cosines: &HashMap<(usize, usize), f64>,
) -> Vec<CandidatePair> {
    let mut pairs: Vec<CandidatePair> = scores
        .into_iter()
        .map(|((left, right), structural)| CandidatePair {
            left,
            right,
            min_node_count: min_node_count(fingerprints, left, right),
            score: PairScore {
                structural,
                token_jaccard: jaccard_for(signatures, left, right),
                embedding_cos: cosines.get(&(left, right)).copied().unwrap_or(0.0),
            },
        })
        .collect();
    pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
    pairs
}

/// Returns the smaller endpoint node count. Defaults to 0 when either
/// index is out of bounds — an impossible state in the current pipeline,
/// but keeps the helper total.
fn min_node_count(fingerprints: &[Fingerprint], left: usize, right: usize) -> usize {
    let l = fingerprints.get(left).map_or(0, |f| f.node_count);
    let r = fingerprints.get(right).map_or(0, |f| f.node_count);
    l.min(r)
}

/// Looks up both signatures and returns their estimated Jaccard. Returns
/// 0.0 when either signature is missing, which cannot happen in practice
/// because the pipeline always produces one signature per fingerprint.
fn jaccard_for(signatures: &[Signature], left: usize, right: usize) -> f64 {
    let Some(l) = signatures.get(left) else {
        return 0.0;
    };
    let Some(r) = signatures.get(right) else {
        return 0.0;
    };
    estimate_jaccard(l, r)
}

/// Puts the smaller index first. Pair keys are order-insensitive.
const fn order(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Applies the compound "survives clustering?" decision to a single pair.
/// Exact structural pairs (`structural == 1.0`) always survive when they
/// clear [`FUSED_THRESHOLD`]. LSH-only pairs additionally have to clear
/// [`LSH_ONLY_MIN_JACCARD`] so tiny sibling-window collisions do not
/// merge thousands of unrelated fingerprints via transitive closure.
fn is_surviving(pair: &CandidatePair) -> bool {
    if pair.score.fused() < FUSED_THRESHOLD {
        return false;
    }
    let lsh_only = pair.score.structural <= 0.0;
    if lsh_only && pair.score.token_jaccard < LSH_ONLY_MIN_JACCARD {
        return false;
    }
    if lsh_only && pair.min_node_count < LSH_ONLY_MIN_NODE_COUNT {
        return false;
    }
    true
}

/// Filters `pairs` by the fused threshold and returns the connected
/// components as [`FusedCluster`]s. Members inside each cluster are sorted
/// ascending so the final output is deterministic.
#[must_use]
pub fn cluster_by_transitive_closure(pairs: &[CandidatePair]) -> Vec<FusedCluster> {
    let surviving: Vec<&CandidatePair> = pairs.iter().filter(|pair| is_surviving(pair)).collect();
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

    /// Returns the per-signal mean. Zero when no pairs were folded in.
    fn mean(self) -> PairScore {
        if self.count == 0 {
            return PairScore {
                structural: 0.0,
                token_jaccard: 0.0,
                embedding_cos: 0.0,
            };
        }
        let divisor = f64::from(self.count);
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
