//! Ranking weight for rendered clusters ([PIPELINE-RANK-WORST-FIRST],
//! [RANK-MASS-SUM], [RANK-CATEGORY], [RANK-STRUCTURAL-ONLY],
//! [FUSED-CONTENT-GATE]).
//!
//! One place decides how a visible cluster's weight is composed: the
//! duplicated mass (visible occurrences × canonical nodes), scaled only
//! by the bucket and category policy multipliers. Confidence never
//! discounts the weight — it already did its job at admission
//! ([FUSED-CLUSTER-SIGNALS], Baker 1995: a pair either p-matches or it
//! does not), and re-discounting it erases duplicated-line mass at
//! ranking (gh #458, [RANK-MASS-SUM]). Split out of `report.rs` to keep
//! both files inside the size budget.

use crate::{
    buckets::{classify, ClusterKind},
    clone_category::CloneCategory,
    config::RankingPolicy,
    report::ReportCluster,
};

/// Re-ranks visible clusters by non-hidden occurrence count so mixed
/// clusters dominated by `report_hide` paths cannot push fully-visible
/// clusters down the ranking ([#140 EXCLUSION-CONFIG],
/// [PIPELINE-RANK-WORST-FIRST]). The weight is the duplicated mass —
/// canonical nodes × member pairs, the mass to fix
/// ([RANK-MASS-SUM]) — never confidence-discounted, so a five-member
/// near-miss family outranks a two-member byte-identical pair when its
/// mass is larger (gh #458). The clone-category multiplier from
/// [RANK-CATEGORY] and the structural-only multiplier from
/// [RANK-STRUCTURAL-ONLY] are folded in here so a `data`-category or
/// shape-only-evidence cluster sinks below comparable full-evidence
/// clones; both multipliers are `1.0` in `keep`/`ignore` modes and for
/// non-matching clusters, which therefore keep their prior weight.
/// At equal mass cluster id makes the order total and reproducible
/// ([RANK-MASS-SUM]); there is no fused tie-break — confidence did
/// its job at admission ([FUSED-SCOPE]). Hidden occurrences still
/// travel on each cluster for downstream context.
pub(crate) fn reweigh_by_visible_occurrences(
    clusters: &mut [ReportCluster],
    policy: RankingPolicy,
) {
    for cluster in &mut *clusters {
        let visible = visible_occurrence_count(cluster);
        let base = visible_rank_weight(cluster.canonical_node_count, visible);
        cluster.weight = base
            * category_multiplier(cluster, policy)
            * structural_only_multiplier(cluster, policy);
    }
    clusters.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    stamp_ranks(clusters);
}

/// Stamps each cluster's worst-first position and severity band
/// ([PIPELINE-RANK-WORST-FIRST], [SEVERITY-BAND],
/// [VSIX-TOP-OFFENDERS-RANK-GLOBAL]).
///
/// Rank is 1-based over the whole report, so a client that renders any
/// subset — one file's clusters, one folder's, one language's — still
/// shows each cluster's standing in the repository rather than its
/// position in the slice it happens to be looking at. The band is the
/// engine's classification of that rank, not the client's: consumers
/// render `rank_band` verbatim and never re-derive a percentile.
pub(crate) fn stamp_ranks(clusters: &mut [ReportCluster]) {
    let total = clusters.len();
    for (index, cluster) in clusters.iter_mut().enumerate() {
        let rank = index.saturating_add(1);
        cluster.rank = rank;
        rank_band(rank, total).clone_into(&mut cluster.rank_band);
    }
}

/// The severity band for a 1-based `rank` out of `total`
/// ([SEVERITY-BAND]). Four bands over the rank percentile: the worst
/// percent, the worst tenth, the worse half, and the tail.
fn rank_band(rank: usize, total: usize) -> &'static str {
    let percentile = rank_percentile(rank, total);
    if percentile >= 0.99 {
        "worst"
    } else if percentile >= 0.9 {
        "top10"
    } else if percentile >= 0.5 {
        "mid"
    } else {
        "faint"
    }
}

/// Position of a 1-based `rank` in `[0, 1]`, worst first: rank 1 is
/// `1.0`, the last rank is `0.0` ([SEVERITY-BAND]). A single-cluster
/// report has no spread to express, so it reads `0.0`.
fn rank_percentile(rank: usize, total: usize) -> f64 {
    if total <= 1 {
        return 0.0;
    }
    1.0 - (lossless_u32_to_f64(rank.saturating_sub(1))
        / lossless_u32_to_f64(total.saturating_sub(1)))
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
/// table that the content gate ([FUSED-CONTENT-GATE]) also routed to
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A cluster carrying only what the ranking pass reads.
    fn ranked(id: &str) -> ReportCluster {
        crate::report_fixtures::fixture_cluster(id, Vec::new())
    }

    /// [SEVERITY-BAND] The four cut points every visual surface's glyph
    /// density depends on. Held here because this is the only place they
    /// exist: a client that re-derived them could drift silently.
    #[test]
    fn rank_band_cut_points() {
        assert_eq!(
            rank_band(1, 100),
            "worst",
            "rank 1 of 100 tops the percentile"
        );
        assert_eq!(rank_band(5, 100), "top10");
        assert_eq!(rank_band(40, 100), "mid");
        assert_eq!(rank_band(80, 100), "faint");
        assert_eq!(
            rank_band(100, 100),
            "faint",
            "the last rank sits at percentile 0"
        );
        assert_eq!(
            rank_band(1, 1),
            "faint",
            "a single-cluster report has no spread to express"
        );
        assert!((rank_percentile(1, 100) - 1.0).abs() < f64::EPSILON);
        assert!(rank_percentile(100, 100).abs() < f64::EPSILON);
        assert!(rank_percentile(1, 1).abs() < f64::EPSILON);
    }

    /// [PIPELINE-RANK-WORST-FIRST] Ranks are 1..n over the whole report,
    /// so a client rendering any subset still shows repository standing.
    #[test]
    fn stamp_ranks_numbers_the_whole_report() {
        let mut clusters: Vec<ReportCluster> =
            (0..10).map(|index| ranked(&format!("c{index}"))).collect();
        stamp_ranks(&mut clusters);
        let ranks: Vec<usize> = clusters.iter().map(|cluster| cluster.rank).collect();
        assert_eq!(ranks, (1..=10).collect::<Vec<usize>>());
        let bands: Vec<&str> = clusters
            .iter()
            .map(|cluster| cluster.rank_band.as_str())
            .collect();
        assert_eq!(bands.first(), Some(&"worst"), "the worst offender leads");
        assert_eq!(bands.last(), Some(&"faint"), "the tail is quietest");
        assert!(
            clusters.iter().all(|cluster| !cluster.rank_band.is_empty()),
            "every rendered cluster carries a band"
        );
    }

    /// [SEVERITY-BAND] The band can only quieten as the ranking worsens.
    /// A brightening band would make the decoration order contradict the
    /// ranking the report itself publishes.
    #[test]
    fn rank_band_never_brightens_down_the_report() {
        let order = ["worst", "top10", "mid", "faint"];
        let mut clusters: Vec<ReportCluster> =
            (0..40).map(|index| ranked(&format!("c{index}"))).collect();
        stamp_ranks(&mut clusters);
        let mut previous = 0;
        for cluster in &clusters {
            let position = order
                .iter()
                .position(|band| *band == cluster.rank_band)
                .unwrap_or(order.len());
            assert!(
                position < order.len(),
                "{} carries an unknown band {}",
                cluster.id,
                cluster.rank_band
            );
            assert!(
                position >= previous,
                "{} at rank {} brightened to {}",
                cluster.id,
                cluster.rank,
                cluster.rank_band
            );
            previous = position;
        }
    }
}
