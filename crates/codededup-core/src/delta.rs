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

use serde::{Deserialize, Serialize};

use crate::report::{CacheStats, Report, ReportCluster};

/// Diff between two report generations.
///
/// `from_generation` and `to_generation` are monotonic counters owned
/// by the session that produced the deltas; they are opaque to the
/// delta itself. `clusters_added` / `clusters_removed` / `clusters_updated`
/// partition the symmetric difference of cluster ids — a cluster whose
/// payload is bit-identical across generations lands in none of the
/// three.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDelta {
    /// Generation of the earlier report (the `prev` argument to
    /// [`ReportDelta::between`]). `0` is reserved for the
    /// synthetic "before any analysis" generation so the very first
    /// delta carries `from_generation: 0`.
    pub from_generation: u64,
    /// Generation of the later report.
    pub to_generation: u64,
    /// Clusters present in `to` but not in `from`, keyed by cluster id.
    /// Ordered worst-first to mirror the report's own ordering so a
    /// subscriber applying the delta sees the most impactful new
    /// clusters first.
    pub clusters_added: Vec<ReportCluster>,
    /// Cluster ids present in `from` but not in `to`. Id-only because
    /// the subscriber already has the payload from the previous
    /// delivery.
    pub clusters_removed: Vec<String>,
    /// Clusters present in both snapshots whose payload changed
    /// (weight, size, signals, occurrences, interpretation). Ordered
    /// worst-first.
    pub clusters_updated: Vec<ReportCluster>,
    /// Cache telemetry for the generation-producing run.
    pub cache_stats: CacheStats,
    /// Producer version stamped on the later snapshot. Lets a client
    /// that missed generations detect tool upgrades without parsing a
    /// full report.
    pub tool_version: String,
}

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

        Self {
            from_generation,
            to_generation,
            clusters_added,
            clusters_removed,
            clusters_updated,
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
    }
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

/// Returns `true` when two clusters with the same id carry the same
/// user-visible payload. Avoids a full structural `PartialEq` on
/// [`ReportCluster`] (which would need derives across the whole report
/// type surface) by comparing the fields a subscriber actually
/// observes.
fn clusters_equal(left: &ReportCluster, right: &ReportCluster) -> bool {
    (left.weight - right.weight).abs() <= f64::EPSILON
        && left.size == right.size
        && left.canonical_node_count == right.canonical_node_count
        && signals_equal(left.signals, right.signals)
        && left.summary == right.summary
        && left.interpretation == right.interpretation
        && occurrences_equal(&left.occurrences, &right.occurrences)
}

/// Bit-approximate equality for the four signal components.
fn signals_equal(left: crate::report::ReportSignals, right: crate::report::ReportSignals) -> bool {
    (left.structural - right.structural).abs() <= f64::EPSILON
        && (left.token_jaccard - right.token_jaccard).abs() <= f64::EPSILON
        && (left.embedding_cos - right.embedding_cos).abs() <= f64::EPSILON
        && (left.fused - right.fused).abs() <= f64::EPSILON
}

/// Exact equality across two occurrence lists. Ordered — occurrence
/// order is stable under [PIPELINE-CLUSTER-EXACT] so a reorder would
/// itself be a semantic change.
fn occurrences_equal(
    left: &[crate::report::ReportOccurrence],
    right: &[crate::report::ReportOccurrence],
) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right.iter()).all(|(a, b)| {
        a.path == b.path
            && a.start_byte == b.start_byte
            && a.end_byte == b.end_byte
            && a.hidden == b.hidden
    })
}
