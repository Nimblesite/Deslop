//! [FUSED-CANDIDATE-BUCKET-RECOVERY] The connectivity walk over a large
//! hash group, with the probes of one batch of members against the
//! existing components asked in parallel.
//!
//! A generated parser table puts thousands of same-shape windows that
//! weld to almost nothing into one hash group. Walked one member at a
//! time, every member is probed against every component before it, and
//! one worker holds the whole stage while the others idle. The batched
//! walk reaches the same connectivity: a batch of members is probed
//! against the components that existed before the batch in parallel,
//! then the batch is settled in member order, probing a member against
//! an earlier batch member only when nothing has connected them yet.

use std::sync::{Mutex, PoisonError};

use super::{candidate_exists, CopyWeld, Probed};
use crate::pair::CandidatePair;

/// Members probed against the existing components in one parallel batch.
/// Settling a batch probes its members against one another on one
/// thread, so a batch stays small enough that this never dominates the
/// parallel probing before it.
const PROBE_BATCH_MEMBERS: usize = 16;
/// Probes per claimed chunk inside a batch.
const PROBE_CHUNK_TASKS: usize = 32;

/// Hands out one weld per worker and takes it back after the batch, so
/// a worker's memos survive from batch to batch.
pub(super) struct WeldPool<'a, W> {
    /// Builds a weld for a worker that finds the pool empty.
    open: &'a (dyn Fn() -> W + Sync),
    /// Welds not currently leased.
    idle: Mutex<Vec<W>>,
}

impl<'a, W> WeldPool<'a, W> {
    /// An empty pool over `open`.
    pub(super) fn new(open: &'a (dyn Fn() -> W + Sync)) -> Self {
        Self {
            open,
            idle: Mutex::new(Vec::new()),
        }
    }

    /// An idle weld, or a new one.
    pub(super) fn lease(&self) -> W {
        let idle = self
            .idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop();
        idle.unwrap_or_else(|| (self.open)())
    }

    /// Returns leased welds for the next batch.
    pub(super) fn release(&self, welds: impl IntoIterator<Item = W>) {
        self.idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(welds);
    }
}

/// Disjoint sets over `0..n`, used to settle one batch.
pub(super) struct DisjointSets {
    /// Parent of each node; a root is its own parent.
    parent: Vec<usize>,
}

impl DisjointSets {
    /// `count` singleton sets.
    pub(super) fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
        }
    }

    /// The root of `node`'s set, compressing the path behind it.
    pub(super) fn find(&mut self, node: usize) -> usize {
        let mut root = node;
        while let Some(&parent) = self.parent.get(root).filter(|&&parent| parent != root) {
            root = parent;
        }
        let mut current = node;
        while let Some(parent) = self.parent.get_mut(current) {
            if *parent == root {
                break;
            }
            current = std::mem::replace(parent, root);
        }
        root
    }

    /// Merges the sets of `left` and `right`.
    pub(super) fn union(&mut self, left: usize, right: usize) {
        let (left, right) = (self.find(left), self.find(right));
        if left != right {
            if let Some(parent) = self.parent.get_mut(right) {
                *parent = left;
            }
        }
    }
}

/// The first weld of `member` into `component`, skipping pairs that are
/// already candidates.
fn first_weld(
    component: &[usize],
    member: usize,
    weld: &mut dyn CopyWeld,
    existing: &[CandidatePair],
    probed: &Probed,
) -> Option<CandidatePair> {
    component.iter().copied().find_map(|other| {
        if candidate_exists(existing, other, member) {
            return None;
        }
        probed.count();
        weld.weld(other, member)
    })
}

/// Probes every member of `batch` against every component in parallel;
/// the result for member `i` and component `index` sits at
/// `i * components.len() + index`.
fn probe_batch<W: CopyWeld + Send>(
    batch: &[usize],
    components: &[Vec<usize>],
    pool: &WeldPool<'_, W>,
    workers: usize,
    existing: &[CandidatePair],
    probed: &Probed,
) -> Vec<Option<CandidatePair>> {
    let tasks: Vec<(usize, usize)> = batch
        .iter()
        .flat_map(|&member| (0..components.len()).map(move |index| (member, index)))
        .collect();
    let (results, welds) = crate::shard::map_chunks(
        tasks.chunks(PROBE_CHUNK_TASKS),
        workers,
        || pool.lease(),
        |weld, chunk: &[(usize, usize)]| {
            chunk
                .iter()
                .map(|&(member, index)| {
                    components
                        .get(index)
                        .and_then(|component| first_weld(component, member, weld, existing, probed))
                })
                .collect::<Vec<Option<CandidatePair>>>()
        },
    );
    pool.release(welds);
    results.into_iter().flatten().collect()
}

/// Settles one probed batch in member order: a member joins every
/// component it welded, and is probed against an earlier batch member
/// only while nothing connects them. Returns the new components.
fn settle_batch(
    batch: &[usize],
    components: Vec<Vec<usize>>,
    results: &[Option<CandidatePair>],
    weld: &mut dyn CopyWeld,
    existing: &[CandidatePair],
    probed: &Probed,
    pairs: &mut Vec<CandidatePair>,
) -> Vec<Vec<usize>> {
    let base = components.len();
    let mut sets = DisjointSets::new(base.saturating_add(batch.len()));
    for (position, &member) in batch.iter().enumerate() {
        let node = base.saturating_add(position);
        for index in 0..base {
            if let Some(pair) = results
                .get(position.saturating_mul(base).saturating_add(index))
                .copied()
                .flatten()
            {
                pairs.push(pair);
                sets.union(index, node);
            }
        }
        for (earlier_position, &earlier) in batch.iter().enumerate().take(position) {
            let earlier_node = base.saturating_add(earlier_position);
            if sets.find(earlier_node) == sets.find(node)
                || candidate_exists(existing, earlier, member)
            {
                continue;
            }
            probed.count();
            if let Some(pair) = weld.weld(earlier, member) {
                pairs.push(pair);
                sets.union(earlier_node, node);
            }
        }
    }
    let mut by_root: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, component) in components.into_iter().enumerate() {
        by_root
            .entry(sets.find(index))
            .or_default()
            .extend(component);
    }
    for (position, &member) in batch.iter().enumerate() {
        by_root
            .entry(sets.find(base.saturating_add(position)))
            .or_default()
            .push(member);
    }
    by_root
        .into_values()
        .map(|mut members| {
            members.sort_unstable();
            members
        })
        .collect()
}

/// The welded pairs of one hash group, batch by batch.
pub(super) fn parallel_group_pairs<W: CopyWeld + Send>(
    members: &[usize],
    pool: &WeldPool<'_, W>,
    workers: usize,
    existing: &[CandidatePair],
    probed: &Probed,
) -> Vec<CandidatePair> {
    let mut components: Vec<Vec<usize>> = Vec::new();
    let mut pairs = Vec::new();
    let mut weld = pool.lease();
    for batch in members.chunks(PROBE_BATCH_MEMBERS) {
        let results = probe_batch(batch, &components, pool, workers, existing, probed);
        components = settle_batch(
            batch, components, &results, &mut weld, existing, probed, &mut pairs,
        );
    }
    pool.release([weld]);
    pairs
}
