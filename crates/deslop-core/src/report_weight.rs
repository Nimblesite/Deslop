//! Ranking weight for rendered clusters ([PIPELINE-RANK-WORST-FIRST],
//! [RANK-CATEGORY], [RANK-STRUCTURAL-ONLY], [FUSION-CONTENT-GATE]).
//!
//! One place decides how a visible cluster's weight is composed:
//! geometry (visible occurrences × canonical nodes) scaled by the bucket
//! and category multipliers, then by the content-gated fused confidence.
//! Split out of `report.rs` to keep both files inside the size budget.

use crate::{
    buckets::{classify, ClusterKind},
    clone_category::CloneCategory,
    config::RankingPolicy,
    report::ReportCluster,
};

/// Re-ranks visible clusters by non-hidden occurrence count so mixed
/// clusters dominated by `report_hide` paths cannot push fully-visible
/// clusters down the ranking ([#140 EXCLUSION-CONFIG],
/// [PIPELINE-RANK-WORST-FIRST]). The clone-category multiplier from
/// [RANK-CATEGORY] and the structural-only multiplier from
/// [RANK-STRUCTURAL-ONLY] are folded in here so a `data`-category or
/// shape-only-evidence cluster sinks below comparable full-evidence
/// clones; both multipliers are `1.0` in `keep`/`ignore` modes and for
/// non-matching clusters, which therefore keep their prior weight.
/// The content-gated fused confidence then scales the weight continuously
/// ([FUSION-CONTENT-GATE]): the bucket multipliers demote by class, the
/// confidence orders within and across classes — at equal geometry a
/// byte-proven copy outranks a consistent rename, which outranks
/// shape-only coincidence, and two same-bucket clusters rank by how much
/// of their content actually agrees. Hidden occurrences still travel on
/// each cluster for downstream context.
pub(crate) fn reweigh_by_visible_occurrences(
    clusters: &mut [ReportCluster],
    policy: RankingPolicy,
) {
    for cluster in &mut *clusters {
        let visible = visible_occurrence_count(cluster);
        let base = visible_rank_weight(cluster.canonical_node_count, visible);
        cluster.weight = base
            * category_multiplier(cluster, policy)
            * structural_only_multiplier(cluster, policy)
            * confidence_factor(cluster);
    }
    clusters.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
}

/// Returns the ranking-weight multiplier for `cluster` under `policy`
/// ([RANK-CATEGORY]). `data`-category clusters get the policy's demote
/// multiplier; everything else stays at `1.0`.
fn category_multiplier(cluster: &ReportCluster, policy: RankingPolicy) -> f64 {
    match CloneCategory::from_wire_label(&cluster.category) {
        CloneCategory::DataTable => policy.data_weight_multiplier(),
        CloneCategory::Logic => 1.0,
    }
}

/// Returns the ranking-weight multiplier for structural-only clusters
/// under `policy` ([RANK-STRUCTURAL-ONLY]). Keyed off the same
/// [`ClusterKind::StructuralOnly`] routing that assigns the wire label,
/// so a labelled cluster is always the cluster the policy demotes
/// (inconsistency #1). `data`-category clusters are exempt:
/// their weight belongs to the more specific, user-configurable
/// `[ranking] data_clones` policy ([RANK-CATEGORY]), and stacking both
/// demotions would make `data_clone_weight = 1.0` unable to restore a
/// table that the content gate ([FUSION-CONTENT-GATE]) also routed to
/// the structural-only bucket.
fn structural_only_multiplier(cluster: &ReportCluster, policy: RankingPolicy) -> f64 {
    if classify(cluster) == ClusterKind::StructuralOnly
        && CloneCategory::from_wire_label(&cluster.category) != CloneCategory::DataTable
    {
        policy.structural_only_weight_multiplier()
    } else {
        1.0
    }
}

/// Continuous confidence factor for the ranking weight
/// ([FUSION-CONTENT-GATE]): the content-gated fused confidence, except
/// for `data`-category clusters, whose demotion belongs exclusively to
/// the user-configurable `[ranking] data_clones` policy
/// ([RANK-CATEGORY]). Stacking the confidence demotion on top would
/// make `data_clone_weight = 1.0` unable to restore a table — the same
/// knob contract [`structural_only_multiplier`] already honours.
fn confidence_factor(cluster: &ReportCluster) -> f64 {
    if CloneCategory::from_wire_label(&cluster.category) == CloneCategory::DataTable {
        return 1.0;
    }
    cluster.signals.fused
}

/// Counts non-hidden occurrences on a rendered cluster. Hidden
/// occurrences still travel with the cluster so consumers retain the
/// "regular code duplicates generated code" context, but ranking
/// ignores them.
fn visible_occurrence_count(cluster: &ReportCluster) -> usize {
    cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .count()
}

/// Mirrors [PIPELINE-RANK-WORST-FIRST] but feeds it the visible
/// occurrence count. Empty visible sets score zero so a cluster that
/// is technically not all-hidden but has only one actionable copy
/// sinks below cleaner clusters with more refactorable duplication.
fn visible_rank_weight(canonical_node_count: usize, visible_size: usize) -> f64 {
    if visible_size < 2 {
        return 0.0;
    }
    let nodes = lossless_u32_to_f64(canonical_node_count.max(1));
    let size_minus_one = lossless_u32_to_f64(visible_size.saturating_sub(1));
    nodes * size_minus_one
}

/// Converts a `usize` to `f64` losslessly. Values past `u32::MAX` are
/// clamped — cluster cardinalities never reach that range in
/// practice but the clamp keeps the math precision-safe under the
/// workspace's `cast_precision_loss` lint.
fn lossless_u32_to_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}
