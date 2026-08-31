//! Clone cluster materialisation and ranking.
//!
//! Implements [PIPELINE-CLUSTER-EXACT], the fused-clustering output of
//! [FUSED-STRATEGY-BOUNDED-MAX], and the "worst offenders first" scoring of
//! [PIPELINE-RANK-WORST-FIRST]. Consumes [`FusedCluster`]s from
//! [`crate::pair::cluster_by_transitive_closure`] — the two inputs
//! contributing to those clusters are (a) exact structural buckets per
//! [PIPELINE-CLUSTER-EXACT] / Baxter 1998 ([TECH-AST-FINGERPRINT]) and
//! (b) token LSH bucket collisions per `SourcererCC`
//! ([TECH-TOKEN-SOURCERERCC]).

use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
    path::{Path, PathBuf},
};

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    pair::FusedCluster,
    state::FileId,
};

/// Deterministic grouped-signal benchmark workload.
#[cfg(feature = "benchmark")]
pub mod benchmark;
/// The authored declaration an occurrence sits inside
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
mod scope;
/// Cross-cluster subsumption ([PIPELINE-CLUSTER-SUBSUME]).
mod subsume;
use scope::DeclarationScopes;
use subsume::collapse_cross_cluster_overlap;

/// A set of fingerprints that share the same hash, i.e. a detected
/// (structural) clone cluster.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Hex-encoded first 8 bytes of the cluster hash — stable identifier for
    /// reports. Collisions would be astronomical and would still be the same
    /// cluster.
    pub id: String,
    /// Members of the cluster, in discovery order.
    pub members: Vec<Fingerprint>,
    /// Duplicated mass from [RANK-MASS-SUM]. Higher = more code to fix.
    pub mass: u64,
}

/// Minimum number of logical locations required for a reportable
/// duplicate cluster after same-file overlap collapse.
const MIN_REPORTABLE_MEMBERS: usize = 2;

/// Builds ranked clusters from a fused-cluster list produced by
/// [`crate::pair::cluster_by_transitive_closure`]. Each `FusedCluster`
/// references fingerprint indices; this function materialises the full
/// [`Cluster`] so the ranking and rendering stages do not have to know
/// how the cluster was discovered.
///
/// Cluster ids hash the smallest member's digest together with every
/// member's workspace-relative path ([PIPELINE-DETERMINISM], gh #430),
/// so identical fused clusters across runs always report the same id
/// while same-shape findings in different workspaces remain distinct.
/// Inputs accepted by [`build_ranked_fused_clusters`]. Grouped for the
/// same reason [`crate::report::ReportInputs`] exists: the list
/// outgrew the 7-argument function budget, and every field here is
/// borrowed for the whole build so one struct keeps the call sites
/// name-checked.
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
    /// cluster id digest ([PIPELINE-DETERMINISM], gh #430).
    pub file_paths: &'a HashMap<FileId, PathBuf>,
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

/// Materialises every fused cluster that remains reportable.
fn reportable_clusters<L: BuildHasher + Sync>(
    inputs: &ClusterBuildInputs<'_, L>,
    scopes: &DeclarationScopes<'_, impl BuildHasher + Sync>,
) -> Vec<Cluster> {
    inputs
        .fused_clusters
        .iter()
        .filter_map(|fused| build_fused_cluster(inputs, fused, scopes))
        .collect()
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
) -> Option<Cluster> {
    let fingerprints = inputs.fingerprints;
    let occurrence_indices = collapse_overlapping_per_file(fused, fingerprints, scopes);
    if occurrence_indices.len() < MIN_REPORTABLE_MEMBERS {
        return None;
    }
    let members: Vec<Fingerprint> = occurrence_indices
        .iter()
        .filter_map(|index| fingerprints.get(*index).cloned())
        .collect();
    Some(materialize_cluster(members, inputs.file_paths))
}

/// Builds the final reportable cluster from already-filtered members.
fn materialize_cluster(
    members: Vec<Fingerprint>,
    file_paths: &HashMap<FileId, PathBuf>,
) -> Cluster {
    let size = members.len();
    let smallest_nodes = smallest_node_count(&members);
    let mass = duplicate_mass(smallest_nodes, size);
    let id_source = cluster_id_source(&members, file_paths);
    Cluster {
        id: encode_short_id(id_source),
        members,
        mass,
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

/// Selects the deterministic hash source for the public cluster id
/// ([PIPELINE-DETERMINISM], gh #430).
///
/// The smallest member's digest alone names every cluster that shares a
/// normalised subtree: the #107 fixture stamps three unrelated
/// same-shape findings — one per file — with one id, so `cluster-by-id`
/// resolves to whichever is found first and the ranking tie-break stops
/// being a total order. Hashing that digest together with every
/// member's workspace-relative path keeps the id content-derived —
/// identical clusters across runs still agree, because both inputs are
/// functions of workspace state, never of registration history — while
/// distinguishing findings that merely share a shape. `file_paths` must
/// cover every member's file; an uncovered file degrades that member's
/// contribution to empty and reintroduces the shape-only collision the
/// id exists to prevent.
fn cluster_id_source(members: &[Fingerprint], file_paths: &HashMap<FileId, PathBuf>) -> [u8; 32] {
    let Some(smallest) = members.iter().min_by_key(|member| member.hash) else {
        return [0_u8; 32];
    };
    let mut paths: Vec<&Path> = members
        .iter()
        .map(|member| {
            file_paths
                .get(&member.file_id)
                .map_or(Path::new(""), |path| path.as_path())
        })
        .collect();
    paths.sort_unstable();
    let mut hasher = blake3::Hasher::new();
    let _ = hasher.update(&smallest.hash);
    for path in paths {
        let _ = hasher.update(path.as_os_str().as_encoded_bytes());
        let _ = hasher.update(&[0]);
    }
    *hasher.finalize().as_bytes()
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
/// member. Within a run, the member carrying the strongest cross-file
/// discovery edge always beats a wider, more weakly matched one; width
/// only breaks ties between peers (see [`cross_file_edge_strengths`]).
#[must_use]
fn collapse_overlapping_per_file(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    scopes: &DeclarationScopes<'_, impl BuildHasher>,
) -> Vec<usize> {
    let strengths = cross_file_edge_strengths(fused, fingerprints);
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
        out.extend(collapse_overlapping_single_file(bucket, &strengths, scopes));
    }
    // Corpus-index order, not `FileId` order: ids encode registration
    // history (a removed-and-restored file gets a fresh id), while the
    // corpus index follows the path-ordered snapshot, so rendered
    // occurrence order stays byte-identical across edit history
    // ([PIPELINE-DETERMINISM]).
    out.sort_unstable();
    out
}

/// The strongest surviving-edge strength each member holds to a member
/// in a *different file*, from the component's discovery edges.
///
/// This is what the same-file overlap collapse ranks representatives
/// by. A transitive-closure component can chain several views of one
/// region together — an exact sibling-window match and a whole-file
/// root that merely token-matched the window in the other file — and
/// the collapse must keep the occurrence that carries the strongest
/// cross-file evidence (#339). Choosing by width alone let the weakly
/// matched root displace the exact window, drop the cluster's measured
/// `structural` from 1.0 to 0.0, and hand subsumption a reason to
/// delete the only view of the duplicate. Same-file edges deliberately
/// do not count: they describe within-file duplication, which never
/// needs to survive a *same-file* collapse to stay reported.
fn cross_file_edge_strengths(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
) -> HashMap<usize, f64> {
    let mut strengths: HashMap<usize, f64> = HashMap::new();
    for edge in &fused.edges {
        let (Some(left), Some(right)) = (fingerprints.get(edge.left), fingerprints.get(edge.right))
        else {
            continue;
        };
        if left.file_id == right.file_id {
            continue;
        }
        for index in [edge.left, edge.right] {
            let best = strengths.entry(index).or_insert(0.0);
            *best = best.max(edge.strength);
        }
    }
    strengths
}

/// Greedy sweep over one file's occurrences: sort by `(start, -end)`
/// and keep one canonical member per overlapping run. The member with
/// the strongest cross-file discovery edge wins
/// ([`cross_file_edge_strengths`]); between equals the widest byte
/// range (largest physical clone) wins, and equal-width ties keep the
/// first-encountered member so the result stays deterministic across
/// runs.
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
    strengths: &HashMap<usize, f64>,
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
            strength: strengths.get(&index).copied().unwrap_or(0.0),
            declaration: scopes.enclosing(&member),
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
    /// Strongest cross-file discovery edge it carries
    /// ([`cross_file_edge_strengths`]).
    strength: f64,
    /// The authored declaration it sits strictly inside, when the
    /// grammar names one ([`DeclarationScopes::enclosing`]).
    declaration: Option<ByteRange>,
}

impl Occurrence {
    /// True when the two occupy one authored declaration's worth of
    /// scope, so a grade measured over one describes the other's code
    /// too.
    ///
    /// Two occurrences strictly inside the *same* declaration qualify.
    /// So does the asymmetric case: `self` at or above declaration
    /// level (no function production encloses it) against `other`
    /// inside one. A whole-file view holding a function whole, against
    /// an interior window of that same function, is the same
    /// non-comparability seen from one level up — the window scores
    /// higher only by dropping part of what the file says. `ledger_left`
    /// and `ledger_right` reorder every statement of one function: the
    /// file view measured 0.850 and an interior 80..438 window measured
    /// 0.897, and electing the window published `structural 0.730,
    /// token 0.727`, which buckets `loosely_similar` and is hidden —
    /// two fully duplicated files reported as nothing
    /// (`lsh_only_nearmiss_recall`).
    ///
    /// Two occurrences that are both at or above declaration level do
    /// **not** qualify. They span whole declarations, so what one
    /// excludes is other declarations rather than part of one, and the
    /// grades describe comparable code — which is what keeps #339's
    /// exact sibling window ahead of the wider token-matched view.
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
        self.range.start <= other.range.start
            && other.range.end <= self.range.end
            && (self.range.start < other.range.start || other.range.end < self.range.end)
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

    /// Strictly stronger cross-file evidence displaces the incumbent;
    /// between equals, only a strictly wider byte span wins.
    ///
    /// **Inside one declaration the grades are not comparable**
    /// ([PIPELINE-CLUSTER-EXACT-SCOPE], gh #408). A window nested in
    /// the occurrence it competes with scores a higher cross-file edge
    /// exactly to the extent that it drops the statements the two
    /// copies disagree on, so the strength contest inside one authored
    /// declaration elects whichever window omits the most. In
    /// `typescript-type3` the enclosing view of `accumulate`/`aggregate`
    /// measured 0.857 against the 37-node run nested in it at 1.00, and
    /// the 1.00 was the interior `let` + `for` with the extra
    /// `running = running + 2` cut off the end: the pair was published
    /// as Merkle-equal, and the one statement that makes it a Type-3
    /// near-miss disappeared from the report
    /// (`js_ts_signatures::typescript_near_miss_produces_cross_file_structural_cluster`,
    /// `js_ts_clone_buckets::javascript_near_miss_extra_guard_is_a_proven_rename`).
    ///
    /// Across declarations the two spans describe genuinely different
    /// amounts of authored code and the grade is the honest
    /// discriminator, which is what keeps #339 intact: there the
    /// enclosing view is a run of *top-level bindings* whose tail
    /// differs in shape, no function production encloses either view,
    /// and the exact sibling window at 1.00 must still displace the
    /// weakly token-matched wider view at 0.879
    /// (`fsharp_issue_339_sibling_window_rename`). The numbers alone
    /// cannot separate the two — 0.857 must win and 0.879 must lose —
    /// so the scope is what decides, never a threshold.
    fn displaces(&self, candidate: &Occurrence) -> bool {
        if self.representative.encloses(candidate)
            && self.representative.shares_declaration_with(candidate)
        {
            return false;
        }
        match candidate.strength.total_cmp(&self.representative.strength) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => candidate.range.len() > self.representative.range.len(),
        }
    }
}

/// Implements the [RANK-MASS-SUM] formula: duplicated mass only.
///
/// `mass = canonical_node_count × max(visible_occurrences − 1, 0)`
#[must_use]
pub(crate) fn duplicate_mass(canonical_node_count: usize, visible_occurrences: usize) -> u64 {
    let nodes = u64::try_from(canonical_node_count).unwrap_or(u64::MAX);
    let copies = u64::try_from(visible_occurrences.saturating_sub(1)).unwrap_or(u64::MAX);
    nodes.saturating_mul(copies)
}

/// Shortens a full 32-byte hash to an 8-byte hex stable id for reporting.
#[must_use]
pub fn encode_short_id(hash: [u8; 32]) -> String {
    let mut out = String::with_capacity(16);
    for byte in hash.iter().take(8) {
        let high = (*byte >> 4) & 0x0F;
        let low = *byte & 0x0F;
        out.push(hex_nibble(high));
        out.push(hex_nibble(low));
    }
    out
}

/// Maps a 0..=15 nibble to its lowercase hex character.
const fn hex_nibble(nibble: u8) -> char {
    match nibble {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'a',
        11 => 'b',
        12 => 'c',
        13 => 'd',
        14 => 'e',
        _ => 'f',
    }
}
