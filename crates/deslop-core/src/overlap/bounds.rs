//! Sound upper bounds for shared-node overlap ([FUSED-SHARED-SUBTREE-BOUND]).

use std::collections::HashMap;

use super::EndpointView;
use crate::fingerprint::Fingerprint;

/// Sound upper bound on the alignment's shared-node count
/// ([FUSED-SHARED-SUBTREE-BOUND]). Any edit script maps some set `M`
/// of node pairs; its cost is `deletes + inserts + relabels =
/// larger + smaller − 2|M| + relabels`, so the shared mass
/// `larger − TED` never exceeds the kind-preserving part of `M` — which
/// is bounded by the smaller endpoint and by the kind-multiset
/// intersection. Both bounds are constant per node, so refusing an
/// alignment here can never refuse a pair the alignment would admit.
pub(super) fn kind_shared_upper_bound(left: &EndpointView, right: &EndpointView) -> usize {
    let smaller = left.total.min(right.total);
    smaller.min(kind_intersection(&left.kind_counts, &right.kind_counts))
}

/// Multiset-intersection cardinality of two kind-count maps.
fn kind_intersection(
    left: &HashMap<&'static str, usize>,
    right: &HashMap<&'static str, usize>,
) -> usize {
    let (small, large) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    small
        .iter()
        .map(|(kind, count)| (*count).min(large.get(kind).copied().unwrap_or(0)))
        .fold(0_usize, usize::saturating_add)
}

/// The upper bound as an overlap ratio against the larger endpoint —
/// directly comparable to `SHARED_SUBTREE_MIN_OVERLAP`
/// ([FUSED-SHARED-SUBTREE-BOUND]).
pub(super) fn kind_bound_ratio(left: &EndpointView, right: &EndpointView) -> f64 {
    let larger = left.total.max(right.total);
    if larger == 0 {
        return 0.0;
    }
    lossless_count(kind_shared_upper_bound(left, right)) / lossless_count(larger)
}

/// Fingerprints already count their normalised nodes. No alignment can
/// share more than the smaller endpoint, so this bound needs no view.
pub(crate) fn endpoint_count_bound(left: &Fingerprint, right: &Fingerprint) -> f64 {
    let larger = left.node_count.max(right.node_count);
    if larger == 0 {
        return 0.0;
    }
    lossless_count(left.node_count.min(right.node_count)) / lossless_count(larger)
}

/// Lossless small-count conversion for the coverage divisor.
pub(super) fn lossless_count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}
