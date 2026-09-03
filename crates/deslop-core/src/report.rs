//! Canonical mass-only report assembly.

use std::{collections::HashMap, hash::BuildHasher, path::Path, time::Instant};

use crate::{
    boilerplate::BoilerplateRange,
    cluster::Cluster,
    cluster_filters::{noise_workers, NOISE_CHUNK_CLUSTERS},
    config::ExclusionConfig,
    fingerprint::Fingerprint,
    observe::elapsed_ms,
    report_boilerplate::build_boilerplate_hints,
    report_metrics::{compute_repo_metrics, AnalysedLines, MetricsInputs},
    report_render::ReportSources,
    report_weight::rank_by_mass,
    state::{FileId, FileRegistry},
};

mod hidden;
use hidden::{log_hidden_cluster, materialise_with_visibility, NOISE_TOTALS_RUN_STAGE};

pub use crate::wire_generated::{
    CacheStats, EmbeddingProvenance, PairClassification, PairComparison, PairComparisonParams,
    PairEndpoint, PairEvidence, Report, ReportCluster, ReportOccurrence,
};

/// The render-stage parse cache, re-exported where [`ReportInputs`]
/// carries it ([CLONE-NOISE-REPARSE-CACHE]).
pub use crate::cluster_filters::ParseCache;

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
    /// The shape families the clusters were admitted out of, indexed by
    /// [`Cluster::shape_family`] ([CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY]).
    pub shape_families: &'a [Vec<Fingerprint>],
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
    /// Shared parse cache for the render-stage noise checks
    /// ([CLONE-NOISE-REPARSE-CACHE]).
    pub parse_cache: &'a ParseCache,
}

/// [PERF-FLUTTER-TODO-SUBSUME] Materialises every cluster with its
/// visibility decision across worker threads sharing one parse cache —
/// the render-stage convictions run the same filters the noise split
/// shards, and ran here on one thread for a quarter of an hour on the
/// Flutter corpus. Results come back in input order, so the report is
/// the same whatever the workers' timing.
fn materialise_all<'a, S: BuildHasher + Sync>(
    inputs: &ReportInputs<'a, S>,
    report_sources: &ReportSources<'a>,
) -> Vec<(ReportCluster, bool)> {
    let (chunks, _states) = crate::shard::map_chunks(
        inputs.clusters.chunks(NOISE_CHUNK_CLUSTERS),
        noise_workers(inputs.clusters.len()),
        || (),
        |(), chunk: &[Cluster]| {
            chunk
                .iter()
                .map(|cluster| {
                    materialise_with_visibility(cluster, inputs, report_sources, inputs.parse_cache)
                })
                .collect::<Vec<(ReportCluster, bool)>>()
        },
    );
    chunks.into_iter().flatten().collect()
}

/// Converts closure components to the canonical report.
#[must_use]
pub fn render_report<S: BuildHasher + Sync>(inputs: ReportInputs<'_, S>) -> Report {
    let started = Instant::now();
    let report_sources = ReportSources::new(inputs.sources);
    let materialised = materialise_all(&inputs, &report_sources);
    // Every render-stage noise check has now run: `cluster_is_hidden` is
    // the only render-stage caller of the noise filters, and it is
    // reached solely from the loop above. Without this the render
    // stage's convictions were recorded into the shared counters and
    // discarded unread ([PERF-FLUTTER-TODO-OBSERVABILITY]).
    inputs.parse_cache.log_noise_totals(NOISE_TOTALS_RUN_STAGE);
    for (cluster, hidden) in &materialised {
        if *hidden {
            log_hidden_cluster(cluster, "noise or role gate");
        }
    }
    let visible_internal: Vec<&Cluster> = inputs
        .clusters
        .iter()
        .zip(&materialised)
        .filter_map(|(cluster, (_, hidden))| (!hidden).then_some(cluster))
        .collect();
    let clusters_hidden = materialised.iter().filter(|(_, hidden)| *hidden).count();
    let mut clusters: Vec<ReportCluster> = materialised
        .into_iter()
        .filter_map(|(cluster, hidden)| if hidden { None } else { Some(cluster) })
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
        stage = "report_build",
        visible_clusters = clusters.len(),
        clusters_hidden,
        highest_mass = clusters.first().map_or(0, |cluster| cluster.mass),
        elapsed_ms = elapsed_ms(started),
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
