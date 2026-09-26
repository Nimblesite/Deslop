//! [CLONE-BUCKETS-STRUCTURAL-ONLY] Build informational findings separately from clones.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    time::Instant,
};

use super::{super::store::relative_path_key, StageLedger};
use crate::{
    ast::NormalizedNode,
    buckets::ClusterKind,
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs, ClusterKindJudge},
    content::{tree_index_of, ContentMeasurer},
    fingerprint::Fingerprint,
    overlap::endpoint_count_bound,
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

/// A connected pair already has its only edge; larger complete components
/// likewise have no missing relation for copy recovery to discover.
const MIN_RECOVERY_MEMBERS: usize = 3;
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

/// [FUSED-CANDIDATE-BUCKET-RECOVERY] Only equal hashes can add structural edges.
fn recovery_hash_groups(family: &FusedCluster, fingerprints: &[Fingerprint]) -> Vec<Vec<usize>> {
    let mut by_hash: BTreeMap<[u8; 32], Vec<usize>> = BTreeMap::new();
    for &member in &family.members {
        if let Some(fingerprint) = fingerprints.get(member) {
            by_hash.entry(fingerprint.hash).or_default().push(member);
        } else {
            tracing::error!(member, "copy recovery member outside fingerprint corpus");
        }
    }
    by_hash
        .into_values()
        .filter(|members| members.len() >= PAIR_ENDPOINTS)
        .collect()
}

/// [CLONE-KIND-FOLD] Avoid kind measurements on complete families and on
/// cross-hash pairs that recovery cannot add.
fn clone_groups(members: &[usize], judge: &dyn ClusterKindJudge) -> Vec<Vec<usize>> {
    judge
        .groups(members)
        .into_iter()
        .filter_map(|(members, kind)| kind.is_clone().then_some(members))
        .collect()
}

/// Mixed families need direct pair proofs: a greedy group can hide a clone
/// when an earlier cross-hash member consumes its possible reference.
fn certified_hash_pairs(
    members: &[usize],
    judge: &dyn ClusterKindJudge,
    existing: &[CandidatePair],
) -> Vec<Vec<usize>> {
    members
        .iter()
        .enumerate()
        .flat_map(|(offset, left)| {
            members
                .iter()
                .skip(offset.saturating_add(1))
                .filter_map(move |right| {
                    if candidate_exists(existing, *left, *right) {
                        return None;
                    }
                    judge
                        .kind(&[*left, *right])
                        .filter(|kind| kind.is_clone())
                        .map(|_kind| vec![*left, *right])
                })
        })
        .collect()
}

/// Existing candidates retain their original evidence; recovery cannot
/// reconstruct them, so their clone verdict is unnecessary here.
fn candidate_exists(existing: &[CandidatePair], left: usize, right: usize) -> bool {
    let key = (left.min(right), left.max(right));
    existing
        .binary_search_by_key(&key, |pair| (pair.left, pair.right))
        .is_ok()
}

/// Keep the original greedy fold for a single-hash family; mixed families
/// measure every same-hash pair so a nontransitive clone cannot disappear.
fn family_recovery_groups(
    family: &FusedCluster,
    fingerprints: &[Fingerprint],
    judge: &dyn ClusterKindJudge,
    existing: &[CandidatePair],
) -> Vec<Vec<usize>> {
    let groups = recovery_hash_groups(family, fingerprints);
    if groups.len() == 1
        && groups
            .first()
            .is_some_and(|members| members.len() == family.members.len())
    {
        return clone_groups(&family.members, judge);
    }
    groups
        .iter()
        .flat_map(|members| certified_hash_pairs(members, judge, existing))
        .collect()
}

/// The clone-certified groups or pairs that may add structural edges.
fn recovery_groups<'a>(
    families: &'a [FusedCluster],
    fingerprints: &'a [Fingerprint],
    judge: &'a dyn ClusterKindJudge,
    existing: &'a [CandidatePair],
) -> impl Iterator<Item = Vec<usize>> + 'a {
    families
        .iter()
        .filter(|family| has_missing_pair(family))
        .flat_map(|family| family_recovery_groups(family, fingerprints, judge, existing))
}

/// Unique valid edges must fill every possible member pair before we skip.
fn has_missing_pair(family: &FusedCluster) -> bool {
    let count = family.members.len();
    if count < MIN_RECOVERY_MEMBERS {
        return false;
    }
    let Some(expected) = count
        .checked_mul(count.saturating_sub(1))
        .map(|edges| edges / PAIR_ENDPOINTS)
    else {
        return true;
    };
    if family.edges.len() < expected {
        return true;
    }
    unique_valid_edges(family) < expected
}

/// Ignore duplicate, self, and out-of-component edges before skipping work.
fn unique_valid_edges(family: &FusedCluster) -> usize {
    let members: HashSet<_> = family.members.iter().copied().collect();
    family
        .edges
        .iter()
        .filter(|edge| {
            edge.left != edge.right && members.contains(&edge.left) && members.contains(&edge.right)
        })
        .map(|edge| (edge.left.min(edge.right), edge.left.max(edge.right)))
        .collect::<HashSet<_>>()
        .len()
}

/// [PERF-FLUTTER-TODO-OBSERVABILITY] Count the potentially measured pairs
/// without logging source paths or contents.
fn recovery_workload(
    families: &[FusedCluster],
    fingerprints: &[Fingerprint],
) -> (usize, usize, usize) {
    families
        .iter()
        .filter(|family| has_missing_pair(family))
        .flat_map(|family| recovery_hash_groups(family, fingerprints))
        .fold((0, 0, 0), |(groups, pairs, largest), members| {
            let count = members.len();
            let comparisons = count.saturating_mul(count.saturating_sub(1)) / PAIR_ENDPOINTS;
            (
                groups.saturating_add(1),
                pairs.saturating_add(comparisons),
                largest.max(count),
            )
        })
}

/// Emits one count-only event before a potentially expensive fold.
fn log_recovery_workload(families: &[FusedCluster], fingerprints: &[Fingerprint]) {
    if !tracing::enabled!(tracing::Level::INFO) {
        return;
    }
    let (eligible, comparisons, max_members) = recovery_workload(families, fingerprints);
    tracing::info!(
        families = families.len(),
        eligible,
        comparisons,
        max_members,
        "copy recovery workload"
    );
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
        log_recovery_workload(families, fingerprints);
        pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
        let groups = recovery_groups(families, fingerprints, judge, pairs);
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
