//! Rendered-cluster fixtures for suites that need a report without
//! running the pipeline ([PIPELINE-DETERMINISM], [SEVERITY-BAND]).
//!
//! A hand-built [`ReportCluster`] literal was copied into a dozen suites
//! across three crates. Every wire field a surface reads has to be
//! answered in each copy, and a copy that omits one renders a zero
//! rather than failing to compile once the field gains a serde default —
//! the silent wrong answer the accuracy contract exists to prevent. One
//! builder instead, so a new wire field is answered once.
//!
//! Compiled only for tests: `deslop-core`'s own `#[cfg(test)]` modules
//! reach it directly, and the other crates enable it through the
//! `test-support` feature they already carry in their dev-dependencies.

use crate::{
    report::{CacheStats, Report, ReportCluster, ReportOccurrence},
    report_metrics::RepoMetrics,
};

/// One visible occurrence over `[start, end)` of `path`, with no line
/// information — suites that assert on lines set them explicitly.
#[must_use]
pub fn fixture_occurrence(path: &str, start: usize, end: usize) -> ReportOccurrence {
    ReportOccurrence {
        path: std::path::PathBuf::from(path),
        start_byte: start,
        end_byte: end,
        start_line: 0,
        end_line: 0,
        hidden: false,
        in_diff: None,
    }
}

/// A complete mass-only rendered cluster over `occurrences`.
#[must_use]
pub fn fixture_cluster(id: &str, occurrences: Vec<ReportOccurrence>) -> ReportCluster {
    let occurrence_count = occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .count();
    let canonical_node_count = 4;
    let mut cluster = ReportCluster {
        id: id.to_owned(),
        rank: 1,
        rank_band: "worst".to_owned(),
        mass: fixture_mass(canonical_node_count, occurrence_count),
        canonical_node_count,
        occurrences_total: occurrences.len(),
        occurrences,
        occurrence_count,
        occurrences_truncated: false,
        intersects_diff: None,
        is_newly_introduced: None,
    };
    restamp_fixture(&mut cluster);
    cluster
}

/// Restamps fixture mass and occurrence counts after membership changes.
pub fn restamp_fixture(cluster: &mut ReportCluster) {
    cluster.occurrences_total = cluster.occurrences.len();
    cluster.occurrence_count = cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .count();
    cluster.mass = fixture_mass(cluster.canonical_node_count, cluster.occurrence_count);
}

/// Computes the canonical fixture mass formula.
fn fixture_mass(canonical_node_count: usize, occurrence_count: usize) -> u64 {
    u64::try_from(canonical_node_count)
        .unwrap_or(u64::MAX)
        .saturating_mul(u64::try_from(occurrence_count.saturating_sub(1)).unwrap_or(u64::MAX))
}

/// A complete rendered report carrying `clusters` and nothing else —
/// no cache activity, no boilerplate hints, no embedding pass, and the
/// zeroed metrics a corpus with no analysed lines produces.
///
/// Suites that assert on report-level projections ([LIVE-DELTA]) need a
/// `Report` and not just its clusters; hand-building one answers a
/// dozen wire fields that have nothing to do with what is being
/// asserted, and a copy that omits one renders a zero rather than
/// failing to compile.
#[must_use]
pub fn fixture_report(clusters: Vec<ReportCluster>) -> Report {
    Report {
        tool_version: crate::version().to_owned(),
        min_nodes: 4,
        files_analysed: clusters.len(),
        clusters_hidden: 0,
        cache_stats: CacheStats::default(),
        metrics: RepoMetrics::default(),
        schema_doc: String::new(),
        boilerplate_hints: Vec::new(),
        embedding_provenance: None,
        clusters,
        clusters_outside_diff: None,
        literal_findings: Vec::new(),
        literal_findings_total: 0,
        literal_findings_hidden: 0,
        literal_findings_capped: false,
        literal_max_findings: 0,
    }
}
