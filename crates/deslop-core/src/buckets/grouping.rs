//! [CLONE-KIND-FOLD] Keep established copies when a shape family contains unrelated members.

use std::collections::{BTreeMap, BTreeSet};

use super::ClusterKind;

/// Reference occurrence first, followed only by established matching members.
pub(crate) type KindGroup = (Vec<usize>, ClusterKind);

/// Forms clone groups and informational groups without treating missing edges as copies.
pub(crate) fn group_members(
    members: &[usize],
    mut compare: impl FnMut(usize, usize) -> Option<ClusterKind>,
) -> Vec<KindGroup> {
    let mut uncovered: BTreeSet<usize> = members.iter().copied().collect();
    let mut groups = BTreeMap::new();
    while let Some(reference) = uncovered.pop_first() {
        let (clones, information) = reference_groups(reference, members, &mut compare);
        if clones.0.len() > 1 {
            for member in &clones.0 {
                let _removed = uncovered.remove(member);
            }
            insert_group(&mut groups, clones);
        }
        if information.0.len() > 1 {
            insert_group(&mut groups, information);
        }
    }
    groups.into_values().collect()
}

/// Separates a reference's confirmed copies from its informational matches.
fn reference_groups(
    reference: usize,
    members: &[usize],
    compare: &mut impl FnMut(usize, usize) -> Option<ClusterKind>,
) -> (KindGroup, KindGroup) {
    let mut clones = (vec![reference], ClusterKind::Identical);
    let mut information = (vec![reference], ClusterKind::StructuralOnly);
    for &member in members.iter().filter(|member| **member != reference) {
        if let Some(kind) = compare(reference, member) {
            let group = if kind.is_clone() {
                &mut clones
            } else {
                &mut information
            };
            group.0.push(member);
            group.1 = group.1.weaker(kind);
        }
    }
    (clones, information)
}

/// Identical membership is one finding, regardless of its selected reference.
fn insert_group(groups: &mut BTreeMap<Vec<usize>, KindGroup>, group: KindGroup) {
    let mut key = group.0.clone();
    key.sort_unstable();
    if !group.1.is_clone() && !retain_new_information(groups, &group) {
        return;
    }
    let _entry = groups.entry(key).or_insert(group);
}

/// An informational subset adds no relation absent from its containing informational group.
fn retain_new_information(groups: &mut BTreeMap<Vec<usize>, KindGroup>, group: &KindGroup) -> bool {
    if groups
        .values()
        .any(|existing| !existing.1.is_clone() && covers_relations(existing, group))
    {
        return false;
    }
    groups.retain(|_, existing| existing.1.is_clone() || !covers_relations(group, existing));
    true
}

/// Every removed reference-to-member relation must already be represented.
fn covers_relations(container: &KindGroup, subset: &KindGroup) -> bool {
    let Some((reference, members)) = subset.0.split_first() else {
        return false;
    };
    let Some((cover_reference, cover_members)) = container.0.split_first() else {
        return false;
    };
    members.iter().all(|member| {
        (reference == cover_reference && cover_members.contains(member))
            || (member == cover_reference && cover_members.contains(reference))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXPECTED_MIXED_GROUPS: usize = 2;

    #[test]
    fn unrelated_reference_does_not_hide_the_copied_subset() {
        let groups = group_members(&[0, 1, 2], |left, right| {
            Some(if left == 0 || right == 0 {
                ClusterKind::StructuralOnly
            } else {
                ClusterKind::NearlyIdentical
            })
        });
        assert!(groups.contains(&(vec![1, 2], ClusterKind::NearlyIdentical)));
        assert!(groups.contains(&(vec![0, 1, 2], ClusterKind::StructuralOnly)));
        assert_eq!(
            groups.len(),
            EXPECTED_MIXED_GROUPS,
            "one complete informational group and the real copied subset"
        );
    }

    #[test]
    fn rejected_endpoint_cannot_become_a_similar_clone() {
        let groups = group_members(&[0, 1, 2], |left, right| {
            ((left == 0 && right == 1) || (left == 1 && right == 0))
                .then_some(ClusterKind::NearlyIdentical)
        });
        assert_eq!(groups, vec![(vec![0, 1], ClusterKind::NearlyIdentical)]);
    }
}
