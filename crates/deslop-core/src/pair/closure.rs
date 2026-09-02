//! Transitive-closure clustering over surviving candidate pairs
//! ([FUSED-STRATEGY-BOUNDED-MAX]). Split from the parent module, which
//! owns the pair types, thresholds, and the survival decision.

use std::collections::{BTreeMap, BTreeSet};

use super::{CandidatePair, FusedCluster, FusedEdge, SurvivalStats};

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

/// Groups members by union-find root, attaching each surviving edge to
/// the component it glued together.
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
    let mut edges: BTreeMap<usize, Vec<FusedEdge>> = BTreeMap::new();
    for pair in surviving {
        let root = find(parents, pair.left);
        edges.entry(root).or_default().push(FusedEdge {
            left: pair.left,
            right: pair.right,
        });
    }
    groups
        .into_iter()
        .map(|(root, members)| FusedCluster {
            members: members.into_iter().collect(),
            edges: edges.remove(&root).unwrap_or_default(),
            convicted: false,
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

#[cfg(test)]
mod tests {
    use super::{cluster_by_transitive_closure, CandidatePair};
    use crate::pair::PairScore;

    const NODE_COUNT: usize = 40;
    const ADMITTED_SCORE: f64 = 1.0;
    const FUSED_FLOOR: f64 = 0.85;

    // [FUSED-STRATEGY-BOUNDED-MAX] Admitted pair edges, not structural
    // families or post-closure repair, define the connected component.
    #[test]
    fn closure_preserves_every_member_reachable_by_admitted_edges() {
        let clusters = cluster_by_transitive_closure(&[
            admitted_pair(0, 1),
            admitted_pair(1, 2),
            admitted_pair(2, 3),
        ]);
        assert_eq!(clusters.len(), 1, "the admitted chain forms one component");
        let Some(cluster) = clusters.first() else {
            return;
        };
        assert_eq!(cluster.members, vec![0, 1, 2, 3]);
        assert_eq!(cluster.edges.len(), 3, "every admitted pair edge remains");
    }

    fn admitted_pair(left: usize, right: usize) -> CandidatePair {
        CandidatePair {
            left,
            right,
            endpoint_node_counts: (NODE_COUNT, NODE_COUNT),
            lsh_only_node_floor: NODE_COUNT,
            lsh_only_min_jaccard: 0.0,
            fused_min_score: FUSED_FLOOR,
            shared_subtree_overlap: 0.0,
            score: PairScore {
                structural: ADMITTED_SCORE,
                token_jaccard: 0.0,
                embedding_cos: 0.0,
            },
        }
    }
}
