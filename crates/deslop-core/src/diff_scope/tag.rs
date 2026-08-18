//! Wire-tag stamping and `--only-changed` filtering
//! ([OUTPUT-SCHEMA-DIFF-TAGS], [CLI-ARG-ONLY-CHANGED]).

use crate::wire_generated::{Report, ReportCluster, ReportOccurrence};

use super::DiffScope;

/// Stamps `in_diff` / `intersects_diff` / `is_newly_introduced` onto
/// every cluster ([OUTPUT-SCHEMA-DIFF-TAGS]). Tags describe, never
/// filter: the cluster list is unchanged in length and order.
pub fn tag_clusters(clusters: &mut [ReportCluster], scope: &DiffScope) {
    for cluster in clusters {
        tag_cluster(cluster, scope);
    }
}

/// Stamps one cluster. `intersects_diff` is true when any non-hidden
/// occurrence is in the diff; `is_newly_introduced` when every
/// non-hidden occurrence is — the whole visible family arrived with
/// the change.
fn tag_cluster(cluster: &mut ReportCluster, scope: &DiffScope) {
    for occurrence in &mut cluster.occurrences {
        occurrence.in_diff = Some(occurrence_in_diff(occurrence, scope));
    }
    let visible: Vec<bool> = cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .map(|occurrence| occurrence.in_diff == Some(true))
        .collect();
    let any_in_diff = visible.iter().any(|in_diff| *in_diff);
    cluster.intersects_diff = Some(any_in_diff);
    cluster.is_newly_introduced = Some(any_in_diff && visible.iter().all(|in_diff| *in_diff));
}

/// True when the occurrence's line range overlaps an added span.
fn occurrence_in_diff(occurrence: &ReportOccurrence, scope: &DiffScope) -> bool {
    let start = u64::try_from(occurrence.start_line).unwrap_or(0);
    let end = u64::try_from(occurrence.end_line).unwrap_or(0);
    scope.intersects(&occurrence.path, start, end)
}

/// Drops every cluster that does not intersect the diff and records
/// how many were omitted ([CLI-ARG-ONLY-CHANGED]). Metrics are never
/// touched — filtering changes what is listed, not what was measured.
pub fn apply_only_changed(report: &mut Report) {
    let before = report.clusters.len();
    report
        .clusters
        .retain(|cluster| cluster.intersects_diff == Some(true));
    report.clusters_outside_diff = Some(before.saturating_sub(report.clusters.len()));
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn occurrence(path: &str, start_line: i64, end_line: i64, hidden: bool) -> ReportOccurrence {
        ReportOccurrence {
            path: PathBuf::from(path),
            start_byte: 0,
            end_byte: 0,
            start_line,
            end_line,
            hidden,
            in_diff: None,
        }
    }

    fn cluster(occurrences: Vec<ReportOccurrence>) -> ReportCluster {
        ReportCluster {
            id: "cluster".to_owned(),
            weight: 1.0,
            size: occurrences.len(),
            canonical_node_count: 10,
            signals: crate::wire_generated::ReportSignals {
                structural: 1.0,
                token_jaccard: 1.0,
                embedding_cos: 0.0,
                fused: 1.0,
            },
            bucket: "identical".to_owned(),
            category: "logic".to_owned(),
            occurrences_total: occurrences.len(),
            occurrences,
            occurrences_truncated: false,
            summary: String::new(),
            interpretation: String::new(),
            intersects_diff: None,
            is_newly_introduced: None,
        }
    }

    fn scope_with(path: &str, lines: [u64; 2]) -> DiffScope {
        let mut scope = DiffScope::default();
        scope.insert_lines(PathBuf::from(path), lines);
        scope
    }

    // [OUTPUT-SCHEMA-DIFF-TAGS]: a mixed cluster intersects but is not
    // newly introduced; every occurrence carries an explicit verdict.
    #[test]
    fn mixed_cluster_intersects_without_being_newly_introduced() {
        let mut clusters = vec![cluster(vec![
            occurrence("src/new.rs", 1, 10, false),
            occurrence("src/old.rs", 1, 10, false),
        ])];
        tag_clusters(&mut clusters, &scope_with("src/new.rs", [2, 3]));
        assert_eq!(clusters[0].intersects_diff, Some(true));
        assert_eq!(clusters[0].is_newly_introduced, Some(false));
        assert_eq!(clusters[0].occurrences[0].in_diff, Some(true));
        assert_eq!(clusters[0].occurrences[1].in_diff, Some(false));
    }

    // [OUTPUT-SCHEMA-DIFF-TAGS]: hidden occurrences never decide the
    // cluster verdicts — a hidden out-of-diff copy cannot veto
    // `is_newly_introduced`.
    #[test]
    fn hidden_occurrences_do_not_veto_newly_introduced() {
        let mut clusters = vec![cluster(vec![
            occurrence("src/new.rs", 1, 5, false),
            occurrence("src/new.rs", 6, 9, false),
            occurrence("generated/gen.rs", 1, 5, true),
        ])];
        tag_clusters(&mut clusters, &scope_with("src/new.rs", [1, 9]));
        assert_eq!(clusters[0].is_newly_introduced, Some(true));
        assert_eq!(clusters[0].occurrences[2].in_diff, Some(false));
    }

    // [CLI-ARG-ONLY-CHANGED]: untouched clusters are dropped and
    // counted; intersecting clusters and metrics-bearing fields stay.
    #[test]
    fn only_changed_drops_and_counts_untouched_clusters() {
        let scope = scope_with("src/new.rs", [1, 5]);
        let mut touched = cluster(vec![occurrence("src/new.rs", 1, 5, false)]);
        let mut untouched = cluster(vec![occurrence("src/old.rs", 1, 5, false)]);
        tag_cluster(&mut touched, &scope);
        tag_cluster(&mut untouched, &scope);
        let mut report = Report {
            tool_version: "test".to_owned(),
            min_nodes: 30,
            files_analysed: 2,
            clusters_hidden: 0,
            cache_stats: crate::wire_generated::CacheStats::default(),
            metrics: crate::report_metrics::RepoMetrics::empty(),
            schema_doc: String::new(),
            action_hints: Vec::new(),
            boilerplate_hints: Vec::new(),
            embedding_provenance: None,
            clusters: vec![touched, untouched],
            clusters_outside_diff: None,
        };
        apply_only_changed(&mut report);
        assert_eq!(report.clusters.len(), 1);
        assert_eq!(report.clusters_outside_diff, Some(1));
        assert_eq!(report.clusters[0].intersects_diff, Some(true));
    }
}
