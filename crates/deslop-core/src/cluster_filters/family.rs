//! Partitioning a fused component into families, and restricting the
//! component to one of them.
//!
//! Two passes split a component before signals are measured —
//! [CLONE-NOISE-VERBATIM-SUBGROUP] by the source bytes a member covers,
//! [PIPELINE-CLUSTER-EXACT-SCOPE] by the normalised subtree it hashes to — and
//! both need the same two operations: group the members by a key in
//! first-appearance order, then rebuild the component around one group
//! keeping only the discovery edges whose endpoints stayed. The key is
//! the only difference, so it is the only thing either caller supplies.

use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::Hash,
};

use crate::pair::{FusedCluster, FusedEdge};

/// Groups `member_indices` by `key`, preserving first-appearance order
/// both between families and inside each one, so a split is
/// deterministic ([PIPELINE-DETERMINISM]).
///
/// A member whose key cannot be computed joins no family, which can only
/// ever make a pass do less.
///
/// Keys are located by hash, never by scanning the keys already seen: a
/// linear probe per member made the pass quadratic in members with a
/// full key comparison — a whole verbatim byte slice — per probe, and
/// one corpus-scale component paid tens of seconds for what one walk
/// answers ([PERF-FLUTTER-TODO-PAIRS]). First-appearance order is
/// carried by the slot index, so the grouping is byte-identical to the
/// scanning form.
pub(super) fn families_by<Key, Compute>(member_indices: &[usize], key: Compute) -> Vec<Vec<usize>>
where
    Key: Eq + Hash,
    Compute: Fn(usize) -> Option<Key>,
{
    let mut slots: HashMap<Key, usize> = HashMap::new();
    let mut families: Vec<Vec<usize>> = Vec::new();
    for index in member_indices.iter().copied() {
        let Some(computed) = key(index) else {
            continue;
        };
        match slots.entry(computed) {
            Entry::Occupied(slot) => {
                if let Some(family) = families.get_mut(*slot.get()) {
                    family.push(index);
                }
            }
            Entry::Vacant(slot) => {
                let _inserted = slot.insert(families.len());
                families.push(vec![index]);
            }
        }
    }
    families
}

/// The component restricted to `family`: its members, and only the
/// discovery edges whose both endpoints stayed. Membership is tested
/// against a set, not by scanning the family per edge endpoint — the
/// same corpus-scale component that made [`families_by`] quadratic
/// carries a proportional edge population.
pub(super) fn restrict(fused: &FusedCluster, family: &[usize]) -> FusedCluster {
    let kept: HashSet<usize> = family.iter().copied().collect();
    let edges: Vec<FusedEdge> = fused
        .edges
        .iter()
        .filter(|edge| kept.contains(&edge.left) && kept.contains(&edge.right))
        .copied()
        .collect();
    FusedCluster {
        members: family.to_vec(),
        edges,
    }
}
