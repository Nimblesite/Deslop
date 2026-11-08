//! Visibility decisions for the rendered report.
//!
//! Every rule here removes a cluster the pipeline built, so each one is
//! a deliberate precision trade with a defect behind it — and a
//! duplicate that silently disappears is the hardest defect class to
//! notice. The rules run at report materialisation, after the noise
//! split: a component the split convicted and split apart reaches this
//! pass as its surviving byte-identical families, and a component the
//! split could not change is convicted here or survives. Every hidden
//! clone is counted in `clusters_hidden`; informational suppressions appear only in detailed telemetry.
//!
//! The mass-only wire carries no bucket and no cluster signal, so the
//! rules are keyed on facts the report still exposes: report-hide
//! visibility, the language-agnostic and per-language noise patterns
//! ([CLONE-NOISE-*]). The embedding role guard runs before closure.

use std::{collections::BTreeMap, hash::BuildHasher};

use super::{ReportCluster, ReportInputs};
use crate::{
    cluster::{file_set, holds_shape_only, Cluster},
    cluster_filters::{
        escapes_as_copy, is_noise_pattern, is_single_file_declaration_family, ParseCache,
    },
    report_hide::ReportHide,
    report_render::{cluster_to_report, ReportSources},
    state::FileId,
};

/// Decides whether one cluster must be dropped from the ranked report.
///
/// The cheap test runs first: a cluster whose every occurrence sits in a
/// report-hidden path (e.g. all members in generated `*.g.dart` /
/// `*.freezed.dart` files) is dropped regardless of the expensive
/// re-parse checks below, so those are skipped. Without this a large
/// generated file is re-walked once per cluster only to be hidden anyway
/// ([CLONE-NOISE-REPARSE-CACHE]). The remaining rules:
/// - the shape family the cluster was admitted out of, when the
///   cluster alone is only a fragment of an idiom the filters would
///   convict whole ([CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY]);
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
        || shape_family_convicts(cluster, inputs, parse_cache)
        || is_single_file_declaration_family(
            cluster,
            inputs.sources,
            inputs.file_languages,
            parse_cache,
        )
}

/// [CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY] Whether the shape family the
/// cluster was admitted out of is a convicted idiom the cluster is a
/// fragment of.
///
/// A byte-identical copy the escape hatch protects is never hidden this
/// way: a cluster whose occurrences share exact bytes, spanning two
/// files where the convicting filter demands it, is the copy the family
/// was split to protect.
fn shape_family_convicts<S: BuildHasher>(
    cluster: &crate::cluster::Cluster,
    inputs: &ReportInputs<'_, S>,
    parse_cache: &ParseCache,
) -> bool {
    let Some(family) = cluster
        .shape_family
        .and_then(|index| inputs.shape_families.get(index))
    else {
        return false;
    };
    let same_shape = same_shape_members(family, &cluster.members);
    if same_shape.len() <= cluster.members.len() {
        return false;
    }
    is_noise_pattern(
        &same_shape,
        inputs.sources,
        inputs.file_languages,
        parse_cache,
    )
    .is_some_and(|filter| {
        !escapes_as_copy(
            &cluster.members,
            inputs.sources,
            filter,
            inputs.file_languages,
            parse_cache,
        )
    })
}

/// The members of a pre-gate family that share a shape with the admitted
/// cluster: the same normalised subtree, by Merkle hash. The pre-gate
/// closure also carries the containers and sub-windows rescued around
/// those shapes; a filter reading an ordered call sequence across mixed
/// depths sees no common header and convicts nothing, so the idiom's own
/// copies are what it is asked about ([CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY]).
fn same_shape_members(
    family: &[crate::fingerprint::Fingerprint],
    members: &[crate::fingerprint::Fingerprint],
) -> Vec<crate::fingerprint::Fingerprint> {
    let shapes: std::collections::HashSet<[u8; 32]> =
        members.iter().map(|member| member.hash).collect();
    family
        .iter()
        .filter(|member| shapes.contains(&member.hash))
        .cloned()
        .collect()
}

/// Materialises one cluster and its visibility decision together, so
/// the metrics count exactly the clusters the report renders.
pub(crate) fn materialise_with_visibility<'a, S: BuildHasher>(
    cluster: &'a crate::cluster::Cluster,
    inputs: &ReportInputs<'_, S>,
    report_sources: &ReportSources<'a>,
    hide: &ReportHide<'_>,
) -> (ReportCluster, bool) {
    let report_cluster = cluster_to_report(
        cluster,
        inputs.registry,
        inputs.file_languages,
        inputs.scan_root,
        hide,
        report_sources,
    );
    let hidden = cluster_is_hidden(cluster, &report_cluster, inputs, inputs.parse_cache);
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
        kind = cluster.kind.wire_label(),
        occurrences = cluster.occurrences.len(),
        why,
        "cluster hidden from report",
    );
}

/// [PIPELINE-CLUSTER-SUBSUME] [CLONE-BUCKETS-STRUCTURAL-ONLY] Hides every
/// shape-only view a *visible* clone strictly encloses, with some of it in
/// each of the clone's copies: the clone already reports that code, read
/// wider. Clones and shape-only views are resolved in separate passes, so
/// this is where they meet, and it runs after the noise filters because
/// only a published view may absorb another: a clone the filters hide
/// releases what it would have absorbed (`type3_enclosing_method`,
/// `python_issue_100_kwargs_ctor`).
pub(crate) fn absorb_shape_only_into_visible_clones(
    clusters: &[Cluster],
    materialised: &mut [(ReportCluster, bool)],
) {
    let holders = visible_clones_by_file_set(clusters, materialised);
    for (cluster, (report, hidden)) in clusters.iter().zip(materialised.iter_mut()) {
        if *hidden || cluster.kind.is_clone() {
            continue;
        }
        let absorbed = holders
            .get(&file_set(&cluster.members))
            .is_some_and(|outers| outers.iter().any(|outer| holds_shape_only(outer, cluster)));
        if absorbed {
            *hidden = true;
            log_hidden_cluster(report, "shape-only view held by a visible clone");
        }
    }
}

/// Visible clones grouped by the exact set of files they name
/// ([PIPELINE-CLUSTER-SUBSUME-FILESET]): a view can only enclose a view
/// over its own files.
fn visible_clones_by_file_set<'a>(
    clusters: &'a [Cluster],
    materialised: &[(ReportCluster, bool)],
) -> BTreeMap<Vec<FileId>, Vec<&'a Cluster>> {
    clusters
        .iter()
        .zip(materialised)
        .filter(|(cluster, (_, hidden))| !hidden && cluster.kind.is_clone())
        .fold(BTreeMap::new(), |mut groups, (cluster, _)| {
            groups
                .entry(file_set(&cluster.members))
                .or_insert_with(Vec::new)
                .push(cluster);
            groups
        })
}
