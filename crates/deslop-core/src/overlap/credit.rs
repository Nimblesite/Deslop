//! Large-tree coverage fallback for [FUSION-SHARED-SUBTREE].
//!
//! Endpoints past [`super::ALIGNMENT_MAX_NODES`] are scored by greedy
//! maximal shared-Merkle-subtree coverage — a conservative lower bound
//! on the alignment, so it can suppress a rescue but never manufacture
//! one.

use std::collections::HashMap;

use super::EndpointView;

/// Greedy-maximal shared-Merkle-subtree node credit. Largest left
/// subtrees first, each credit consuming one concrete right-side
/// occurrence, nested-in-credited spans skipped on **both** endpoints.
/// A conservative lower bound on [`super::alignment::aligned_shared_nodes`]
/// — node mass matched under a bijection of disjoint identical subtrees
/// is achievable by an alignment. The bijection needs both sides
/// tracked: consuming bare hash counts on the right let a disjoint left
/// copy re-claim nodes nested inside an already-credited right subtree,
/// counting them twice and overshooting the alignment this bound stands
/// in for (`the_fallback_never_credits_a_nested_right_subtree_twice`).
///
/// Left entries arrive largest-first, so every candidate span is no
/// larger than the spans already credited on its side; a strict
/// container has strictly more nodes than its subtree, so a later
/// candidate can never contain a credited span and the nested-inside
/// test alone keeps each side's credited spans disjoint.
pub(super) fn credit_shared_nodes(left: &EndpointView, right: &EndpointView) -> usize {
    let mut open_right: HashMap<[u8; 32], Vec<(usize, usize)>> = HashMap::new();
    for entry in &right.entries {
        open_right
            .entry(entry.hash)
            .or_default()
            .push((entry.byte_range.start, entry.byte_range.end));
    }
    let mut left_taken: Vec<(usize, usize)> = Vec::new();
    let mut right_taken: Vec<(usize, usize)> = Vec::new();
    let mut credit = 0_usize;
    for entry in &left.entries {
        let span = (entry.byte_range.start, entry.byte_range.end);
        if nested_in_credited(span, &left_taken) {
            continue;
        }
        let Some(claimed) = claim_right_occurrence(entry.hash, &mut open_right, &right_taken)
        else {
            continue;
        };
        credit = credit.saturating_add(entry.node_count);
        left_taken.push(span);
        right_taken.push(claimed);
    }
    credit
}

/// True when `span` nests inside any already-credited span.
fn nested_in_credited(span: (usize, usize), taken: &[(usize, usize)]) -> bool {
    let (start, end) = span;
    taken
        .iter()
        .any(|(taken_start, taken_end)| *taken_start <= start && end <= *taken_end)
}

/// Consumes and returns one right-side occurrence of `hash` that is not
/// nested inside an already-credited right span. Identical hashes have
/// identical node counts, so any open occurrence is an equally-sized
/// witness and the first open one serves.
fn claim_right_occurrence(
    hash: [u8; 32],
    open_right: &mut HashMap<[u8; 32], Vec<(usize, usize)>>,
    right_taken: &[(usize, usize)],
) -> Option<(usize, usize)> {
    let candidates = open_right.get_mut(&hash)?;
    let position = candidates
        .iter()
        .position(|candidate| !nested_in_credited(*candidate, right_taken))?;
    Some(candidates.swap_remove(position))
}
