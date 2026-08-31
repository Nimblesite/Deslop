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
            // The overlap is admission evidence for the pair it was
            // measured on ([FUSED-SHARED-SUBTREE]), so the edge
            // carries it. This is what lets the same-file collapse
            // elect the *enclosing* method of a Type-3 near-miss over
            // the windows nested inside it: the same insertion costs
            // proportionally less over the wider context, so the
            // enclosing pair's overlap outranks every sub-window's —
            // and outranks their token estimates, which reward the
            // window precisely for excluding the difference.
            strength: pair
                .score
                .finite()
                .bounded_fused()
                .max(pair.shared_subtree_overlap),
        });
    }
    groups
        .into_iter()
        .map(|(root, members)| FusedCluster {
            members: members.into_iter().collect(),
            edges: edges.remove(&root).unwrap_or_default(),
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
