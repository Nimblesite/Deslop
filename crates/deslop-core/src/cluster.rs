//! Clone cluster materialisation and ranking.
//!
//! Implements [PIPELINE-CLUSTER-EXACT], the fused-clustering output of
//! [FUSION-STRATEGY-MAX-SUM], and the "worst offenders first" scoring of
//! [PIPELINE-RANK-WORST-FIRST]. Consumes [`FusedCluster`]s from
//! [`crate::pair::cluster_by_transitive_closure`] — the two inputs
//! contributing to those clusters are (a) exact structural buckets per
//! [PIPELINE-CLUSTER-EXACT] / Baxter 1998 ([TECH-AST-FINGERPRINT]) and
//! (b) token LSH bucket collisions per `SourcererCC`
//! ([TECH-TOKEN-SOURCERERCC]).

use std::collections::{BTreeMap, HashMap};

use crate::{
    content::ContentEvidence,
    fingerprint::{ranges_overlap, Fingerprint},
    lsh::Signature,
    pair::{FusedCluster, PairScore},
    state::FileId,
};

/// Rendered-truth signal measurement ([FUSION-CLUSTER-SIGNALS]).
mod signals;
/// Cross-cluster subsumption ([PIPELINE-CLUSTER-EXACT], #50).
mod subsume;
use signals::measured_signals;
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
    /// Weight from [PIPELINE-RANK-WORST-FIRST]. Higher = worse offender.
    pub weight: f64,
    /// Measured signal breakdown ([FUSION-CLUSTER-SIGNALS]): the
    /// per-signal mean over every unordered pair of the rendered
    /// members — Merkle-hash equality for `structural`, MinHash
    /// Jaccard for `token_jaccard`, vector cosine for `embedding_cos`.
    /// Never aggregated from the discovery edges that glued the
    /// transitive-closure component together.
    pub signals: PairScore,
    /// Measured raw-content evidence across the members'
    /// normalisation-collapsed leaves ([FUSION-CONTENT-GATE], #331/#336):
    /// pooled byte agreement, Type-2 rename consistency, and literal
    /// dominance ([CLONE-NOISE-LITERAL-TABLE]). Starts
    /// [`ContentEvidence::unmeasured`];
    /// `crate::content::attach_content_evidence` measures it during
    /// render, before bucket routing and the ranking weight read it.
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
/// occurrences ([FUSION-CLUSTER-SIGNALS]) from `signatures` and
/// `embedding_vectors`. Cluster ids are derived from the smallest
/// member's hash so identical fused clusters across runs always report
/// the same id.
#[must_use]
pub fn build_ranked_fused_clusters(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    embedding_vectors: &HashMap<usize, Vec<f32>>,
    fused_clusters: &[FusedCluster],
) -> Vec<Cluster> {
    let mut clusters =
        reportable_clusters(fingerprints, signatures, embedding_vectors, fused_clusters);
    let dropped_below_min_members = fused_clusters.len().saturating_sub(clusters.len());
    clusters.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    let collapsed = collapse_cross_cluster_overlap(clusters);
    log_ranked_cluster_distribution(&collapsed, fused_clusters.len(), dropped_below_min_members);
    collapsed
}

/// Materialises every fused cluster that remains reportable.
fn reportable_clusters(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    embedding_vectors: &HashMap<usize, Vec<f32>>,
    fused_clusters: &[FusedCluster],
) -> Vec<Cluster> {
    fused_clusters
        .iter()
        .filter_map(|fused| {
            build_fused_cluster(fingerprints, signatures, embedding_vectors, fused)
        })
        .collect()
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
fn build_fused_cluster(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    embedding_vectors: &HashMap<usize, Vec<f32>>,
    fused: &FusedCluster,
) -> Option<Cluster> {
    let occurrence_indices = collapsed_member_indices(fingerprints, fused);
    if occurrence_indices.len() < MIN_REPORTABLE_MEMBERS {
        return None;
    }
    let measured = measured_signals(
        &occurrence_indices,
        fingerprints,
        signatures,
        embedding_vectors,
    );
    let members: Vec<Fingerprint> = occurrence_indices
        .iter()
        .filter_map(|index| fingerprints.get(*index).cloned())
        .collect();
    Some(materialize_cluster(members, measured))
}

/// Collects valid fingerprint indices for one fused cluster and
/// collapses overlapping same-file artifacts. Indices (not cloned
/// fingerprints) survive the collapse so signal measurement can reach
/// each rendered occurrence's signature and embedding vector.
fn collapsed_member_indices(fingerprints: &[Fingerprint], fused: &FusedCluster) -> Vec<usize> {
    let raw_members: Vec<usize> = fused
        .members
        .iter()
        .copied()
        .filter(|index| *index < fingerprints.len())
        .collect();
    collapse_overlapping_per_file(raw_members, fingerprints)
}

/// Builds the final reportable cluster from already-filtered members.
fn materialize_cluster(members: Vec<Fingerprint>, signals: PairScore) -> Cluster {
    let size = members.len();
    let smallest_nodes = smallest_node_count(&members);
    let rank_nodes = refactor_potential_node_count(smallest_nodes, signals);
    let spanned_bytes = spanned_byte_count(&members);
    let weight = rank_weight(rank_nodes, size, spanned_bytes);
    let id_source = cluster_id_source(&members);
    Cluster {
        id: encode_short_id(id_source),
        members,
        weight,
        signals,
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

/// Sums physical byte spans for the ranking formula.
fn spanned_byte_count(members: &[Fingerprint]) -> u64 {
    members
        .iter()
        .map(|member| u64::try_from(member.byte_range.len()).unwrap_or(u64::MAX))
        .fold(0_u64, u64::saturating_add)
}

/// Returns the node count used for ranking.
///
/// Low-structural Type-4 clusters often span a large interface-shaped AST
/// region while only a small body fragment is actually refactorable. Keep the
/// rendered `canonical_node_count` unchanged, but rank those clusters by a
/// conservative refactor-potential fraction so exact duplicates stay ahead.
fn refactor_potential_node_count(clone_node_count: usize, signals: PairScore) -> usize {
    if signals.structural < LOW_STRUCTURAL_TYPE4_CEILING
        && signals.embedding_cos >= TYPE4_EMBEDDING_FLOOR
    {
        clone_node_count
            .saturating_mul(LOW_STRUCTURAL_TYPE4_WEIGHT_NUMERATOR)
            .checked_div(LOW_STRUCTURAL_TYPE4_WEIGHT_DENOMINATOR)
            .unwrap_or(clone_node_count)
            .max(1)
    } else {
        clone_node_count
    }
}

/// Selects the deterministic hash source for the public cluster id.
fn cluster_id_source(members: &[Fingerprint]) -> [u8; 32] {
    members
        .iter()
        .min_by_key(|member| member.hash)
        .map_or([0_u8; 32], |member| member.hash)
}

/// Collapses overlapping sibling-window occurrences that live in the
/// same file into a single canonical member per overlapping region.
///
/// Fixes issue #2 ([PIPELINE-CLUSTER-EXACT] sibling-extension runaway):
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
/// member (the widest window, i.e. the largest byte span).
#[must_use]
fn collapse_overlapping_per_file(members: Vec<usize>, fingerprints: &[Fingerprint]) -> Vec<usize> {
    let mut by_file: BTreeMap<FileId, Vec<(usize, Fingerprint)>> = BTreeMap::new();
    for index in members {
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
        out.extend(collapse_overlapping_single_file(bucket));
    }
    out
}

/// Greedy sweep over one file's occurrences: sort by `(start, -end)`
/// and keep one canonical member per overlapping run, choosing the
/// member with the widest byte range (largest physical clone) as the
/// representative. Equal-width ties keep the first-encountered member
/// so the result stays deterministic across runs.
fn collapse_overlapping_single_file(mut bucket: Vec<(usize, Fingerprint)>) -> Vec<usize> {
    bucket.sort_by_key(|(_, member)| {
        (
            member.byte_range.start,
            usize::MAX.saturating_sub(member.byte_range.end),
        )
    });
    let mut kept: Vec<(usize, Fingerprint)> = Vec::with_capacity(bucket.len());
    for candidate in bucket {
        match kept.last_mut() {
            Some(current) if ranges_overlap(&current.1, &candidate.1) => {
                if candidate.1.byte_range.len() > current.1.byte_range.len() {
                    *current = candidate;
                }
            }
            _ => kept.push(candidate),
        }
    }
    kept.into_iter().map(|(index, _)| index).collect()
}

/// Implements the [PIPELINE-RANK-WORST-FIRST] formula.
///
/// `weight = clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)`
///
/// Values are capped at `f64`'s mantissa precision (2^53) before conversion;
/// real-world inputs are orders of magnitude below that, so the clamp only
/// protects against pathological inputs rather than reshaping the formula.
#[must_use]
fn rank_weight(clone_node_count: usize, cluster_size: usize, spanned_bytes: u64) -> f64 {
    let nodes = lossless_f64_from_usize(clone_node_count);
    let size_minus_one = lossless_f64_from_usize(cluster_size.saturating_sub(1));
    let spanned = lossless_f64_from_u64(spanned_bytes.saturating_add(1));
    nodes * size_minus_one * spanned.log2()
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
/// Structural ceiling below which Type-4 span size is treated as low-signal.
const LOW_STRUCTURAL_TYPE4_CEILING: f64 = 0.10;
/// Semantic confidence floor for Type-4 ranking dampening.
const TYPE4_EMBEDDING_FLOOR: f64 = 0.90;
/// Rank low-structural Type-4 clusters at 10% of their AST node span.
const LOW_STRUCTURAL_TYPE4_WEIGHT_NUMERATOR: usize = 1;
/// Denominator for the low-structural Type-4 ranking fraction.
const LOW_STRUCTURAL_TYPE4_WEIGHT_DENOMINATOR: usize = 10;

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
