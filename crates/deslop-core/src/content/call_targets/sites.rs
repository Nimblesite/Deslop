//! [FUSED-CONTENT-GATE-CALL-TARGET-PADDING] Surrounding syntax cannot erase a changed operation.

use super::{CallTarget, MemberContent};

/// Fixed-receiver selector changes across an equal, ordered sequence of call shapes.
pub(super) fn changes(left: &MemberContent, right: &MemberContent) -> Vec<(usize, usize)> {
    let left_calls = call_sites(left);
    let right_calls = call_sites(right);
    if !corresponding_shapes(&left_calls, &right_calls) {
        return Vec::new();
    }
    left_calls
        .into_iter()
        .zip(right_calls)
        .filter(|(first, second)| {
            unchanged_receiver(left, right, (first.1, second.1))
                && keys(left, right, (first.0, second.0)).is_some_and(|(left, right)| left != right)
        })
        .map(|(left, right)| (left.0, right.0))
        .collect()
}

/// Each external call must correspond in both authored order and structural shape.
fn corresponding_shapes(left: &[(usize, &CallTarget)], right: &[(usize, &CallTarget)]) -> bool {
    left.iter()
        .map(|(_, target)| target.shape())
        .eq(right.iter().map(|(_, target)| target.shape()))
}

/// External calls in authored order, with their own structural identities.
fn call_sites(member: &MemberContent) -> Vec<(usize, &CallTarget)> {
    member
        .external_calls
        .iter()
        .enumerate()
        .filter_map(|(index, target)| target.as_ref().map(|target| (index, target)))
        .collect()
}

/// No receiver handle or owned contract renamed across these corresponding calls.
fn unchanged_receiver(
    left: &MemberContent,
    right: &MemberContent,
    targets: (&CallTarget, &CallTarget),
) -> bool {
    targets.0.has_contract()
        && targets.1.has_contract()
        && targets.0.receiver_positions().len() == targets.1.receiver_positions().len()
        && targets
            .0
            .receiver_positions()
            .iter()
            .zip(targets.1.receiver_positions())
            .all(|(left_index, right_index)| {
                keys(left, right, (*left_index, *right_index))
                    .is_some_and(|(left, right)| left == right)
            })
}

/// The independently indexed authored keys at corresponding positions.
fn keys(
    left: &MemberContent,
    right: &MemberContent,
    indices: (usize, usize),
) -> Option<(u64, u64)> {
    Some((
        left.keys.get(indices.0)?.key,
        right.keys.get(indices.1)?.key,
    ))
}
