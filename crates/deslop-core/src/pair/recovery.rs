//! [FUSED-CANDIDATE-BUCKET-RECOVERY] Copy recovery: the pairs a
//! structural bucket's star hides ([FUSED-CANDIDATE-BUCKET-STAR]).
//!
//! The candidate generator pairs every member of a bucket with the member
//! that sorts first. When that member differs from the copies behind it,
//! the content gate drops its edges and the copies fall apart. Recovery
//! walks each incomplete shape family's same-hash members for
//! connectivity, asking the content gate itself whether two members weld,
//! and emits every welded pair as a candidate. The closure after the gate
//! then connects exactly what pairing every two members would have
//! connected, at about one verdict per copy instead of one per pair
//! (gh #506: a generated parser table of 1,549 same-shape windows asked
//! 10.4M verdicts before the walk).

use std::{
    collections::{BTreeMap, HashSet},
    hash::BuildHasher,
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{CandidatePair, CopyProbe, FusedCluster, PairGate};
use crate::fingerprint::Fingerprint;

#[path = "recovery/parallel.rs"]
mod parallel;
#[cfg(test)]
#[path = "recovery/tests.rs"]
mod tests;

use parallel::{parallel_group_pairs, WeldPool};

/// A connected pair already has its only edge; larger complete components
/// likewise have no missing relation for copy recovery to discover.
const MIN_RECOVERY_MEMBERS: usize = 3;
/// Fewest incomplete families per worker worth sharding recovery for
/// ([`crate::shard::worker_count`]).
const RECOVERY_SHARD_MIN_FAMILIES: NonZeroUsize = match NonZeroUsize::new(64) {
    Some(floor) => floor,
    None => NonZeroUsize::MIN,
};
/// Incomplete families per claimed chunk: one family of thousands of
/// same-hash members must not hold the stage open on one worker.
const RECOVERY_CHUNK_FAMILIES: usize = 8;
/// A pair connects two distinct member indices.
const PAIR_ENDPOINTS: usize = 2;
/// Members of one hash group from which the walk probes each batch of
/// members against the existing components in parallel: below it one
/// worker finishes the group faster than the batches can be dealt.
const PARALLEL_GROUP_MEMBERS: usize = 256;
/// Fewest members of a parallel group per worker
/// ([`crate::shard::worker_count`]).
const PARALLEL_GROUP_MIN_MEMBERS_PER_WORKER: NonZeroUsize = match NonZeroUsize::new(32) {
    Some(floor) => floor,
    None => NonZeroUsize::MIN,
};

/// Counts the welds asked for, across every worker.
#[derive(Default)]
pub(super) struct Probed(AtomicUsize);

impl Probed {
    /// One more weld asked for.
    fn count(&self) {
        let _previous = self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Every weld asked for so far.
    fn total(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

/// Decides whether two Merkle-equal members weld, and builds the
/// candidate they weld through when they do.
pub(crate) trait CopyWeld {
    /// The candidate for `left` and `right`, or `None` when they do not weld.
    fn weld(&mut self, left: usize, right: usize) -> Option<CandidatePair>;
}

impl<F: FnMut(usize, usize) -> Option<CandidatePair>> CopyWeld for F {
    fn weld(&mut self, left: usize, right: usize) -> Option<CandidatePair> {
        self(left, right)
    }
}

/// The scan's weld: the pair is built through the discovery guards and
/// admitted by the same content gate every candidate faces, so recovery
/// adds nothing the gate would not keep and misses nothing it would.
pub(crate) struct GatedCopyWeld<'a, S: BuildHasher, L: BuildHasher> {
    /// Builds the structural pair.
    probe: CopyProbe<'a, S>,
    /// Admits or refuses it.
    gate: PairGate<'a, L>,
    /// The corpus both read.
    fingerprints: &'a [Fingerprint],
}

impl<'a, S: BuildHasher, L: BuildHasher> GatedCopyWeld<'a, S, L> {
    /// One worker's weld over the shared corpus.
    pub(crate) fn new(
        probe: CopyProbe<'a, S>,
        gate: PairGate<'a, L>,
        fingerprints: &'a [Fingerprint],
    ) -> Self {
        Self {
            probe,
            gate,
            fingerprints,
        }
    }
}

impl<S: BuildHasher, L: BuildHasher> CopyWeld for GatedCopyWeld<'_, S, L> {
    fn weld(&mut self, left: usize, right: usize) -> Option<CandidatePair> {
        let pair = self.probe.structural_pair(left, right)?;
        self.gate.admits(&pair, self.fingerprints).then_some(pair)
    }
}

/// Adds to `pairs` every same-hash pair the weld joins inside an
/// incomplete shape family. `open` yields one weld per worker.
pub(crate) fn recover_copy_pairs<W: CopyWeld + Send>(
    pairs: &mut Vec<CandidatePair>,
    families: &[FusedCluster],
    fingerprints: &[Fingerprint],
    open: impl Fn() -> W + Sync,
) {
    log_recovery_workload(families, fingerprints);
    pairs.sort_unstable_by_key(|pair| (pair.left, pair.right));
    let probed = Probed::default();
    let recovered = recovered_pairs(families, fingerprints, &open, pairs, &probed);
    tracing::info!(
        recovered = recovered.len(),
        probed = probed.total(),
        "copy edges recovered"
    );
    pairs.extend(recovered);
}

/// The welded pairs of every incomplete family. Families whose hash
/// groups are all small recover independently, so they are dealt to
/// workers in chunks and the pairs reassembled in family order; a family
/// holding a large group is walked afterwards, one group at a time, with
/// every worker probing its batches ([PIPELINE-DETERMINISM]). Every
/// verdict is the gate's, whichever worker asked for it.
fn recovered_pairs<W: CopyWeld + Send>(
    families: &[FusedCluster],
    fingerprints: &[Fingerprint],
    open: &(impl Fn() -> W + Sync),
    existing: &[CandidatePair],
    probed: &Probed,
) -> Vec<CandidatePair> {
    let (large, small): (Vec<&FusedCluster>, Vec<&FusedCluster>) = families
        .iter()
        .filter(|family| has_missing_pair(family))
        .partition(|family| has_large_group(family, fingerprints));
    let workers = crate::shard::worker_count(small.len(), RECOVERY_SHARD_MIN_FAMILIES);
    let (chunks, _welds) = crate::shard::map_chunks(
        small.chunks(RECOVERY_CHUNK_FAMILIES),
        workers,
        open,
        |weld, chunk: &[&FusedCluster]| {
            chunk
                .iter()
                .flat_map(|family| family_pairs(family, fingerprints, weld, existing, probed))
                .collect::<Vec<CandidatePair>>()
        },
    );
    let pool = WeldPool::new(open);
    chunks
        .into_iter()
        .flatten()
        .chain(
            large.iter().flat_map(|family| {
                large_family_pairs(family, fingerprints, &pool, existing, probed)
            }),
        )
        .collect()
}

/// Whether any hash group of `family` is large enough for the batched walk.
fn has_large_group(family: &FusedCluster, fingerprints: &[Fingerprint]) -> bool {
    recovery_hash_groups(family, fingerprints)
        .iter()
        .any(|members| members.len() >= PARALLEL_GROUP_MEMBERS)
}

/// A family with at least one large hash group: its large groups are
/// walked in parallel batches and its small groups, which recover
/// independently, are dealt to workers in chunks; the pairs come back
/// in hash-group order either way ([PIPELINE-DETERMINISM]).
fn large_family_pairs<W: CopyWeld + Send>(
    family: &FusedCluster,
    fingerprints: &[Fingerprint],
    pool: &WeldPool<'_, W>,
    existing: &[CandidatePair],
    probed: &Probed,
) -> Vec<CandidatePair> {
    let groups = recovery_hash_groups(family, fingerprints);
    let small: Vec<(usize, &[usize])> = groups
        .iter()
        .enumerate()
        .filter(|(_, members)| members.len() < PARALLEL_GROUP_MEMBERS)
        .map(|(index, members)| (index, members.as_slice()))
        .collect();
    let mut by_group: Vec<Vec<CandidatePair>> = vec![Vec::new(); groups.len()];
    for (index, pairs) in small_group_pairs(&small, pool, existing, probed) {
        if let Some(slot) = by_group.get_mut(index) {
            *slot = pairs;
        }
    }
    for (index, members) in groups.iter().enumerate() {
        if members.len() >= PARALLEL_GROUP_MEMBERS {
            let workers =
                crate::shard::worker_count(members.len(), PARALLEL_GROUP_MIN_MEMBERS_PER_WORKER);
            if let Some(slot) = by_group.get_mut(index) {
                *slot = parallel_group_pairs(members, pool, workers, existing, probed);
            }
        }
    }
    by_group.into_iter().flatten().collect()
}

/// The welded pairs of each small hash group, keyed by the group's
/// index, with the groups dealt to workers in chunks.
fn small_group_pairs<W: CopyWeld + Send>(
    groups: &[(usize, &[usize])],
    pool: &WeldPool<'_, W>,
    existing: &[CandidatePair],
    probed: &Probed,
) -> Vec<(usize, Vec<CandidatePair>)> {
    let workers = crate::shard::worker_count(groups.len(), RECOVERY_SHARD_MIN_FAMILIES);
    let (chunks, welds) = crate::shard::map_chunks(
        groups.chunks(RECOVERY_CHUNK_FAMILIES),
        workers,
        || pool.lease(),
        |weld, chunk: &[(usize, &[usize])]| {
            chunk
                .iter()
                .map(|&(index, members)| {
                    (index, connected_hash_pairs(members, weld, existing, probed))
                })
                .collect::<Vec<(usize, Vec<CandidatePair>)>>()
        },
    );
    pool.release(welds);
    chunks.into_iter().flatten().collect()
}

/// Only equal hashes can add structural edges, so a family is walked one
/// hash group at a time.
fn family_pairs(
    family: &FusedCluster,
    fingerprints: &[Fingerprint],
    weld: &mut dyn CopyWeld,
    existing: &[CandidatePair],
    probed: &Probed,
) -> Vec<CandidatePair> {
    recovery_hash_groups(family, fingerprints)
        .iter()
        .flat_map(|members| connected_hash_pairs(members, weld, existing, probed))
        .collect()
}

/// The same-hash groups of a family that could still hold a pair.
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

/// Walks one hash group for connectivity: each member is offered to the
/// members of every component it is not yet in, stopping at that
/// component's first weld, and every weld is emitted. A chain of
/// nontransitive copies still joins link by link, while a group of `n`
/// copies costs about `n` verdicts. An existing candidate is never
/// rebuilt: it keeps its own evidence and faces the gate on its own.
fn connected_hash_pairs(
    members: &[usize],
    weld: &mut dyn CopyWeld,
    existing: &[CandidatePair],
    probed: &Probed,
) -> Vec<CandidatePair> {
    let mut components: Vec<Vec<usize>> = Vec::new();
    let mut pairs = Vec::new();
    for &member in members {
        let mut joined = Vec::new();
        for (index, component) in components.iter().enumerate() {
            let welded = component.iter().copied().find_map(|other| {
                if candidate_exists(existing, other, member) {
                    return None;
                }
                probed.count();
                weld.weld(other, member)
            });
            if let Some(pair) = welded {
                pairs.push(pair);
                joined.push(index);
            }
        }
        join_components(&mut components, &joined, member);
    }
    pairs
}

/// Merges the components `member` welded into, with `member` appended,
/// into the first of them; a member that welded nothing starts its own.
fn join_components(components: &mut Vec<Vec<usize>>, joined: &[usize], member: usize) {
    let Some((&first, rest)) = joined.split_first() else {
        components.push(vec![member]);
        return;
    };
    let mut merged = vec![member];
    for &index in rest.iter().rev() {
        if index < components.len() {
            merged.append(&mut components.remove(index));
        }
    }
    if let Some(target) = components.get_mut(first) {
        target.append(&mut merged);
        target.sort_unstable();
    }
}

/// Whether `left` and `right` are already a candidate (`existing` is
/// sorted by endpoint pair).
fn candidate_exists(existing: &[CandidatePair], left: usize, right: usize) -> bool {
    let key = (left.min(right), left.max(right));
    existing
        .binary_search_by_key(&key, |pair| (pair.left, pair.right))
        .is_ok()
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

/// [PERF-FLUTTER-TODO-OBSERVABILITY] Count the same-hash pairs in reach of
/// recovery without logging source paths or contents; the connectivity walk
/// probes far fewer than this when the members are copies.
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

/// Emits one count-only event before a potentially expensive walk.
fn log_recovery_workload(families: &[FusedCluster], fingerprints: &[Fingerprint]) {
    if !tracing::enabled!(tracing::Level::INFO) {
        return;
    }
    let (eligible, pairs_in_reach, max_members) = recovery_workload(families, fingerprints);
    tracing::info!(
        families = families.len(),
        eligible,
        pairs_in_reach,
        max_members,
        "copy recovery workload"
    );
}
