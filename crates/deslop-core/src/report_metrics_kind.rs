//! Sub-metrics per clone kind ([METRICS-TOTAL-INCLUDE]).

use std::collections::{BTreeSet, HashMap};

use crate::{
    buckets::{ClusterKind, TotalInclude},
    report_metrics::{fold_cluster_lines, percent, KindMetric, MetricsInputs},
    state::FileId,
    wire_generated::ThresholdSummary,
};

/// Projects every visible clone cluster's non-hidden occurrences into
/// one per-file line-set bucket per clone kind.
pub(crate) fn fold_kind_buckets(
    inputs: &MetricsInputs<'_>,
) -> HashMap<ClusterKind, HashMap<FileId, BTreeSet<u64>>> {
    let mut buckets: HashMap<ClusterKind, HashMap<FileId, BTreeSet<u64>>> = HashMap::new();
    for &cluster in inputs.clusters {
        if !cluster.kind.is_clone() {
            continue;
        }
        fold_cluster_lines(cluster, inputs, buckets.entry(cluster.kind).or_default());
    }
    buckets
}

/// Unions the line-set buckets of every included kind into the total's
/// per-file projection. Overlapping clusters of different kinds share
/// lines through the union, exactly as overlapping clusters of one kind
/// share them through the set.
pub(crate) fn union_included_lines(
    buckets: &HashMap<ClusterKind, HashMap<FileId, BTreeSet<u64>>>,
    include: &TotalInclude,
) -> HashMap<FileId, BTreeSet<u64>> {
    let mut total: HashMap<FileId, BTreeSet<u64>> = HashMap::new();
    for kind in include.kinds() {
        let Some(bucket) = buckets.get(kind) else {
            continue;
        };
        for (file_id, lines) in bucket {
            total.entry(*file_id).or_default().extend(lines);
        }
    }
    total
}

/// Builds the [METRICS-TOTAL-INCLUDE] `per_kind` rows: one per clone
/// kind with a visible cluster, in category display order, each carrying
/// its own figures whether or not it counts toward the total. Thresholds
/// stay `none()` here; the config layer resolves them beside the total's.
pub(crate) fn kind_metrics(
    buckets: &HashMap<ClusterKind, HashMap<FileId, BTreeSet<u64>>>,
    analysed_loc: u64,
    include: &TotalInclude,
) -> Vec<KindMetric> {
    ClusterKind::all()
        .into_iter()
        .filter_map(|kind| single_kind_metric(kind, buckets, analysed_loc, include))
        .collect()
}

/// Computes one [`KindMetric`] row for `kind` if it has visible duplicated lines.
fn single_kind_metric(
    kind: ClusterKind,
    buckets: &HashMap<ClusterKind, HashMap<FileId, BTreeSet<u64>>>,
    analysed_loc: u64,
    include: &TotalInclude,
) -> Option<KindMetric> {
    let duplicated_loc = buckets.get(&kind)?;
    let lines: u64 = duplicated_loc
        .values()
        .map(|set| u64::try_from(set.len()).unwrap_or(u64::MAX))
        .sum();
    Some(KindMetric {
        kind,
        duplicated_loc: lines,
        duplication_percent: percent(lines, analysed_loc),
        included_in_total: include.contains(kind),
        threshold: ThresholdSummary::none(),
    })
}
