//! Large-tree coverage fallback for [FUSION-SHARED-SUBTREE].
//!
//! Endpoints past [`super::ALIGNMENT_MAX_NODES`] are too large for the
//! Zhang–Shasha alignment, so their shared mass is estimated instead —
//! and the estimate must never exceed what the alignment would have
//! measured, or the rescue admits pairs the honest measure rejects.
//!
//! Two properties are what make it safe, and the version this replaced
//! had neither.
//!
//! **The matching must respect order.** A tree alignment is ordered: in
//! any Tai mapping one node precedes another in post-order on the left
//! exactly when its partner does on the right. A bijection of identical
//! subtrees chosen without regard to order is not realisable as an
//! alignment — two subtrees matched in swapped order cannot both be
//! kept, one must be deleted and reinserted — so crediting both reports
//! mass no alignment reaches. The matching here walks both endpoints
//! left to right and never looks backwards, so the spans it pairs are
//! disjoint and in the same order on both sides.
//!
//! **Matched mass is not shared mass.** `structural` is
//! `max(nodes) − TED`, which charges for everything *un*matched on both
//! sides; matched node pairs alone ignore that charge entirely. A
//! matching of `m` node pairs costs at most `n₁ + n₂ − 2m` edits, so the
//! shared mass it guarantees is `2m − min(n₁, n₂)` — which is nothing at
//! all when the match is small relative to the endpoints, exactly as the
//! alignment would report.

use std::collections::HashMap;

use crate::fingerprint::Fingerprint;

use super::EndpointView;

/// One matched span's byte extent.
type Span = (usize, usize);

/// Shared node mass the alignment is guaranteed to reach on this pair —
/// a conservative lower bound on [`super::alignment::aligned_shared_nodes`],
/// in the same units, so the two are directly comparable.
///
/// Pinned by `the_large_tree_fallback_never_exceeds_the_alignment`,
/// `the_fallback_never_credits_a_nested_right_subtree_twice` and
/// `the_fallback_never_credits_mass_no_ordered_alignment_can_reach`.
pub(super) fn credit_shared_nodes(left: &EndpointView, right: &EndpointView) -> usize {
    let matched = matched_node_pairs(left, right);
    // A matching of `matched` node pairs leaves everything else on both
    // sides to be deleted or inserted, so it costs at most
    // `left.total + right.total - 2 * matched` edits and the shared mass
    // it guarantees is `max - cost`, i.e. `2 * matched - min`. Saturating
    // at zero: a match too small to pay for the unmatched remainder
    // guarantees no shared mass at all.
    matched
        .saturating_mul(2)
        .saturating_sub(left.total.min(right.total))
}

/// Node pairs an ordered alignment can certainly match: a greedy
/// left-to-right pairing of identical Merkle subtrees, outermost first
/// at each position.
///
/// Both cursors only ever advance, so every claimed span starts at or
/// after the previous one ended. That makes the spans disjoint on both
/// sides and identically ordered — the two conditions a Tai mapping
/// needs — and it is what stops a subtree nested inside an
/// already-claimed one from being counted a second time.
fn matched_node_pairs(left: &EndpointView, right: &EndpointView) -> usize {
    let occurrences = occurrences_by_hash(right);
    let mut left_cursor = 0_usize;
    let mut right_cursor = 0_usize;
    let mut matched = 0_usize;
    for entry in &left.entries {
        if entry.byte_range.start < left_cursor {
            continue;
        }
        let Some(claimed_end) = first_at_or_after(&occurrences, entry.hash, right_cursor) else {
            continue;
        };
        matched = matched.saturating_add(entry.node_count);
        left_cursor = entry.byte_range.end;
        right_cursor = claimed_end;
    }
    matched
}

/// The right endpoint's creditable spans grouped by Merkle hash. Each
/// group keeps the source order of [`EndpointView::entries`], which is
/// ascending by start, so the group can be searched by position.
fn occurrences_by_hash(right: &EndpointView) -> HashMap<[u8; 32], Vec<Span>> {
    let mut occurrences: HashMap<[u8; 32], Vec<Span>> = HashMap::new();
    for entry in &right.entries {
        occurrences
            .entry(entry.hash)
            .or_default()
            .push((entry.byte_range.start, entry.byte_range.end));
    }
    occurrences
}

/// The end of the earliest occurrence of `hash` starting at or after
/// `cursor`, or `None` when the right endpoint has none left.
///
/// Earliest rather than any: it leaves the most room for the matches
/// still to come, and taking a later one could only reduce the bound.
fn first_at_or_after(
    occurrences: &HashMap<[u8; 32], Vec<Span>>,
    hash: [u8; 32],
    cursor: usize,
) -> Option<usize> {
    let spans = occurrences.get(&hash)?;
    let position = spans.partition_point(|(start, _end)| *start < cursor);
    spans.get(position).map(|(_start, end)| *end)
}

/// Orders creditable spans for the greedy walk: ascending by start, and
/// at one start the widest first, so a container is offered before the
/// subtrees nested inside it.
pub(super) fn credit_order(left: &Fingerprint, right: &Fingerprint) -> std::cmp::Ordering {
    left.byte_range
        .start
        .cmp(&right.byte_range.start)
        .then(right.node_count.cmp(&left.node_count))
}
