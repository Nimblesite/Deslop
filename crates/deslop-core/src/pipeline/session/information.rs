//! [CLONE-BUCKETS-STRUCTURAL-ONLY] Build informational findings separately from clones.

use std::{collections::HashMap, path::PathBuf, time::Instant};

use super::{super::store::relative_path_key, StageLedger};
use crate::{
    ast::NormalizedNode,
    buckets::ClusterKind,
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs, ClusterKindJudge},
    fingerprint::Fingerprint,
    pair::{missing_structural_pairs, CandidatePair, FusedCluster},
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
}

impl PipelineSession {
    /// [CLONE-KIND-FOLD] An unrelated star centre cannot erase copies among its neighbours.
    pub(super) fn recover_copy_pairs(
        &self,
        pairs: &mut Vec<CandidatePair>,
        families: &[FusedCluster],
        fingerprints: &[Fingerprint],
        judge: &dyn ClusterKindJudge,
    ) {
        let groups = families
            .iter()
            .flat_map(|family| judge.groups(&family.members))
            .filter_map(|(members, kind)| kind.is_clone().then_some(members));
        pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
        let recovered = self.build_missing_copy_pairs(fingerprints, groups, pairs);
        tracing::debug!(recovered = recovered.len(), "copy edges recovered");
        pairs.extend(recovered);
    }

    /// Uses the same candidate constructor and language policy as initial discovery.
    fn build_missing_copy_pairs(
        &self,
        fingerprints: &[Fingerprint],
        groups: impl Iterator<Item = Vec<usize>>,
        existing: &[CandidatePair],
    ) -> Vec<CandidatePair> {
        missing_structural_pairs(
            fingerprints,
            &self.store.signatures(),
            groups,
            existing,
            &self.file_languages,
            self.exclusion.allows_cross_language_comparison(),
        )
    }

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
        for (fused_clusters, clones) in [(clones, true), (information, false)] {
            let kinds = KindFilter { judge, clones };
            findings.extend(build_ranked_fused_clusters(&ClusterBuildInputs {
                fingerprints,
                fused_clusters,
                trees,
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
