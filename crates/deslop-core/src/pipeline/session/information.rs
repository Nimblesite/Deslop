//! [CLONE-BUCKETS-STRUCTURAL-ONLY] Build informational findings separately from clones.

use std::{collections::HashMap, path::PathBuf, time::Instant};

use super::{super::store::relative_path_key, StageLedger};
use crate::{
    ast::NormalizedNode,
    buckets::ClusterKind,
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs, ClusterKindJudge},
    content::{tree_index_of, ContentMeasurer},
    fingerprint::Fingerprint,
    overlap::endpoint_count_bound,
    pair::FusedCluster,
    pipeline::PipelineSession,
    state::FileId,
};

/// Restricts materialisation to one side of the clone/information boundary.
struct KindFilter<'a> {
    /// Shared pair measurement, including known rename evidence.
    judge: &'a dyn ClusterKindJudge,
    /// True for clones, false for informational matches.
    clones: bool,
}

/// A pair connects two distinct member indices.
const PAIR_ENDPOINTS: usize = 2;

/// [CLONE-BUCKETS-STRUCTURAL-ONLY-COUNT-BOUND] Candidate families that
/// still need a pair-kind verdict for informational output.
fn eligible_information(
    families: &[FusedCluster],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str>,
    shape_floor: f64,
) -> Vec<FusedCluster> {
    let tree_index = tree_index_of(trees);
    let mut content = ContentMeasurer::default();
    let eligible: Vec<_> = families
        .iter()
        .filter(|family| {
            may_have_information_pair(
                family,
                fingerprints,
                &tree_index,
                &mut content,
                sources,
                languages,
                shape_floor,
            )
        })
        .cloned()
        .collect();
    tracing::info!(
        families = families.len(),
        eligible = eligible.len(),
        skipped = families.len().saturating_sub(eligible.len()),
        "informational count-bound workload"
    );
    eligible
}

/// Every classified shape-only pair must clear the configured overlap floor.
/// In sorted counts, any eligible pair has an eligible adjacent pair.
fn may_have_information_pair(
    family: &FusedCluster,
    fingerprints: &[Fingerprint],
    trees: &HashMap<FileId, &NormalizedNode>,
    content: &mut ContentMeasurer,
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str>,
    shape_floor: f64,
) -> bool {
    let Some(mut members) = family
        .members
        .iter()
        .map(|index| fingerprints.get(*index))
        .collect::<Option<Vec<_>>>()
    else {
        return true;
    };
    if same_shape_may_be_information(&members, trees, content, sources, languages) {
        return true;
    }
    members.sort_unstable_by_key(|found| found.node_count);
    members.windows(PAIR_ENDPOINTS).any(|pair| {
        matches!(pair, [left, right] if left.hash != right.hash && endpoint_count_bound(left, right) >= shape_floor)
    })
}

/// Equal shape remains eligible unless each same-hash pair proves matching
/// authored frontiers and no semantic contradiction.
fn same_shape_may_be_information(
    members: &[&Fingerprint],
    trees: &HashMap<FileId, &NormalizedNode>,
    content: &mut ContentMeasurer,
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str>,
) -> bool {
    let mut first_by_hash = HashMap::new();
    members.iter().any(|member| {
        first_by_hash
            .insert(member.hash, *member)
            .is_some_and(|first| {
                !same_shape_pair_is_exact(first, member, trees, content, sources, languages)
            })
    })
}

/// A skipped pair must clear both raw identity and canonical frontier checks.
fn same_shape_pair_is_exact(
    left: &Fingerprint,
    right: &Fingerprint,
    trees: &HashMap<FileId, &NormalizedNode>,
    content: &mut ContentMeasurer,
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str>,
) -> bool {
    super::super::pair_compare::same_source_bytes_and_language(left, right, sources, languages)
        && content
            .pair((left, right), trees, sources, languages)
            .exact_frontier_match(sources)
}

impl ClusterKindJudge for KindFilter<'_> {
    fn kind(&self, members: &[usize]) -> Option<ClusterKind> {
        self.judge
            .kind(members)
            .filter(|kind| kind.is_clone() == self.clones)
    }

    fn groups(&self, members: &[usize]) -> Vec<(Vec<usize>, ClusterKind)> {
        self.judge
            .groups(members)
            .into_iter()
            .filter(|(_, kind)| kind.is_clone() == self.clones)
            .collect()
    }

    fn span_kind(&self, left: &Fingerprint, right: &Fingerprint) -> Option<ClusterKind> {
        self.judge
            .span_kind(left, right)
            .filter(|kind| kind.is_clone() == self.clones)
    }
}

impl PipelineSession {
    /// Informational shapes cannot displace real copies during overlap suppression.
    pub(super) fn ranked_clusters(
        &self,
        fingerprints: &[Fingerprint],
        clones: &[FusedCluster],
        information: &[FusedCluster],
        trees: &[NormalizedNode],
        judge: &dyn ClusterKindJudge,
        ledger: &mut StageLedger,
    ) -> Vec<Cluster> {
        let paths = self.report_paths(fingerprints);
        let started = Instant::now();
        let mut findings = Vec::new();
        let eligible = eligible_information(
            information,
            fingerprints,
            trees,
            &self.sources,
            &self.file_languages,
            self.exclusion.routing().nearly_identical_min_shape,
        );
        for (fused_clusters, clones) in [(clones, true), (eligible.as_slice(), false)] {
            let kinds = KindFilter { judge, clones };
            findings.extend(build_ranked_fused_clusters(&ClusterBuildInputs {
                fingerprints,
                fused_clusters,
                trees,
                sources: &self.sources,
                file_languages: &self.file_languages,
                file_paths: &paths,
                kinds: &kinds,
            }));
        }
        ledger.record("ranked_build", clones.len(), findings.len(), started);
        findings
    }

    /// Stable relative paths are shared by clone and informational identities.
    fn report_paths(&self, fingerprints: &[Fingerprint]) -> HashMap<FileId, PathBuf> {
        fingerprints
            .iter()
            .filter_map(|found| {
                self.registry
                    .path(found.file_id)
                    .map(|path| (found.file_id, relative_path_key(path, &self.root)))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "information/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "information/exact_text_tests.rs"]
mod exact_text_tests;
