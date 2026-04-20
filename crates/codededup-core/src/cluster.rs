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

use crate::{
    fingerprint::Fingerprint,
    pair::{FusedCluster, PairScore},
};

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
    /// Per-cluster signal breakdown, when available. Structural-only
    /// exact clusters report `structural = 1.0`, `token_jaccard = 1.0`
    /// because every member shares a Merkle hash and therefore a k-gram
    /// set. Fused clusters carry the mean of pair scores.
    pub signals: PairScore,
}

/// Builds ranked clusters from a fused-cluster list produced by
/// [`crate::pair::cluster_by_transitive_closure`]. Each `FusedCluster`
/// references fingerprint indices; this function materialises the full
/// [`Cluster`] so the ranking and rendering stages do not have to know
/// how the cluster was discovered.
///
/// Signal breakdown comes from `cluster.mean_score`. Cluster ids are
/// derived from the smallest member's hash so identical fused clusters
/// across runs always report the same id.
#[must_use]
pub fn build_ranked_fused_clusters(
    fingerprints: &[Fingerprint],
    fused_clusters: &[FusedCluster],
) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = fused_clusters
        .iter()
        .map(|fused| build_fused_cluster(fingerprints, fused))
        .collect();
    clusters.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    clusters
}

/// Rehydrates a single `FusedCluster` into a [`Cluster`]. The
/// clusterer only emits groups with ≥2 members, and any missing
/// fingerprint index silently drops that slot from the rehydrated
/// cluster — a cluster with zero surviving members is still emitted
/// as an empty slot so the report remains deterministic in that
/// degenerate case.
fn build_fused_cluster(fingerprints: &[Fingerprint], fused: &FusedCluster) -> Cluster {
    let members: Vec<Fingerprint> = fused
        .members
        .iter()
        .filter_map(|index| fingerprints.get(*index).cloned())
        .collect();
    let size = members.len();
    let smallest_nodes = members
        .iter()
        .map(|member| member.node_count)
        .min()
        .unwrap_or(0);
    let spanned_bytes: u64 = members
        .iter()
        .map(|member| u64::try_from(member.byte_range.len()).unwrap_or(u64::MAX))
        .fold(0_u64, u64::saturating_add);
    let weight = rank_weight(smallest_nodes, size, spanned_bytes);
    let id_source = members
        .iter()
        .min_by_key(|member| member.hash)
        .map_or([0_u8; 32], |member| member.hash);
    Cluster {
        id: encode_short_id(id_source),
        members,
        weight,
        signals: fused.mean_score,
    }
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
