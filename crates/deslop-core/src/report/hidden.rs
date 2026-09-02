//! Visibility decisions for the rendered report.
//!
//! Every rule here removes a cluster the pipeline built, so each one is
//! a deliberate precision trade with a defect behind it — and a
//! duplicate that silently disappears is the hardest defect class to
//! notice. The rules run at report materialisation, after the noise
//! split: a component the split convicted and split apart reaches this
//! pass as its surviving byte-identical families, and a component the
//! split could not change is convicted here or survives. Every hidden
//! cluster is counted in `clusters_hidden`, so a suppression is
//! observable telemetry, never a silent disappearance.
//!
//! The mass-only wire carries no bucket and no cluster signal, so the
//! rules are keyed on facts the report still exposes: report-hide
//! visibility, the language-agnostic and per-language noise patterns
//! ([CLONE-NOISE-*]). The embedding role guard runs before closure.

use std::hash::BuildHasher;

use crate::{
    cluster_filters::{is_noise_pattern, is_single_file_declaration_family, ParseCache},
    report_render::{cluster_to_report, ReportSources},
};

use super::{ReportCluster, ReportInputs};

/// Decides whether one cluster must be dropped from the ranked report.
///
/// The cheap test runs first: a cluster whose every occurrence sits in a
/// report-hidden path (e.g. all members in generated `*.g.dart` /
/// `*.freezed.dart` files) is dropped regardless of the expensive
/// re-parse checks below, so those are skipped. Without this a large
/// generated file is re-walked once per cluster only to be hidden anyway
/// ([CLONE-NOISE-REPARSE-CACHE]). The remaining rules:
/// - the recognised noise families of [CLONE-NOISE-*] — struct-field
///   runs, match-dispatch tables, signature-only matches, literal
///   variation calls, constant tables, generated-suffix scaffolds and
///   every other shape-only pattern a filter convicts. Each filter
///   guards its own verbatim escape hatch, so a byte-identical copy
///   survives its filter.
pub(crate) fn cluster_is_hidden<S: BuildHasher>(
    cluster: &crate::cluster::Cluster,
    report_cluster: &ReportCluster,
    inputs: &ReportInputs<'_, S>,
    parse_cache: &ParseCache,
) -> bool {
    let occurrences_all_hidden = !report_cluster.occurrences.is_empty()
        && report_cluster.occurrences.iter().all(|occ| occ.hidden);
    if occurrences_all_hidden {
        return true;
    }
    is_noise_pattern(
        &cluster.members,
        inputs.sources,
        inputs.file_languages,
        parse_cache,
    )
    .is_some()
        || is_single_file_declaration_family(
            cluster,
            inputs.sources,
            inputs.file_languages,
            parse_cache,
        )
}

/// Materialises one cluster and its visibility decision together, so
/// the metrics count exactly the clusters the report renders.
pub(crate) fn materialise_with_visibility<'a, S: BuildHasher>(
    cluster: &'a crate::cluster::Cluster,
    inputs: &ReportInputs<'_, S>,
    report_sources: &ReportSources<'a>,
    parse_cache: &ParseCache,
) -> (ReportCluster, bool) {
    let report_cluster = cluster_to_report(
        cluster,
        inputs.registry,
        inputs.file_languages,
        inputs.scan_root,
        inputs.exclusion,
        report_sources,
    );
    let hidden = cluster_is_hidden(cluster, &report_cluster, inputs, parse_cache);
    (report_cluster, hidden)
}

/// The stage label stamped on the run-cumulative noise totals emitted
/// once every render-stage conviction has run.
pub(crate) const NOISE_TOTALS_RUN_STAGE: &str = "run_cumulative_after_report_render";

/// Writes one trace line per hidden cluster so the routing decision is
/// recoverable without re-running the pipeline.
pub(crate) fn log_hidden_cluster(cluster: &ReportCluster, why: &str) {
    tracing::debug!(
        cluster = cluster.id.as_str(),
        occurrences = cluster.occurrences.len(),
        why,
        "cluster hidden from report",
    );
}
