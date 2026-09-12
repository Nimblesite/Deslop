//! Materialises admitted and informational groups ([PIPELINE-CLUSTER-EXACT]).

use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
    path::PathBuf,
};

use crate::{
    ast::{ByteRange, NormalizedNode},
    buckets::ClusterKind,
    fingerprint::Fingerprint,
    pair::{FusedCluster, SHARED_SUBTREE_MIN_NODE_COUNT},
    state::FileId,
};

/// Public cluster ids ([PIPELINE-DETERMINISM]).
mod identity;
/// The authored declaration an occurrence sits inside
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
pub(crate) mod scope;
/// Cross-cluster subsumption ([PIPELINE-CLUSTER-SUBSUME]).
mod subsume;
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
    let collapsed = collapse_cross_cluster_overlap(clusters);
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
    name_clusters(drafts, inputs.file_paths)
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

/// Collapses overlapping sibling-window occurrences that live in the
/// same file into a single canonical member per overlapping region.
///
/// Fixes ([PIPELINE-CLUSTER-EXACT] sibling-extension runaway):
/// the sibling pass at [`crate::sibling`] emits one fingerprint per
/// contiguous window of widths 2..=8. When a physical clone spans many
/// siblings, several windows cover overlapping byte ranges in the same
/// file and — without this dedup — all survive as distinct members of
/// the cluster. That inflates `members.len()` (used by
/// [`rank_weight`]), the rendered `occurrences` list, and the
/// `cluster-by-id` MCP payload.
///
/// Cross-file distinctness is preserved: two occurrences in different
/// files never collapse, no matter how their byte ranges relate. Two
/// non-overlapping occurrences inside the same file also survive —
/// only a transitively overlapping chain collapses to one canonical
/// member. Within a run the representative is selected by authored
/// scope and width only ([PIPELINE-CLUSTER-EXACT-SCOPE]): an enclosing
/// view inside the same authored declaration stays; otherwise the
/// wider byte range wins, with stable byte-range ordering as the
/// tie-breaker. Pair grades never choose a view — a bridge that should
/// not connect two components must fail pair admission, not be hidden
/// by the collapse.
#[must_use]
fn collapse_overlapping_per_file(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    scopes: &DeclarationScopes<'_, impl BuildHasher>,
) -> Vec<usize> {
    let mut by_file: BTreeMap<FileId, Vec<(usize, Fingerprint)>> = BTreeMap::new();
    for index in fused.members.iter().copied() {
        let Some(member) = fingerprints.get(index) else {
            continue;
        };
        by_file
            .entry(member.file_id)
            .or_default()
            .push((index, member.clone()));
    }
    let mut out: Vec<usize> = Vec::new();
    for bucket in by_file.into_values() {
        out.extend(collapse_overlapping_single_file(bucket, scopes));
    }
    // Corpus-index order, not `FileId` order: ids encode registration
    // history (a removed-and-restored file gets a fresh id), while the
    // corpus index follows the path-ordered snapshot, so rendered
    // occurrence order stays byte-identical across edit history
    // ([PIPELINE-DETERMINISM]).
    out.sort_unstable();
    out
}

/// Greedy sweep over one file's occurrences: sort by `(start, -end)`
/// and keep one canonical member per overlapping run. The representative
/// is the enclosing view when it shares an authored declaration with
/// the candidate; otherwise the widest byte range (largest physical
/// clone) wins, and equal-width ties keep the first-encountered member
/// so the result stays deterministic across runs
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
///
/// The run's frontier is tracked separately from its representative
/// ([PIPELINE-CLUSTER-EXACT]). Overlap is transitive, and the window
/// that bridges two others is often narrower than both: for `[0,100]`,
/// `[90,110]`, `[105,200]` the bridge loses the width contest, so a
/// sweep that tests the next window against the representative alone
/// finds `[105,200]` disjoint and publishes one physical region as two
/// occurrences — inflating the cluster size, the occurrence list and the
/// duplication percentage.
fn collapse_overlapping_single_file(
    mut bucket: Vec<(usize, Fingerprint)>,
    scopes: &DeclarationScopes<'_, impl BuildHasher>,
) -> Vec<usize> {
    bucket.sort_by_key(|(_, member)| {
        (
            member.byte_range.start,
            usize::MAX.saturating_sub(member.byte_range.end),
        )
    });
    let mut runs: Vec<OverlapRun> = Vec::with_capacity(bucket.len());
    for (index, member) in bucket {
        let candidate = Occurrence {
            index,
            range: member.byte_range,
            nodes: member.node_count,
            declaration: scopes.enclosing(&member),
            aligned: scopes.aligned_function(&member).is_some(),
            authored: scopes.is_authored_node(&member),
        };
        match runs.last_mut() {
            Some(run) if run.reaches(candidate.range) => run.absorb(candidate),
            _ => runs.push(OverlapRun::start(candidate)),
        }
    }
    runs.into_iter()
        .map(|run| run.representative.index)
        .collect()
}

/// One same-file occurrence competing to represent an overlapping run.
#[derive(Clone, Copy)]
struct Occurrence {
    /// Fingerprint index, which is what the run finally publishes.
    index: usize,
    /// Byte range this occurrence claims.
    range: ByteRange,
    /// Normalised nodes the occurrence holds.
    nodes: usize,
    /// The authored declaration it sits strictly inside, when the
    /// grammar names one ([`DeclarationScopes::enclosing`]).
    declaration: Option<ByteRange>,
    /// The occurrence is an authored function — its range equals a
    /// function-like declaration's ([`DeclarationScopes::aligned_function`]).
    aligned: bool,
    /// The occurrence is a node the author wrote rather than a window
    /// cut over a run of siblings ([`DeclarationScopes::is_authored_node`]).
    authored: bool,
}

impl Occurrence {
    /// Whether one authored declaration covers both views, including a file around a function.
    fn shares_declaration_with(&self, other: &Self) -> bool {
        match (self.declaration, other.declaration) {
            (Some(mine), Some(theirs)) => mine == theirs,
            (None, Some(_)) => true,
            (_, None) => false,
        }
    }

    /// True when this occurrence covers `other` and is wider on at
    /// least one side.
    fn encloses(&self, other: &Self) -> bool {
        self.range.strictly_encloses(other.range)
    }

    /// True when this occurrence is a window — not a node the author
    /// wrote — that encloses the authored function `function` with
    /// fewer than the rescue node floor of sibling nodes around it: the
    /// function plus scraps ([PIPELINE-CLUSTER-EXACT-SCOPE-SCRAPS]).
    fn is_scraps_around(&self, function: &Self) -> bool {
        !self.authored
            && function.aligned
            && self.encloses(function)
            && self.nodes.saturating_sub(function.nodes) < SHARED_SUBTREE_MIN_NODE_COUNT
    }

    /// True when the two share bytes but neither covers the other, so
    /// each starts or ends inside the other's region.
    fn straddles(&self, other: &Self) -> bool {
        self.range.partially_overlaps(other.range)
    }
}

/// One transitively-overlapping run of same-file occurrences, reduced to
/// the reported location plus the frontier the next window is tested
/// against.
struct OverlapRun {
    /// The best occurrence so far — the one the report publishes for
    /// this run.
    representative: Occurrence,
    /// Highest end byte anywhere in the run, which is not always the
    /// representative's end.
    end: usize,
}

impl OverlapRun {
    /// Opens a run at `first`.
    fn start(first: Occurrence) -> Self {
        Self {
            end: first.range.end,
            representative: first,
        }
    }

    /// Returns `true` when `candidate` overlaps the run. Members arrive
    /// in ascending start order, so reaching past the frontier is the
    /// whole half-open overlap test.
    fn reaches(&self, candidate: ByteRange) -> bool {
        candidate.start < self.end
    }

    /// Extends the run, promoting `candidate` to representative when it
    /// outranks the incumbent ([`Self::displaces`]).
    fn absorb(&mut self, candidate: Occurrence) {
        self.end = self.end.max(candidate.range.end);
        if self.displaces(&candidate) {
            self.representative = candidate;
        }
    }

    /// The wider authored scope displaces the incumbent; equal widths
    /// keep the incumbent ([PIPELINE-CLUSTER-EXACT-SCOPE]).
    ///
    /// **Inside one declaration grades are never compared.**
    /// A window nested in the occurrence it competes with measures a
    /// higher cross-file edge exactly to the extent that it drops the
    /// statements the two copies disagree on, so a grade contest inside
    /// one authored declaration would keep whichever window omits the most.
    /// The enclosing view therefore stays when it shares the authored
    /// declaration with the candidate — `typescript-type3` pins the
    /// enclosing `accumulate`/`aggregate` view winning over the 37-node
    /// interior run that dropped the extra `running = running + 2` and
    /// reported a Merkle-equal pair
    /// (`js_ts_signatures::typescript_near_miss_produces_cross_file_structural_cluster`).
    ///
    /// Between views that do not share a declaration, width alone
    /// decides; a bridge that should not connect two files must fail
    /// pair admission, never be hidden by the collapse.
    /// `fsharp_issue_339_sibling_window_rename` keeps passing because
    /// the wider whole-module view never reaches the component (its
    /// pair to the other module fails admission), so the exact sibling
    /// window remains the only view of the region.
    fn displaces(&self, candidate: &Occurrence) -> bool {
        if let Some(verdict) = self.declaration_verdict(candidate) {
            return verdict;
        }
        if let Some(verdict) = self.scraps_verdict(candidate) {
            return verdict;
        }
        if self.representative.encloses(candidate)
            && self.representative.shares_declaration_with(candidate)
        {
            return false;
        }
        // Authored scope and width only ([PIPELINE-CLUSTER-EXACT-SCOPE]).
        // Pair grades cannot choose a view: a bridge that should not
        // connect must fail pair admission, not be hidden by the
        // collapse. Equal-width ties keep the incumbent, so the run
        // stays deterministic across runs.
        candidate.range.len() > self.representative.range.len()
    }

    /// [PIPELINE-CLUSTER-EXACT-SCOPE-SCRAPS] Between an authored function
    /// and a window that encloses it with fewer than the rescue node
    /// floor of siblings around it, the function is the finding. The
    /// window is the function plus scraps — two field declarations, a
    /// constructor line — and reporting it would publish one method of
    /// a family at a different extent from its siblings. A node the
    /// author wrote — a class body, a whole file — keeps the width rule.
    /// `None` where the rule does not decide.
    fn scraps_verdict(&self, candidate: &Occurrence) -> Option<bool> {
        if self.representative.is_scraps_around(candidate) {
            return Some(true);
        }
        if candidate.is_scraps_around(&self.representative) {
            return Some(false);
        }
        None
    }

    /// [PIPELINE-CLUSTER-EXACT-SCOPE-STRADDLE] Between two views that
    /// straddle each other, the one that *is* an authored declaration is
    /// the finding. The other starts or ends inside a function it does
    /// not contain, so it welds a cut-off body to whatever sits beside
    /// it — a namespace line, a class shell, a sibling member — and no
    /// width can make that region something the author wrote. Views
    /// where one contains the other never reach this rule: a whole file
    /// holding a method whole is still the wider authored scope.
    /// `None` where the rule does not decide.
    fn declaration_verdict(&self, candidate: &Occurrence) -> Option<bool> {
        if !self.representative.straddles(candidate) {
            return None;
        }
        match (self.representative.aligned, candidate.aligned) {
            (true, _) => Some(false),
            (false, true) => Some(true),
            (false, false) => None,
        }
    }
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
