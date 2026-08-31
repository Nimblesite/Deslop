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
    content::{attach_content_evidence, ContentEvidence},
    fingerprint::Fingerprint,
    lsh::SignatureLookup,
    overlap::OverlapMeasurer,
    pair::{FusedCluster, PairScore},
    state::FileId,
};

/// Deterministic grouped-signal benchmark workload.
#[cfg(feature = "benchmark")]
pub mod benchmark;
/// The authored declaration an occurrence sits inside
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
mod scope;
/// Rendered-truth signal measurement ([FUSED-CLUSTER-SIGNALS]).
mod signals;
/// Cross-cluster subsumption ([PIPELINE-CLUSTER-SUBSUME]).
mod subsume;
use scope::DeclarationScopes;
use signals::measured_signals;
use subsume::collapse_cross_cluster_overlap;
pub(crate) use subsume::VERBATIM_OVERTURN_MIN_NODES;

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
    /// Weight from [PIPELINE-RANK-WORST-FIRST]. Higher = worse offender.
    pub weight: f64,
    /// Measured signal breakdown ([FUSED-CLUSTER-SIGNALS]): one elected
    /// admitted pair's three axes together — Merkle-hash equality /
    /// shared-subtree overlap for `structural`, `MinHash` Jaccard for
    /// `token_jaccard`, vector cosine for `embedding_cos`. Per-axis
    /// maxima are forbidden because they could describe no real pair;
    /// a pair that never cleared admission contributes nothing (gh #458).
    pub signals: PairScore,
    /// The admitted pair — as positions into [`Self::members`], which
    /// is the rendered occurrence order — whose evidence the signals
    /// display ([FUSED-CLUSTER-SIGNALS] gh #458). `None` when no
    /// admitted pair survives the same-file overlap collapse.
    pub signal_source: Option<(usize, usize)>,
    /// The elected pair's measured raw-content evidence from its
    /// normalisation-collapsed leaves ([FUSED-CONTENT-GATE]): byte
    /// agreement, Type-2 rename consistency, and literal dominance
    /// ([CLONE-NOISE-LITERAL-TABLE]). Starts
    /// [`ContentEvidence::unmeasured`];
    /// [`crate::content::attach_content_evidence`] measures it inside
    /// [`build_ranked_fused_clusters`], before cross-cluster
    /// subsumption elects the surviving view and before bucket routing
    /// and the ranking weight read it (#367).
    pub content: ContentEvidence,
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
/// The signal breakdown is measured between each cluster's rendered
/// occurrences ([FUSED-CLUSTER-SIGNALS]) from the inputs' `signatures`
/// and `embedding_vectors`, and each cluster's [`ContentEvidence`] is
/// measured from `trees` and `sources` **before** cross-cluster
/// subsumption elects the surviving view ([FUSED-CONTENT-GATE],
/// [PIPELINE-CLUSTER-SUBSUME]). Cluster ids hash the smallest member's
/// digest together with every member's workspace-relative path
/// ([PIPELINE-DETERMINISM], gh #430), so identical fused clusters across
/// runs always report the same id while same-shape findings in different
/// Inputs accepted by [`build_ranked_fused_clusters`]. Grouped for the
/// same reason [`crate::report::ReportInputs`] exists: the list
/// outgrew the 7-argument function budget, and every field here is
/// borrowed for the whole build so one struct keeps the call sites
/// name-checked.
#[derive(Debug)]
pub struct ClusterBuildInputs<'a, S: BuildHasher, H: BuildHasher, L: BuildHasher> {
    /// Every live fingerprint, flat, in corpus order.
    pub fingerprints: &'a [Fingerprint],
    /// Per-fingerprint `MinHash` signatures, positionally aligned.
    pub signatures: &'a dyn SignatureLookup,
    /// Embedding vectors by corpus index ([FUSED-CLUSTER-SIGNALS]).
    pub embedding_vectors: &'a HashMap<usize, Vec<f32>, S>,
    /// Transitive-closure components to rehydrate.
    pub fused_clusters: &'a [FusedCluster],
    /// Normalised trees the fingerprints walk.
    pub trees: &'a [NormalizedNode],
    /// Source bytes keyed by the file id each fingerprint references.
    pub sources: &'a HashMap<FileId, Vec<u8>, H>,
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
pub fn build_ranked_fused_clusters<
    S: BuildHasher + Sync,
    H: BuildHasher + Sync,
    L: BuildHasher + Sync,
>(
    inputs: &ClusterBuildInputs<'_, S, H, L>,
) -> Vec<Cluster> {
    let mut clusters = reportable_clusters(
        inputs,
        &DeclarationScopes::new(inputs.trees, inputs.file_languages),
    );
    let dropped_below_min_members = inputs.fused_clusters.len().saturating_sub(clusters.len());
    clusters.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    // [FUSED-CONTENT-GATE] before [PIPELINE-CLUSTER-SUBSUME] (#367,
    // #408): subsumption deletes whole views, and the choice must see
    // the same measured content evidence the report will render — a
    // survivor elected on raw geometry cannot be re-elected later.
    attach_content_evidence(
        &mut clusters,
        inputs.trees,
        inputs.sources,
        inputs.file_languages,
    );
    let collapsed = collapse_cross_cluster_overlap(clusters);
    log_ranked_cluster_distribution(
        &collapsed,
        inputs.fused_clusters.len(),
        dropped_below_min_members,
    );
    collapsed
}

/// Fewest fused clusters worth sharding the signal build across
/// threads — below this the spawn cost outweighs the measurement.
const SIGNAL_SHARD_MIN_CLUSTERS: usize = 256;

/// Fused clusters per claimed chunk. Kept small because the cost of a
/// chunk is dominated by its widest cluster: the fewer clusters share a
/// chunk, the less an unlucky draw can hold the stage open.
const SIGNAL_CHUNK_CLUSTERS: usize = 8;

/// Materialises every fused cluster that remains reportable.
///
/// A corpus-scale run pays for this stage in the O(k²) per-cluster pair
/// measurement ([FUSED-CLUSTER-SIGNALS]): one 877-member scaffold
/// cluster measures 384k pairs, most of them full tree alignments.
/// Clusters are independent, every measurement is a pure function of
/// the corpus, and each occurrence belongs to exactly one component —
/// so the build runs sharded over the cluster list with one
/// [`OverlapMeasurer`] per worker, and results merge in input order
/// ([PERF-FLUTTER-TODO-PAIRS]). Threads change who computes a value,
/// never the value: the same pairs feed the same measurer arithmetic.
fn reportable_clusters<S: BuildHasher + Sync, H: BuildHasher + Sync, L: BuildHasher + Sync>(
    inputs: &ClusterBuildInputs<'_, S, H, L>,
    scopes: &DeclarationScopes<'_, impl BuildHasher + Sync>,
) -> Vec<Cluster> {
    let workers = signal_worker_count(inputs.fused_clusters.len());
    if workers <= 1 {
        let mut overlap = OverlapMeasurer::new(inputs.trees);
        let mut spent = BuildSpent::default();
        let clusters = inputs
            .fused_clusters
            .iter()
            .filter_map(|fused| {
                build_fused_cluster(inputs, fused, &mut overlap, scopes, &mut spent)
            })
            .collect();
        log_signal_measurement(overlap.stats(), &spent);
        return clusters;
    }
    // [PERF-FLUTTER-TODO-PAIRS] Many small chunks claimed on demand
    // rather than one contiguous block per worker. A cluster's signal
    // build is quadratic in its member count, so a handful of wide
    // scaffold clusters dominate the stage and a contiguous split
    // strands them on one worker (13.6 s against a 3.9 s balanced
    // ideal on the Flutter framework slice). Each worker keeps one
    // measurer across every chunk it claims, so the alignment memos
    // still accumulate; results reassemble in cluster order, so the
    // report is unchanged ([PIPELINE-DETERMINISM]).
    let (shards, states) = crate::shard::map_chunks(
        inputs.fused_clusters.chunks(SIGNAL_CHUNK_CLUSTERS),
        workers,
        || (OverlapMeasurer::new(inputs.trees), BuildSpent::default()),
        |(overlap, shard_spent), chunk| {
            chunk
                .iter()
                .filter_map(|fused| {
                    build_fused_cluster(inputs, fused, overlap, scopes, shard_spent)
                })
                .collect::<Vec<Cluster>>()
        },
    );
    let mut totals = crate::overlap::MeasureStats::default();
    let mut spent = BuildSpent::default();
    for (overlap, shard_spent) in &states {
        totals = totals.add(overlap.stats());
        spent.absorb(shard_spent);
    }
    log_signal_measurement(totals, &spent);
    let mut clusters = Vec::with_capacity(inputs.fused_clusters.len());
    for shard in shards {
        clusters.extend(shard);
    }
    clusters
}

/// Worker count for the sharded signal build: available parallelism,
/// capped so every shard carries whole clusters worth of work.
fn signal_worker_count(clusters: usize) -> usize {
    if clusters < SIGNAL_SHARD_MIN_CLUSTERS {
        return 1;
    }
    // Capped below full parallelism: each worker carries its own
    // measurer with memo populations, and a corpus-scale run's memory
    // ceiling buys more from one fewer worker than the wall loses
    // ([PERF-FLUTTER-TODO-MEMORY]).
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(SIGNAL_SHARD_MAX_WORKERS)
}

/// Most workers the signal build will use, whatever the core count.
const SIGNAL_SHARD_MAX_WORKERS: usize = 14;

/// Wall time the ranked build spent per substage, accumulated across
/// every cluster so the signal event can attribute the stage
/// ([PIPELINE-OBSERVABILITY-STAGES]).
#[derive(Debug, Default)]
struct BuildSpent {
    /// Same-file overlap collapse.
    collapse: std::time::Duration,
    /// Pairwise signal measurement.
    signals: std::time::Duration,
    /// Cluster materialisation (weight, id, member copies).
    materialize: std::time::Duration,
}

impl BuildSpent {
    /// Folds one shard's substage times into the run total.
    fn absorb(&mut self, other: &Self) {
        self.collapse = self.collapse.saturating_add(other.collapse);
        self.signals = self.signals.saturating_add(other.signals);
        self.materialize = self.materialize.saturating_add(other.materialize);
    }
}

/// Emits the cluster-signal overlap measurement counters and substage
/// wall time, so memo effectiveness and cost attribution across the
/// whole ranked build are readable from one event
/// ([FUSED-SHARED-SUBTREE-MEMO], [PIPELINE-OBSERVABILITY-STAGES]).
fn log_signal_measurement(stats: crate::overlap::MeasureStats, spent: &BuildSpent) {
    tracing::info!(
        alignments = stats.alignments,
        credit_fallbacks = stats.credit_fallbacks,
        hash_equal = stats.hash_equal,
        exact_hits = stats.exact_hits,
        unresolved = stats.unresolved,
        collapse_ms = crate::observe::duration_ms(spent.collapse),
        signals_ms = crate::observe::duration_ms(spent.signals),
        materialize_ms = crate::observe::duration_ms(spent.materialize),
        "cluster signal overlaps measured"
    );
}

/// Emits the structured GH#45 ranked-cluster distribution summary.
fn log_ranked_cluster_distribution(clusters: &[Cluster], input_total: usize, dropped: usize) {
    let (largest_weight, mean_weight) = weight_summary(clusters);
    tracing::info!(
        total = clusters.len(),
        input_total,
        dropped_below_min_members = dropped,
        largest_weight,
        mean_weight,
        "ranked clusters built",
    );
}

/// Returns `(largest_weight, mean_weight)` for a ranked cluster slice.
fn weight_summary(clusters: &[Cluster]) -> (f64, f64) {
    let largest = clusters.first().map_or(0.0, |cluster| cluster.weight);
    let total = clusters.iter().map(|cluster| cluster.weight).sum::<f64>();
    let divisor = u32::try_from(clusters.len()).map_or(f64::from(u32::MAX), f64::from);
    let mean = if clusters.is_empty() {
        0.0
    } else {
        total / divisor
    };
    (largest, mean)
}

/// Rehydrates a single `FusedCluster` into a reportable [`Cluster`].
/// Same-file overlap collapse can reduce a fused group to one logical
/// location; those groups are artifacts, not duplicates, and are
/// dropped before ranking. Signals are measured **after** the collapse
/// so they describe exactly the occurrences the report shows.
fn build_fused_cluster<S: BuildHasher + Sync, H: BuildHasher + Sync, L: BuildHasher + Sync>(
    inputs: &ClusterBuildInputs<'_, S, H, L>,
    fused: &FusedCluster,
    overlap: &mut OverlapMeasurer<'_>,
    scopes: &DeclarationScopes<'_, impl BuildHasher>,
    spent: &mut BuildSpent,
) -> Option<Cluster> {
    let fingerprints = inputs.fingerprints;
    let collapse_started = std::time::Instant::now();
    let occurrence_indices = collapse_overlapping_per_file(fused, fingerprints, scopes);
    spent.collapse = spent.collapse.saturating_add(collapse_started.elapsed());
    if occurrence_indices.len() < MIN_REPORTABLE_MEMBERS {
        return None;
    }
    let signals_started = std::time::Instant::now();
    let admitted_pairs: Vec<(usize, usize)> = fused
        .edges
        .iter()
        .map(|edge| (edge.left, edge.right))
        .collect();
    let measured = measured_signals(
        &occurrence_indices,
        &admitted_pairs,
        fingerprints,
        inputs.signatures,
        inputs.embedding_vectors,
        overlap,
    );
    spent.signals = spent.signals.saturating_add(signals_started.elapsed());
    let materialize_started = std::time::Instant::now();
    let members: Vec<Fingerprint> = occurrence_indices
        .iter()
        .filter_map(|index| fingerprints.get(*index).cloned())
        .collect();
    // The source pair is in corpus-index terms; the report reads it as
    // positions into the rendered occurrence order, which is exactly
    // `members`.
    let signal_source = measured.source_pair.and_then(|(left, right)| {
        let left_position = occurrence_indices.binary_search(&left).ok()?;
        let right_position = occurrence_indices.binary_search(&right).ok()?;
        Some((left_position, right_position))
    });
    let cluster = materialize_cluster(members, measured.score, signal_source, inputs.file_paths);
    spent.materialize = spent
        .materialize
        .saturating_add(materialize_started.elapsed());
    Some(cluster)
}

/// Builds the final reportable cluster from already-filtered members.
fn materialize_cluster(
    members: Vec<Fingerprint>,
    signals: PairScore,
    signal_source: Option<(usize, usize)>,
    file_paths: &HashMap<FileId, PathBuf>,
) -> Cluster {
    let size = members.len();
    let smallest_nodes = smallest_node_count(&members);
    let weight = mass_weight(smallest_nodes, size);
    let id_source = cluster_id_source(&members, file_paths);
    Cluster {
        id: encode_short_id(id_source),
        members,
        weight,
        signals,
        signal_source,
        content: ContentEvidence::unmeasured(),
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
/// `weight = clone_node_count × (cluster_size − 1)`
///
/// The visible re-rank in `report_weight.rs` is the authoritative final
/// weight (it folds the category and structural-only policy multipliers
/// and the `report_hide` visibility); this is the mass a cluster carries
/// before that pass, used to keep the pre-render order stable. No
/// `log2(1 + spanned)` term and no confidence factor survives
/// ([RANK-MASS-SUM], gh #458): a duplicate's extent is the mass to fix,
/// never a confidence-scaled figure.
#[must_use]
fn mass_weight(clone_node_count: usize, cluster_size: usize) -> f64 {
    let nodes = lossless_f64_from_usize(clone_node_count);
    let size_minus_one = lossless_f64_from_usize(cluster_size.saturating_sub(1));
    nodes * size_minus_one
}

/// Converts `usize` to `f64`, clamping to 2^53 (the largest integer that
/// round-trips through `f64`) to keep the cast precision-safe.
fn lossless_f64_from_usize(value: usize) -> f64 {
    u64::try_from(value).map_or(F64_MAX_EXACT_INTEGER, lossless_f64_from_u64)
}

/// Converts `u64` to `f64`, clamping to 2^53.
fn lossless_f64_from_u64(value: u64) -> f64 {
    let clamped = value.min(F64_MAX_EXACT_INTEGER_U64);
    // `clamped` fits in 53 bits — split into two `u32` halves so no cast
    // loses precision.
    let high = u32::try_from(clamped >> 32).unwrap_or(u32::MAX);
    let low = u32::try_from(clamped & u64::from(u32::MAX)).unwrap_or(u32::MAX);
    f64::from(high) * F64_TWO_POW_32 + f64::from(low)
}

/// 2^53: largest integer exactly representable by `f64`.
const F64_MAX_EXACT_INTEGER_U64: u64 = 1_u64 << 53;
/// Same value as [`F64_MAX_EXACT_INTEGER_U64`], pre-converted.
const F64_MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;
/// 2^32 as an `f64`. Used by [`lossless_f64_from_u64`] to reassemble 64-bit
/// values without a direct `u64 as f64` cast.
const F64_TWO_POW_32: f64 = 4_294_967_296.0;

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
