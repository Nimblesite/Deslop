//! Canonical mass-only report assembly.

use std::{collections::HashMap, hash::BuildHasher, path::Path};

use crate::{
    boilerplate::BoilerplateRange,
    cluster::Cluster,
    config::ExclusionConfig,
    report_boilerplate::build_boilerplate_hints,
    report_metrics::{compute_repo_metrics, AnalysedLines, MetricsInputs},
    report_render::{cluster_to_report, ReportSources},
    report_weight::rank_by_mass,
    state::{FileId, FileRegistry},
};

pub use crate::wire_generated::{
    CacheStats, EmbeddingProvenance, PairClassification, PairComparison, PairComparisonParams,
    PairEndpoint, PairEvidence, Report, ReportCluster, ReportOccurrence,
};

/// Default occurrence cap applied by [`Report::truncate_for_wire`].
pub const LIVE_WIRE_OCCURRENCE_CAP: usize = 100;

/// Canonical report schema documentation.
pub const SCHEMA_DOC: &str = include_str!("../../../docs/specs/REPORTING-CONTEXT.md");

/// Unlimited literal-finding cap until the literal detector supplies one.
const UNLIMITED_LITERAL_FINDINGS: usize = 0;

impl Report {
    /// Caps occurrence payloads without changing their authoritative totals.
    #[must_use]
    pub fn truncate_for_wire(mut self, cap: usize) -> Self {
        self.schema_doc.clear();
        for cluster in &mut self.clusters {
            if cluster.occurrences.len() > cap {
                cluster.occurrences.truncate(cap);
                cluster.occurrences_truncated = true;
            }
        }
        self
    }
}

/// Returns the authoritative total occurrence count.
#[must_use]
pub fn occurrence_count(cluster: &ReportCluster) -> usize {
    cluster.occurrences_total.max(cluster.occurrences.len())
}

/// Counts distinct visible paths in one cluster.
#[must_use]
pub fn distinct_visible_path_count(cluster: &ReportCluster) -> usize {
    cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .map(|occurrence| &occurrence.path)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// Inputs accepted by [`render_report`].
#[derive(Debug)]
pub struct ReportInputs<'a, S: BuildHasher> {
    /// Final closure components.
    pub clusters: &'a [Cluster],
    /// Registry resolving file identities to paths.
    pub registry: &'a FileRegistry,
    /// Language id by file.
    pub file_languages: &'a HashMap<FileId, &'static str, S>,
    /// Number of analysed files.
    pub files_analysed: usize,
    /// Configured subtree node floor.
    pub min_nodes: u32,
    /// Absolute scan root.
    pub scan_root: &'a Path,
    /// Exclusion and report-hide policy.
    pub exclusion: &'a ExclusionConfig,
    /// Embedding provider provenance.
    pub embedding_provenance: Option<EmbeddingProvenance>,
    /// Incremental-cache telemetry.
    pub cache_stats: CacheStats,
    /// Source bytes by file.
    pub sources: &'a HashMap<FileId, Vec<u8>>,
    /// Analysed lines by file.
    pub analysed_lines: &'a AnalysedLines,
    /// Suppressed import/prologue ranges.
    pub boilerplate_ranges: &'a [BoilerplateRange],
    /// Verified diff scope.
    pub diff: Option<&'a crate::diff_scope::DiffScope>,
}

/// Converts closure components to the canonical report.
#[must_use]
pub fn render_report<S: BuildHasher>(inputs: ReportInputs<'_, S>) -> Report {
    let report_sources = ReportSources::new(inputs.sources);
    let materialised: Vec<ReportCluster> = inputs
        .clusters
        .iter()
        .map(|cluster| {
            cluster_to_report(
                cluster,
                inputs.registry,
                inputs.file_languages,
                inputs.scan_root,
                inputs.exclusion,
                &report_sources,
            )
        })
        .collect();
    let visible_internal: Vec<&Cluster> = inputs
        .clusters
        .iter()
        .zip(&materialised)
        .filter_map(|(cluster, rendered)| (rendered.occurrence_count >= 2).then_some(cluster))
        .collect();
    let clusters_hidden = materialised
        .iter()
        .filter(|cluster| cluster.occurrence_count < 2)
        .count();
    let mut clusters: Vec<ReportCluster> = materialised
        .into_iter()
        .filter(|cluster| cluster.occurrence_count >= 2)
        .collect();
    rank_by_mass(&mut clusters);
    let mut metrics = compute_repo_metrics(&MetricsInputs {
        clusters: &visible_internal,
        sources: inputs.sources,
        line_indices: report_sources.line_indices(),
        file_languages: inputs.file_languages,
        registry: inputs.registry,
        exclusion: inputs.exclusion,
        analysed_lines: inputs.analysed_lines,
        scan_root: inputs.scan_root,
        diff: inputs.diff,
    });
    if let Some(scope) = inputs.diff {
        crate::diff_scope::tag_clusters(&mut clusters, scope);
    }
    metrics.threshold = inputs
        .exclusion
        .resolve_threshold(metrics.duplication_percent);
    let boilerplate_hints = build_boilerplate_hints(
        inputs.boilerplate_ranges,
        inputs.registry,
        inputs.scan_root,
        inputs.exclusion,
    );
    tracing::info!(
        visible_clusters = clusters.len(),
        clusters_hidden,
        highest_mass = clusters.first().map_or(0, |cluster| cluster.mass),
        "mass-ranked report built"
    );
    Report {
        tool_version: crate::version().to_owned(),
        min_nodes: inputs.min_nodes,
        files_analysed: inputs.files_analysed,
        clusters_hidden,
        cache_stats: inputs.cache_stats,
        metrics,
        schema_doc: SCHEMA_DOC.to_owned(),
        boilerplate_hints,
        embedding_provenance: inputs.embedding_provenance,
        clusters,
        clusters_outside_diff: None,
        literal_findings: Vec::new(),
        literal_findings_total: 0,
        literal_findings_hidden: 0,
        literal_findings_capped: false,
        literal_max_findings: UNLIMITED_LITERAL_FINDINGS,
    }
}
