//! Report diff between two analysis generations.
//!
//! Implements the delta half of [LIVE-DELTA]: a pure projection over
//! two [`Report`] snapshots that exploits stable cluster ids
//! ([PIPELINE-CLUSTER-EXACT] / [PIPELINE-RANK-WORST-FIRST] assign the
//! id from the smallest member's hash) to classify every cluster as
//! added, removed, or updated. Subscribers (LSP, MCP, VSIX) consume
//! deltas instead of full snapshots so update traffic stays small when
//! one file changes in a repo with thousands of clusters.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    report::{Report, ReportCluster},
    wire_generated::LiteralFinding,
};

// `ReportDelta` is generated from `docs/models/live-ipc.td` by
// `scripts/typediagram/generate.mjs`. The data shape lives in
// `crate::wire_generated`; the `between`/`is_empty` impls stay here.
pub use crate::wire_generated::ReportDelta;

impl ReportDelta {
    /// Builds the delta between two snapshots. A `None` `prev` means
    /// "before any analysis" — every cluster in `next` appears under
    /// `clusters_added` and `from_generation` is `0`.
    ///
    /// Order of clusters in `clusters_added` and `clusters_updated`
    /// matches their order in `next.clusters`, which is worst-first
    /// per [PIPELINE-RANK-WORST-FIRST]. `clusters_removed` is sorted
    /// by id so the wire representation is deterministic for diff
    /// testing.
    #[must_use]
    pub fn between(prev: Option<(u64, &Report)>, to_generation: u64, next: &Report) -> Self {
        let prev_clusters = prev
            .map(|(_, report)| clusters_by_id(report))
            .unwrap_or_default();
        let from_generation = prev.map_or(0, |(generation, _)| generation);

        let mut clusters_added: Vec<ReportCluster> = Vec::new();
        let mut clusters_updated: Vec<ReportCluster> = Vec::new();
        let mut seen_ids: BTreeSet<&str> = BTreeSet::new();
        for cluster in &next.clusters {
            let _inserted = seen_ids.insert(cluster.id.as_str());
            match prev_clusters.get(cluster.id.as_str()) {
                None => clusters_added.push(cluster.clone()),
                Some(previous) if !clusters_equal(previous, cluster) => {
                    clusters_updated.push(cluster.clone());
                }
                Some(_) => {}
            }
        }

        let mut clusters_removed: Vec<String> = prev_clusters
            .keys()
            .filter(|id| !seen_ids.contains(*id))
            .map(|id| (*id).to_owned())
            .collect();
        clusters_removed.sort();
        let (literal_findings_added, literal_findings_removed, literal_findings_updated) =
            literal_changes(prev.map(|(_, report)| report), next);

        Self {
            from_generation,
            to_generation,
            clusters_added,
            clusters_removed,
            clusters_updated,
            literal_findings_added,
            literal_findings_removed,
            literal_findings_updated,
            metrics: next.metrics.clone(),
            cache_stats: next.cache_stats,
            tool_version: next.tool_version.clone(),
        }
    }

    /// Returns `true` when the delta carries no cluster changes. Used
    /// by the scheduler to decide whether to fire a `report/changed`
    /// notification ([LIVE-NOTIFICATIONS]) — a generation whose only
    /// change is cache stats is not user-visible.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clusters_added.is_empty()
            && self.clusters_removed.is_empty()
            && self.clusters_updated.is_empty()
            && self.literal_findings_added.is_empty()
            && self.literal_findings_removed.is_empty()
            && self.literal_findings_updated.is_empty()
    }
}

/// Computes deterministic literal-finding changes separately from clone clusters.
fn literal_changes(
    previous: Option<&Report>,
    next: &Report,
) -> (Vec<LiteralFinding>, Vec<String>, Vec<LiteralFinding>) {
    let previous = previous.map(literals_by_id).unwrap_or_default();
    let next_by_id = literals_by_id(next);
    let added = next
        .literal_findings
        .iter()
        .filter(|finding| !previous.contains_key(finding.id.as_str()))
        .cloned()
        .collect();
    let updated = next
        .literal_findings
        .iter()
        .filter(|finding| {
            previous
                .get(finding.id.as_str())
                .is_some_and(|old| *old != *finding)
        })
        .cloned()
        .collect();
    let removed = previous
        .keys()
        .filter(|id| !next_by_id.contains_key(**id))
        .map(|id| (*id).to_owned())
        .collect();
    (added, removed, updated)
}

/// Indexes literal findings by stable id.
fn literals_by_id(report: &Report) -> BTreeMap<&str, &LiteralFinding> {
    report
        .literal_findings
        .iter()
        .map(|finding| (finding.id.as_str(), finding))
        .collect()
}

/// Indexes a report's clusters by id. `BTreeMap` keeps the iteration
/// order deterministic for the `clusters_removed` sort.
fn clusters_by_id(report: &Report) -> BTreeMap<&str, &ReportCluster> {
    report
        .clusters
        .iter()
        .map(|cluster| (cluster.id.as_str(), cluster))
        .collect()
}

/// Returns `true` when two clusters with the same id are the same
/// cluster, field for field.
///
/// This was a hand-written list of the fields "a subscriber actually
/// observes", and it had drifted: `bucket`, `category`,
/// `occurrences_total`, `occurrences_truncated`, `intersects_diff` and
/// `is_newly_introduced` were absent from it, as were the content axes
/// of [`ReportSignals`] and the line numbers and diff tag of each
/// occurrence. A cluster could change bucket, change category, gain or
/// lose its diff tags, or move to different lines, and
/// [`ReportDelta::between`] would report nothing — leaving every live
/// subscriber rendering the previous generation's answer.
///
/// The list is gone. [`ReportCluster`] derives [`PartialEq`] in the
/// generated wire module, so the comparison covers every field the wire
/// carries, and a field added to `docs/models/live-ipc.td` is covered
/// the day it lands rather than the day someone remembers this
/// function.
fn clusters_equal(left: &ReportCluster, right: &ReportCluster) -> bool {
    left == right
}
