//! Materialises admitted and informational groups ([PIPELINE-CLUSTER-EXACT]).

use std::{collections::HashMap, hash::BuildHasher, path::PathBuf};

use crate::{
    ast::NormalizedNode, buckets::ClusterKind, fingerprint::Fingerprint, pair::FusedCluster,
    state::FileId,
};

/// The one view each file publishes for an overlapping run
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
mod election;
/// Extends overlapping byte-identical sibling views into their copied run.
mod exact_runs;
/// Public cluster ids ([PIPELINE-DETERMINISM]).
mod identity;
/// Whether a view's width is matched by a copy
/// ([PIPELINE-CLUSTER-EXACT-SCOPE-MATCHED]).
mod matched;
/// The authored declaration an occurrence sits inside
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
pub(crate) mod scope;
/// Cross-cluster subsumption ([PIPELINE-CLUSTER-SUBSUME]).
mod subsume;
use election::collapse_overlapping_per_file;
use exact_runs::coalesce_exact_runs;
pub use identity::encode_short_id;
use identity::{name_clusters, Unnamed};
use scope::DeclarationScopes;
use subsume::collapse_cross_cluster_overlap;

/// Source occurrences with an established clone or informational relation.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Names this finding and no other ([PIPELINE-DETERMINISM]): the
    /// first 8 bytes, hex-encoded, of the digest [`identity`] derives
    /// from the members' shape, their paths, and the cluster's position
    /// rank among the clusters sharing both.
    pub id: String,
    /// Members of the cluster, in discovery order.
    pub members: Vec<Fingerprint>,
    /// Duplicated mass from [RANK-MASS-SUM]. Higher = more code to fix.
    pub mass: u64,
    /// The clone kind: the weakest pair classification between the
    /// canonical member and any other ([CLONE-KIND-FOLD]).
    pub kind: ClusterKind,
    /// The shape family this cluster was admitted out of, for the
    /// report's family-level noise verdict
    /// ([CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY]).
    pub shape_family: Option<usize>,
}

/// Names the clone kind of one reportable cluster from its members'
/// flat-corpus indices, canonical member first ([CLONE-KIND-FOLD]). The
/// session implements it over the same pair measurement `pair/compare`
/// answers with; the build stays ignorant of how a kind is measured.
pub trait ClusterKindJudge: Sync {
    /// The weakest relation between `members[0]` and every other member.
    fn kind(&self, members: &[usize]) -> Option<ClusterKind>;

    /// [CLONE-KIND-FOLD] Separate unrelated members before counting or ranking.
    fn groups(&self, members: &[usize]) -> Vec<(Vec<usize>, ClusterKind)> {
        self.kind(members)
            .map(|kind| (members.to_vec(), kind))
            .into_iter()
            .collect()
    }
}

impl std::fmt::Debug for dyn ClusterKindJudge + '_ {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ClusterKindJudge")
    }
}

/// Minimum number of logical locations required for a reportable
/// duplicate cluster after same-file overlap collapse.
const MIN_REPORTABLE_MEMBERS: usize = 2;

/// Inputs to [`build_ranked_fused_clusters`]; stable ids come from [`identity`].
#[derive(Debug)]
pub struct ClusterBuildInputs<'a, L: BuildHasher> {
    /// Every live fingerprint, flat, in corpus order.
    pub fingerprints: &'a [Fingerprint],
    /// Transitive-closure components to rehydrate.
    pub fused_clusters: &'a [FusedCluster],
    /// Normalised trees the fingerprints walk.
    pub trees: &'a [NormalizedNode],
    /// Source bytes used to verify the complete extent of joined exact windows.
    pub sources: &'a HashMap<FileId, Vec<u8>>,
    /// `FileId → language_id` for declaration-scope matching.
    pub file_languages: &'a HashMap<FileId, &'static str, L>,
    /// `FileId → workspace-relative path` — the second input of the
    /// cluster id digest ([PIPELINE-DETERMINISM]).
    pub file_paths: &'a HashMap<FileId, PathBuf>,
    /// Names each reportable cluster's clone kind ([CLONE-KIND-FOLD]).
    pub kinds: &'a dyn ClusterKindJudge,
}

/// Builds ranked clusters from a fused-cluster list produced by
/// [`crate::pair::cluster_by_transitive_closure`]. Each `FusedCluster`
/// references fingerprint indices; this materialises the full [`Cluster`]
/// so ranking and rendering need not know how the cluster was discovered.
#[must_use]
pub fn build_ranked_fused_clusters<L: BuildHasher + Sync>(
    inputs: &ClusterBuildInputs<'_, L>,
) -> Vec<Cluster> {
    let mut clusters = reportable_clusters(
        inputs,
        &DeclarationScopes::new(inputs.trees, inputs.file_languages),
    );
    let dropped_below_min_members = inputs.fused_clusters.len().saturating_sub(clusters.len());
    clusters.sort_by(|left, right| {
        right
            .mass
            .cmp(&left.mass)
            .then_with(|| left.id.cmp(&right.id))
    });
    let collapsed = collapse_cross_cluster_overlap(clusters, inputs.sources);
    log_ranked_cluster_distribution(
        &collapsed,
        inputs.fused_clusters.len(),
        dropped_below_min_members,
    );
    collapsed
}

/// Materialises every fused cluster that remains reportable, then names
/// them together: an id ranks a cluster among the others sharing its
/// shape and paths, which only the whole set can say
/// ([PIPELINE-DETERMINISM]).
fn reportable_clusters<L: BuildHasher + Sync>(
    inputs: &ClusterBuildInputs<'_, L>,
    scopes: &DeclarationScopes<'_, impl BuildHasher + Sync>,
) -> Vec<Cluster> {
    let drafts: Vec<Unnamed> = inputs
        .fused_clusters
        .iter()
        .flat_map(|fused| build_fused_cluster(inputs, fused, scopes))
        .collect();
    name_clusters(
        coalesce_exact_runs(drafts, inputs.trees, inputs.sources),
        inputs.file_paths,
    )
}

/// Emits the structured GH#45 ranked-cluster distribution summary.
fn log_ranked_cluster_distribution(clusters: &[Cluster], input_total: usize, dropped: usize) {
    let largest_mass = clusters.first().map_or(0, |cluster| cluster.mass);
    tracing::info!(
        total = clusters.len(),
        input_total,
        dropped_below_min_members = dropped,
        largest_mass,
        "ranked clusters built",
    );
}

/// Rehydrates a single `FusedCluster` into a reportable [`Cluster`].
/// Same-file overlap collapse can reduce a fused group to one logical
/// location; those groups are artifacts, not duplicates, and are
/// dropped before ranking.
fn build_fused_cluster<L: BuildHasher + Sync>(
    inputs: &ClusterBuildInputs<'_, L>,
    fused: &FusedCluster,
    scopes: &DeclarationScopes<'_, impl BuildHasher>,
) -> Vec<Unnamed> {
    let fingerprints = inputs.fingerprints;
    let occurrence_indices = collapse_overlapping_per_file(fused, fingerprints, scopes);
    if occurrence_indices.len() < MIN_REPORTABLE_MEMBERS {
        return Vec::new();
    }
    inputs
        .kinds
        .groups(&occurrence_indices)
        .into_iter()
        .map(|(indices, kind)| {
            let members = indices
                .iter()
                .filter_map(|index| fingerprints.get(*index).cloned())
                .collect();
            materialize_cluster(members, kind, fused.shape_family)
        })
        .collect()
}

/// Builds the reportable cluster from already-filtered members, still
/// unnamed: its id waits on the whole set ([`reportable_clusters`]).
fn materialize_cluster(
    members: Vec<Fingerprint>,
    kind: ClusterKind,
    shape_family: Option<usize>,
) -> Unnamed {
    let size = members.len();
    let smallest_nodes = smallest_node_count(&members);
    let mass = duplicate_mass(kind, smallest_nodes, size);
    Unnamed {
        members,
        kind,
        mass,
        shape_family,
    }
}

/// Returns the smallest node count inside a reportable cluster.
fn smallest_node_count(members: &[Fingerprint]) -> usize {
    members
        .iter()
        .map(|member| member.node_count)
        .min()
        .unwrap_or(0)
}

/// Implements the [RANK-MASS-SUM] formula: duplicated mass only.
///
/// `mass = canonical_node_count × max(visible_occurrences − 1, 0)`
#[must_use]
pub(crate) fn duplicate_mass(
    kind: ClusterKind,
    canonical_node_count: usize,
    visible_occurrences: usize,
) -> u64 {
    if !kind.is_clone() {
        return 0;
    }
    let nodes = u64::try_from(canonical_node_count).unwrap_or(u64::MAX);
    let copies = u64::try_from(visible_occurrences.saturating_sub(1)).unwrap_or(u64::MAX);
    nodes.saturating_mul(copies)
}

/// Node floor at which an enclosed family has the standing of copied
/// blocks rather than idiom lines, so a concatenation of it is elected
/// out of the component in favour of the family itself
/// ([PIPELINE-CLUSTER-ELECT-CONTAINER]).
pub(crate) const VERBATIM_OVERTURN_MIN_NODES: usize = 16;
