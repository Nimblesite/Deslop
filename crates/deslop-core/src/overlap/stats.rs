//! Overlap measurement counters ([PIPELINE-OBSERVABILITY-STAGES]).

/// Aggregate measurement counters for one [`OverlapMeasurer`]
/// ([FUSED-SHARED-SUBTREE-MEMO], [PIPELINE-OBSERVABILITY-STAGES]).
/// Snapshot via [`OverlapMeasurer::stats`]; the rescue and cluster
/// stages log them so cache effectiveness and alignment volume are
/// readable from any run.
#[derive(Debug, Default, Clone, Copy)]
pub struct MeasureStats {
    /// Pairs answered `1.0` by Merkle equality of the endpoints.
    pub hash_equal: u64,
    /// Pairs answered from the exact-overlap memo.
    pub exact_hits: u64,
    /// Rescue queries answered from the below-floor bound memo.
    pub bound_hits: u64,
    /// Rescue queries whose freshly computed kind-multiset bound proved
    /// the pair cannot clear the floor, skipping the alignment
    /// ([FUSED-SHARED-SUBTREE-BOUND]).
    pub bound_skips: u64,
    /// Rescue queries the multiset bound admitted but the ordered
    /// subsequence bound then proved cannot clear the floor
    /// ([FUSED-SHARED-SUBTREE-BOUND-ORDER]) — the alignments saved by
    /// respecting post-order rather than kind counts alone.
    pub order_skips: u64,
    /// Exact Zhang–Shasha alignments computed.
    pub alignments: u64,
    /// Greedy large-tree credit fallbacks computed.
    pub credit_fallbacks: u64,
    /// Pairs with an unresolvable endpoint, reported `0.0` uncached.
    pub unresolved: u64,
}

impl MeasureStats {
    /// Sums two counter snapshots — shard results merging in shard
    /// order, deterministically ([PERF-FLUTTER-TODO-RESCUE]).
    #[must_use]
    pub const fn add(self, other: MeasureStats) -> MeasureStats {
        crate::counters::summed_counters!(
            MeasureStats,
            self,
            other,
            hash_equal,
            exact_hits,
            bound_hits,
            bound_skips,
            order_skips,
            alignments,
            credit_fallbacks,
            unresolved,
        )
    }
}
