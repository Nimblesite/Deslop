//! Large-tree coverage fallback for [FUSION-SHARED-SUBTREE] —
//! **quarantined**, see below.
//!
//! Endpoints past [`super::ALIGNMENT_MAX_NODES`] were scored by greedy
//! maximal shared-Merkle-subtree coverage, on the stated premise that
//! it is "a conservative lower bound on the alignment, so it can
//! suppress a rescue but never manufacture one".
//!
//! **That premise is false, and the code is removed.**

use super::EndpointView;

/// Greedy-maximal shared-Merkle-subtree node credit — **removed as a
/// false-positive source**.
///
/// # What it did
///
/// It walked the left endpoint's creditable subtrees largest-first,
/// claimed for each one any unconsumed right-side occurrence of the
/// same Merkle hash, skipped spans nested inside already-credited ones
/// on both sides, and returned the total node mass so matched. That
/// total was then divided by the larger endpoint and returned from
/// `OverlapMeasurer::measure_views` as the pair's `structural` overlap.
///
/// # Why it is wrong
///
/// The bijection it builds is *unordered*. A tree alignment is not: a
/// Tai mapping preserves post-order on both sides, so two subtrees
/// matched in swapped order cannot both be kept — one must be deleted
/// and reinserted. Crediting both anyway reports shared mass that no
/// alignment can achieve, so the value overstates the overlap, and
/// [`crate::pair::SHARED_SUBTREE_MIN_OVERLAP`] then admits pairs the
/// honest measure rejects. Every such admission is a false positive,
/// and it lands on exactly the endpoints too large to check.
///
/// On the 51-minute Flutter run this route measured 4,080 pairs in the
/// rescue and a further 1,730 in the cluster-signal build — every one
/// of them an endpoint too large for the honest measure to check.
///
/// # Which test pins it
///
/// `overlap::tests::the_fallback_never_credits_mass_no_ordered_alignment_can_reach`
/// — two files holding the same two functions in swapped order. The
/// fallback credited 47 shared nodes where the alignment reaches 32.
/// The pre-existing `the_large_tree_fallback_never_exceeds_the_alignment`
/// asserts the same property, but only on a single same-order pair, so
/// it passed throughout.
///
/// # Panics
///
/// Always. This is the [`AGENTS.md`] accuracy quarantine: a measurement
/// that silently overstates overlap is worse than a crash, because a
/// crash is found in seconds and a false positive is never found at
/// all. Restoring large-endpoint measurement needs a bound that
/// respects post-order — see [FUSION-SHARED-SUBTREE-BOUND-ORDER],
/// which already computes one soundly in the other direction.
#[expect(
    clippy::panic,
    reason = "[FUSION-SHARED-SUBTREE] accuracy quarantine: this measurement \
              manufactured false positives and must not run until it is replaced"
)]
pub(super) fn credit_shared_nodes(left: &EndpointView, right: &EndpointView) -> usize {
    panic!(
        "credit_shared_nodes was removed: its greedy bijection ignores post-order \
         and credited shared mass no ordered alignment can reach, admitting pairs \
         the honest measure rejects (pinned by \
         the_fallback_never_credits_mass_no_ordered_alignment_can_reach). Refused \
         {left_entries} creditable left subtrees against {right_entries} right.",
        left_entries = left.entries.len(),
        right_entries = right.entries.len(),
    );
}
