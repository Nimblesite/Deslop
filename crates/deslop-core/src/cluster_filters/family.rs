//! Partitioning a fused component into families, and restricting the
//! component to one of them.
//!
//! Two passes split a component before signals are measured —
//! [CLONE-NOISE-VERBATIM-SUBGROUP] by the source bytes a member covers,
//! [PIPELINE-CLUSTER-ELECT] by the normalised subtree it hashes to — and
//! both need the same two operations: group the members by a key in
//! first-appearance order, then rebuild the component around one group
//! keeping only the discovery edges whose endpoints stayed. The key is
//! the only difference, so it is the only thing either caller supplies.

use crate::pair::{FusedCluster, FusedEdge};

/// Groups `member_indices` by `key`, preserving first-appearance order
/// both between families and inside each one, so a split is
/// deterministic ([PIPELINE-DETERMINISM]).
///
/// A member whose key cannot be computed joins no family, which can only
/// ever make a pass do less.
pub(super) fn families_by<Key, Compute>(member_indices: &[usize], key: Compute) -> Vec<Vec<usize>>
where
    Key: PartialEq,
    Compute: Fn(usize) -> Option<Key>,
{
    let mut order: Vec<Key> = Vec::new();
    let mut families: Vec<Vec<usize>> = Vec::new();
    for index in member_indices.iter().copied() {
        let Some(computed) = key(index) else {
            continue;
        };
        if let Some(slot) = order.iter().position(|seen| *seen == computed) {
            if let Some(family) = families.get_mut(slot) {
                family.push(index);
            }
        } else {
            order.push(computed);
            families.push(vec![index]);
        }
    }
    families
}

/// The component restricted to `family`: its members, and only the
/// discovery edges whose both endpoints stayed.
pub(super) fn restrict(fused: &FusedCluster, family: &[usize]) -> FusedCluster {
    let edges: Vec<FusedEdge> = fused
        .edges
        .iter()
        .filter(|edge| family.contains(&edge.left) && family.contains(&edge.right))
        .copied()
        .collect();
    FusedCluster {
        members: family.to_vec(),
        edges,
    }
}
