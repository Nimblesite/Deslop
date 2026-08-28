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
    report::{CacheStats, Report, ReportCluster, ReportOccurrence, ReportSignals},
    report_metrics::RepoMetrics,
};

/// The signal triple a byte-proven clone renders: a saturated shape
/// match that the content gate had no reason to discount.
#[must_use]
pub fn identical_signals() -> ReportSignals {
    ReportSignals {
        structural: 1.0,
        token_jaccard: 1.0,
        shape: 1.0,
        embedding_cos: 0.0,
        fused: 1.0,
        agreement: 1.0,
        rename_consistency: 0.0,
        literal_fraction: 0.0,
    }
}

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

/// A complete rendered cluster over `occurrences`: an `identical`
/// byte-proven clone, ranked first, with every engine-derived field
/// stamped the way [`crate::report_restamp`] stamps it on a real report.
///
/// Suites override whatever they are pinning — weight, bucket, signals,
/// ids — and call [`restamp_fixture`] afterwards when they changed the
/// signals, so the shape reading, the gate verdict and the evidence
/// sentence stay consistent with the numbers the cluster now carries.
#[must_use]
pub fn fixture_cluster(id: &str, occurrences: Vec<ReportOccurrence>) -> ReportCluster {
    let mut cluster = ReportCluster {
        id: id.to_owned(),
        rank: 1,
        rank_band: String::new(),
        weight: 1.0,
        size: occurrences.len(),
        canonical_node_count: 4,
        signals: identical_signals(),
        signal_source: None,
        bucket: "identical".to_owned(),
        category: "logic".to_owned(),
        language: "rust".to_owned(),
        meets_fused_gate: false,
        evidence_verdict: String::new(),
        occurrences_total: occurrences.len(),
        occurrences,
        occurrence_count: 0,
        occurrences_truncated: false,
        summary: String::new(),
        interpretation: String::new(),
        intersects_diff: None,
        is_newly_introduced: None,
    };
    // A single-cluster report has no spread to express, so the engine
    // bands its only cluster `faint` ([SEVERITY-BAND]).
    "faint".clone_into(&mut cluster.rank_band);
    restamp_fixture(&mut cluster);
    cluster
}

/// Restamps a fixture's engine-derived fields through the one
/// definition, for a suite that changed its signals or occurrences.
pub fn restamp_fixture(cluster: &mut ReportCluster) {
    crate::report_restamp::restamp_cluster(cluster);
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
        action_hints: Vec::new(),
        boilerplate_hints: Vec::new(),
        embedding_provenance: None,
        clusters,
        clusters_outside_diff: None,
    }
}
