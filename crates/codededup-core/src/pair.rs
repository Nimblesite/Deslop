//! Candidate pair scoring and transitive-closure clustering.
//!
//! Implements the non-embedding half of [FUSION-STRATEGY-MAX-SUM]:
//!
//! 1. Take the union of structural-hash bucket members and token-LSH band
//!    collisions as the candidate pair set.
//! 2. Score each pair on two signals in `[0, 1]`: `structural_sim` (1.0 for
//!    members of the same Merkle bucket, else the best-achievable subtree
//!    overlap which is 0.0 for cross-bucket token-only candidates) and
//!    `token_jaccard` estimated from the MinHash signatures.
//! 3. Apply the fused threshold and form clusters via transitive closure.
//!
//! Pair scores go into the final report (see [PRINCIPLES-AUDIENCE-AGENT])
//! so agent consumers can tell **why** each cluster was flagged.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::{
    fingerprint::Fingerprint,
    lsh::{estimate_jaccard, Signature},
};

/// Minimum fused score required before a pair enters a cluster. Fused score
/// is `structural_sim + token_jaccard` (max is 2.0); the 0.70 threshold
/// accepts any exact structural match and any Type-3 pair with Jaccard
/// ≥ 0.70.
pub const FUSED_THRESHOLD: f64 = 0.70;

/// Per-pair score breakdown in `[0, 1]`. See
/// [FUSION-STRATEGY-MAX-SUM] for the semantics.
#[derive(Debug, Clone, Copy)]
pub struct PairScore {
    /// 1.0 when the pair shares an exact Merkle bucket, else 0.0. The
    /// embedding pass in P5 will broaden this to `[0, 1]`.
    pub structural: f64,
    /// Estimated k-gram Jaccard in `[0, 1]`.
    pub token_jaccard: f64,
}

impl PairScore {
    /// Max-normalized sum. Each component is already normalised to `[0, 1]`
    /// so the sum lives in `[0, 2]`.
    #[must_use]
    pub fn fused(self) -> f64 {
        self.structural + self.token_jaccard
    }
}

/// A candidate clone pair identified by fingerprint indices into the
/// flattened fingerprint list, plus its score.
#[derive(Debug, Clone, Copy)]
pub struct CandidatePair {
    /// Lower fingerprint index.
    pub left: usize,
    /// Higher fingerprint index.
    pub right: usize,
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
/// - every LSH band collision (`structural = 0.0`).
///
/// Pair scores include the token Jaccard estimate regardless of how the
/// pair was discovered.
#[must_use]
pub fn candidate_pairs(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    lsh_pairs: &[(usize, usize)],
) -> Vec<CandidatePair> {
    let mut scores: HashMap<(usize, usize), f64> = HashMap::new();
    collect_structural_pairs(fingerprints, &mut scores);
    add_lsh_pairs(lsh_pairs, &mut scores);
    finalise_pairs(signatures, scores)
}

/// Populates `scores` with `1.0` for every pair sharing a Merkle hash.
fn collect_structural_pairs(
    fingerprints: &[Fingerprint],
    scores: &mut HashMap<(usize, usize), f64>,
) {
    let mut by_hash: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        by_hash.entry(fingerprint.hash).or_default().push(index);
    }
    for bucket in by_hash.values() {
        for (i_pos, left) in bucket.iter().enumerate() {
            for right in bucket.iter().skip(i_pos.saturating_add(1)) {
                let key = order(*left, *right);
                let _previous = scores.insert(key, 1.0_f64);
            }
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

/// Converts raw `(left, right) → structural_score` map into a sorted
/// [`CandidatePair`] list with token Jaccard filled in from the signatures.
fn finalise_pairs(
    signatures: &[Signature],
    scores: HashMap<(usize, usize), f64>,
) -> Vec<CandidatePair> {
    let mut pairs: Vec<CandidatePair> = scores
        .into_iter()
        .map(|((left, right), structural)| CandidatePair {
            left,
            right,
            score: PairScore {
                structural,
                token_jaccard: jaccard_for(signatures, left, right),
            },
        })
        .collect();
    pairs.sort_unstable_by(|a, b| (a.left, a.right).cmp(&(b.left, b.right)));
    pairs
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

/// Filters `pairs` by the fused threshold and returns the connected
/// components as [`FusedCluster`]s. Members inside each cluster are sorted
/// ascending so the final output is deterministic.
#[must_use]
pub fn cluster_by_transitive_closure(pairs: &[CandidatePair]) -> Vec<FusedCluster> {
    let surviving: Vec<&CandidatePair> = pairs
        .iter()
        .filter(|pair| pair.score.fused() >= FUSED_THRESHOLD)
        .collect();
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
    for (&member, _) in parents.clone().iter() {
        let root = find(parents, member);
        let _inserted = groups.entry(root).or_default().insert(member);
    }
    let mut totals: BTreeMap<usize, (f64, f64, u32)> = BTreeMap::new();
    for pair in surviving {
        let root = find(parents, pair.left);
        let entry = totals.entry(root).or_insert((0.0_f64, 0.0_f64, 0_u32));
        entry.0 += pair.score.structural;
        entry.1 += pair.score.token_jaccard;
        entry.2 = entry.2.saturating_add(1);
    }
    groups
        .into_iter()
        .map(|(root, members)| build_cluster(root, members, &totals))
        .collect()
}

/// Builds one [`FusedCluster`] from a connected-component membership set
/// and the precomputed pair-score totals.
fn build_cluster(
    root: usize,
    members: BTreeSet<usize>,
    totals: &BTreeMap<usize, (f64, f64, u32)>,
) -> FusedCluster {
    let ordered_members: Vec<usize> = members.into_iter().collect();
    let default_totals = (0.0_f64, 0.0_f64, 0_u32);
    let (structural_sum, jaccard_sum, count) = totals.get(&root).copied().unwrap_or(default_totals);
    let mean_score = if count == 0 {
        PairScore {
            structural: 0.0,
            token_jaccard: 0.0,
        }
    } else {
        let divisor = f64::from(count);
        PairScore {
            structural: structural_sum / divisor,
            token_jaccard: jaccard_sum / divisor,
        }
    };
    FusedCluster {
        members: ordered_members,
        mean_score,
    }
}

/// Ensures `id` has a parent entry (itself) in the union-find.
fn ensure_root(parents: &mut BTreeMap<usize, usize>, id: usize) -> usize {
    *parents.entry(id).or_insert(id)
}

/// Union-find find with path compression.
fn find(parents: &mut BTreeMap<usize, usize>, id: usize) -> usize {
    let parent = parents.get(&id).copied().unwrap_or(id);
    if parent == id {
        return id;
    }
    let root = find(parents, parent);
    let _previous = parents.insert(id, root);
    root
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
