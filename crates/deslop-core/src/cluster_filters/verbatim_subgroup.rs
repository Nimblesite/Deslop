//! [CLONE-NOISE-VERBATIM-SUBGROUP] — partition a noise family off a
//! byte-identical copy instead of erasing both together.
//!
//! Every suppression in [`super`] guards itself with a verbatim escape
//! hatch, and every one of them states the same intent: a byte-for-byte
//! copy is real duplication and survives the filter. That guarantee was
//! written as *"at least two members differ"*, which one unrelated
//! member is enough to satisfy — so a cluster holding a proven copy
//! `A`/`A` **plus** a shape-compatible stranger `C` took the
//! suppression whole and the copy vanished from the report. A duplicate
//! that is never reported is the one defect class no reader can notice.
//!
//! The escape hatch cannot be repaired by loosening the predicate
//! alone: keeping the whole cluster would publish `C` as an occurrence
//! of a copy it is not part of, trading a false negative for a false
//! positive. The family is what has to be separated, so this pass runs
//! before signals are measured: a cluster the noise filters would
//! suppress is replaced by one cluster per byte-identical family it
//! contains, and every member outside those families is dropped. The
//! surviving cluster is then measured, bucketed, ranked and rendered
//! from exactly the occurrences it kept — no signal is inherited from
//! the members that left.
//!
//! Clusters the filters do not suppress are returned untouched, so a
//! consistently-renamed three-way clone stays one three-way clone.
//!
//! [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] The hatch protects a
//! *copy*, and for most filters a copy spans files. A byte-identical
//! family confined to one file is the idiom the filter just recognised,
//! not a paste of it, so it hides with its component.
//!
//! [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL] That holds
//! only where the filter could have seen a second file. A filter whose
//! members must already share one file is exempt: asking it for
//! cross-file proof has one possible answer, and it erased real copies.
//! See [`is_copied_family`].
//!
//! [CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES] "Byte-identical" means
//! the exact source bytes of a member's range — see
//! [`verbatim_families`].

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
    num::NonZeroUsize,
};

use crate::{fingerprint::Fingerprint, pair::FusedCluster, state::FileId};

use super::{
    family::{families_by, restrict},
    is_noise_pattern,
    snippets::ParseCache,
    spans_multiple_files, NoiseFilter,
};

/// Smallest byte-identical family worth keeping, counted in *distinct
/// occurrences*: one lone occurrence is not a duplicate of anything.
const MIN_FAMILY_OCCURRENCES: usize = 2;

/// Replaces every fused cluster the noise filters would suppress but
/// which still contains a byte-identical family with one cluster per
/// such family, dropping the members that belong to none.
///
/// Ordering is preserved: clusters keep their input position and each
/// cluster's families are emitted in first-member order, so the pass is
/// deterministic ([PIPELINE-DETERMINISM]).
///
/// [PERF-FLUTTER-TODO-PAIRS] Every cluster is decided independently of
/// every other, from the corpus alone, so the pass shards across the
/// available cores. It was the run's last wholly serial stage — on the
/// Flutter corpus it held one core for over five minutes while thirteen
/// sat idle — and each worker owns its own [`ParseCache`], so a shared
/// memo is never a shared mutation. Which worker decides which cluster
/// changes nothing: results are written back at their input position
/// and the noise counters are additive.
pub(crate) fn split_noise_verbatim_families<S: BuildHasher + Sync>(
    fused_clusters: &[FusedCluster],
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Vec<FusedCluster> {
    let inputs = SplitInputs {
        fused_clusters,
        fingerprints,
        sources,
        file_languages,
    };
    let slots = decide_every_cluster(&inputs, cache);
    cache.log_noise_totals("noise_verbatim_split");
    slots.into_iter().flatten().flatten().collect()
}

/// One replacement run per input cluster, held at that cluster's input
/// position so the emitted order — and therefore the report — does not
/// depend on which worker decided what.
fn decide_every_cluster<S: BuildHasher + Sync>(
    inputs: &SplitInputs<'_, S>,
    cache: &ParseCache,
) -> Vec<Option<Vec<FusedCluster>>> {
    let order = file_locality_order(inputs.fused_clusters, inputs.fingerprints);
    // One slot per input cluster; a split writes its families as a
    // run, an untouched cluster writes itself, both in input position.
    let mut slots: Vec<Option<Vec<FusedCluster>>> =
        (0..inputs.fused_clusters.len()).map(|_| None).collect();
    if split_workers(order.len()) <= 1 {
        split_serially(&order, inputs, cache, &mut slots);
    } else {
        split_across_workers(&order, inputs, cache, &mut slots);
    }
    slots
}

/// The corpus every cluster's decision reads, gathered so a worker
/// carries one reference instead of four.
#[derive(Clone, Copy)]
struct SplitInputs<'corpus, S: BuildHasher> {
    /// Clusters to decide, in input order.
    fused_clusters: &'corpus [FusedCluster],
    /// Fingerprints every member index resolves against.
    fingerprints: &'corpus [Fingerprint],
    /// Source bytes, for the byte-identical family grouping.
    sources: &'corpus HashMap<FileId, Vec<u8>>,
    /// Language per file, for the noise filters.
    file_languages: &'corpus HashMap<FileId, &'static str, S>,
}

/// Cluster indices in minimum-member-file order.
///
/// [PERF-FLUTTER-TODO-MEMORY] Each file's clusters then arrive together
/// and the bounded [`ParseCache`](super::snippets::ParseCache) tree LRU
/// stays hot. Results are written at each cluster's original position,
/// so the emitted order — and therefore the report — is unchanged.
fn file_locality_order(
    fused_clusters: &[FusedCluster],
    fingerprints: &[Fingerprint],
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..fused_clusters.len()).collect();
    // `Option<FileId>` keys: `None` sorts first, which no real cluster
    // produces (every member resolves), so the order is total.
    order.sort_by_key(|&index| {
        fused_clusters.get(index).and_then(|fused| {
            fused
                .members
                .iter()
                .filter_map(|&member| fingerprints.get(member))
                .map(|found| found.file_id)
                .min()
        })
    });
    order
}

/// Decides every cluster on this thread, against the caller's cache.
fn split_serially<S: BuildHasher>(
    order: &[usize],
    inputs: &SplitInputs<'_, S>,
    cache: &ParseCache,
    slots: &mut [Option<Vec<FusedCluster>>],
) {
    let progress = Progress::new(order.len());
    for &index in order {
        if let Some((position, replacement)) = decide(index, inputs, cache) {
            if let Some(slot) = slots.get_mut(position) {
                *slot = Some(replacement);
            }
        }
        progress.advance(cache);
    }
}

/// Decides every cluster across worker threads, all sharing one cache.
///
/// [PERF-FLUTTER-TODO-MEMORY] The cache is shared rather than cloned per
/// worker, and that is the whole design: a per-worker cache multiplies
/// the parse-tree budget by the core count (a 1.4 GB run became 4.1 GB)
/// and starts every worker cold, so the memoised walks it exists to
/// avoid are recomputed once per worker. Sharing keeps one tree
/// population and one set of memos; the locks are held only around the
/// map operations, never around the walks that fill them.
fn split_across_workers<S: BuildHasher + Sync>(
    order: &[usize],
    inputs: &SplitInputs<'_, S>,
    cache: &ParseCache,
    slots: &mut [Option<Vec<FusedCluster>>],
) {
    let progress = Progress::new(order.len());
    let (claimed, _states) = crate::shard::map_chunks(
        order.chunks(NOISE_CHUNK_CLUSTERS),
        split_workers(order.len()),
        || (),
        |(), chunk| decide_chunk(chunk, inputs, cache, &progress),
    );
    for (position, replacement) in claimed.into_iter().flatten() {
        if let Some(slot) = slots.get_mut(position) {
            *slot = Some(replacement);
        }
    }
}

/// Decides one claimed chunk against the worker's own cache.
fn decide_chunk<S: BuildHasher>(
    chunk: &[usize],
    inputs: &SplitInputs<'_, S>,
    cache: &ParseCache,
    progress: &Progress,
) -> Vec<(usize, Vec<FusedCluster>)> {
    let mut decided = Vec::new();
    for &index in chunk {
        if let Some(outcome) = decide(index, inputs, cache) {
            decided.push(outcome);
        }
        progress.advance(cache);
    }
    decided
}

/// The replacement run for one cluster at its input position: the
/// families it splits into, or the cluster itself unchanged.
fn decide<S: BuildHasher>(
    index: usize,
    inputs: &SplitInputs<'_, S>,
    cache: &ParseCache,
) -> Option<(usize, Vec<FusedCluster>)> {
    let fused = inputs.fused_clusters.get(index)?.clone();
    let replacement = split_one(
        &fused,
        inputs.fingerprints,
        inputs.sources,
        inputs.file_languages,
        cache,
    );
    Some((index, replacement.unwrap_or_else(|| vec![fused])))
}

/// Fewest clusters worth sharding the split at all — below this the
/// thread spawn and the cold per-worker caches cost more than the
/// decisions.
///
/// The `None` arm is compile-time dead: the literal is non-zero, and
/// [`std::num::NonZeroUsize::new`] is the only stable way to say so in a
/// `const` without `unsafe`.
const NOISE_SHARD_MIN_CLUSTERS: NonZeroUsize = match NonZeroUsize::new(512) {
    Some(floor) => floor,
    None => NonZeroUsize::MIN,
};

/// Clusters per claimed chunk. Large enough that a worker's tree LRU
/// stays hot across a run of same-file clusters, small enough that one
/// unlucky draw of wide clusters cannot hold the stage open.
const NOISE_CHUNK_CLUSTERS: usize = 32;

/// How many worker threads decide `clusters` clusters, against this
/// stage's own floor ([`crate::shard::worker_count`]).
fn split_workers(clusters: usize) -> usize {
    crate::shard::worker_count(clusters, NOISE_SHARD_MIN_CLUSTERS)
}

/// Fixed-interval progress for a stage that runs for minutes on a large
/// corpus ([PERF-FLUTTER-TODO-OBSERVABILITY]).
///
/// Shared by every worker so the record counts the stage rather than
/// one thread's share of it. Progress records are diagnostics, not
/// report content: which worker happens to cross an interval boundary
/// does not change a single byte of the output.
struct Progress {
    /// Clusters decided so far, across every worker.
    done: std::sync::atomic::AtomicUsize,
    /// Clusters the stage was handed.
    total: usize,
    /// When the stage started.
    started: std::time::Instant,
}

impl Progress {
    /// Opens a progress record over `total` clusters.
    fn new(total: usize) -> Self {
        Self {
            done: std::sync::atomic::AtomicUsize::new(0),
            total,
            started: std::time::Instant::now(),
        }
    }

    /// Counts one decided cluster, emitting a record on each interval.
    fn advance(&self, cache: &ParseCache) {
        let done = self
            .done
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .saturating_add(1);
        if done % NOISE_PROGRESS_INTERVAL != 0 {
            return;
        }
        tracing::info!(
            stage = "noise_verbatim_split",
            clusters_done = done,
            clusters_total = self.total,
            elapsed_ms = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            "noise split progress"
        );
        cache.log_noise_totals("noise_verbatim_split_progress");
    }
}

/// Clusters between noise-split progress records — bounded, so the
/// per-run log stays small ([PERF-FLUTTER-TODO-OBSERVABILITY]).
const NOISE_PROGRESS_INTERVAL: usize = 5_000;

/// The replacement clusters for one component, or `None` to keep it as
/// it is. `None` covers every cheap case first — a component with no
/// mixed verbatim family needs no re-parse at all — so the noise
/// filters only run on the components a split could actually change
/// ([CLONE-NOISE-REPARSE-CACHE]).
fn split_one<S: BuildHasher>(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Option<Vec<FusedCluster>> {
    let families = splittable_families(fused, fingerprints, sources)?;
    let members = resolved_members(fused, fingerprints)?;
    // [CLONE-NOISE-VERBATIM-SUBGROUP]: a component the noise filters
    // do not suppress is handed on untouched — no split, no member
    // dropped, no panic. The pairwise admission that built the
    // closure decides its fate
    // ([FUSED-STRATEGY-BOUNDED-MAX] step 4).
    let filter = is_noise_pattern(&members, sources, file_languages, cache)?;
    let keepable: Vec<&Vec<usize>> = families
        .iter()
        .filter(|family| is_copied_family(family, fingerprints, filter))
        .collect();
    // No family the hatch protects: the component keeps its own shape and
    // takes the suppression whole, downstream, exactly as it always did.
    // Emitting an empty run here would drop it before the report could
    // count it as hidden.
    (!keepable.is_empty()).then(|| {
        keepable
            .iter()
            .map(|family| restrict(fused, family))
            .collect()
    })
}

/// The byte-identical families in `fused` a split could act on, or
/// `None` when no split could change the component — it holds no family
/// of two or more *distinct occurrences*, or the one it holds already
/// *is* the whole component.
///
/// Answered from the corpus alone, before any re-parse, so the noise
/// filters only ever run on a component a split could actually change
/// ([CLONE-NOISE-REPARSE-CACHE]). Whether a family is a *copy* is a
/// second question, asked in [`is_copied_family`] once the filter that
/// recognised the component is known.
fn splittable_families(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> Option<Vec<Vec<usize>>> {
    let families: Vec<Vec<usize>> = verbatim_families(&fused.members, fingerprints, sources)
        .into_iter()
        .filter(|family| distinct_locations(family, fingerprints) >= MIN_FAMILY_OCCURRENCES)
        .collect();
    let covered: usize = families.iter().map(Vec::len).sum();
    let already_whole = families.len() == 1 && covered == fused.members.len();
    (!families.is_empty() && !already_whole).then_some(families)
}

/// Every member's fingerprint, or `None` when one of them does not
/// resolve: a component judged from fewer members than it holds is
/// judged on evidence it does not have.
fn resolved_members(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
) -> Option<Vec<Fingerprint>> {
    let members: Vec<Fingerprint> = fused
        .members
        .iter()
        .filter_map(|index| fingerprints.get(*index).cloned())
        .collect();
    (members.len() == fused.members.len()).then_some(members)
}

/// Whether `family` is the copy the escape hatch exists to protect
/// ([CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE]): a byte-identical
/// family at two distinct locations which, for most filters, must also
/// span at least two files.
///
/// Byte-identity **across files** is proof of copying — independently
/// authored code does not coincide byte for byte. Byte-identity
/// **within one file** is usually proof of the idiom the noise filter
/// just recognised: the same `monkeypatch.setenv` tail, the same
/// assertion pair against the same fixture, written that way because the
/// pattern mandates it. There the filter's classification is the better
/// evidence, so the family takes the suppression with its component.
///
/// That reasoning needs the filter to have had a *choice*. A filter
/// whose members must already share one file cannot offer cross-file
/// evidence either way, and demanding it deleted real copies: two
/// byte-identical cells of one list literal published when they were the
/// whole literal, and vanished the moment a third, *differing* cell
/// joined — a duplicate erased by the arrival of a line that was never
/// part of it ([CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL],
/// gh #462). Which filter fired therefore decides which question is
/// asked; see [`NoiseFilter::demands_cross_file_copy`].
fn is_copied_family(family: &[usize], fingerprints: &[Fingerprint], filter: NoiseFilter) -> bool {
    distinct_locations(family, fingerprints) >= MIN_FAMILY_OCCURRENCES
        && (!filter.demands_cross_file_copy()
            || spans_multiple_files(
                family
                    .iter()
                    .filter_map(|index| fingerprints.get(*index))
                    .map(|member| member.file_id),
            ))
}

/// How many distinct source locations `family` covers.
///
/// A member is one fingerprinted subtree, and two members can cover the
/// *same* bytes of the *same* file: a block node and the full run of its
/// own children span one range and hash apart, so both are collected and
/// both land in one byte-identical family. That family is one occurrence
/// seen twice, never a copy — sizing it by members read it as a paste,
/// re-parsed a component no split could change, and counted the noise
/// filters as having examined it ([CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES],
/// [PERF-FLUTTER-TODO-OBSERVABILITY]). The duplicate views stay in the
/// family — the same-file overlap collapse selects the authored physical
/// view by scope and width, and it must still see every view
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
fn distinct_locations(family: &[usize], fingerprints: &[Fingerprint]) -> usize {
    family
        .iter()
        .filter_map(|index| fingerprints.get(*index))
        .map(|member| {
            (
                member.file_id,
                member.byte_range.start,
                member.byte_range.end,
            )
        })
        .collect::<HashSet<_>>()
        .len()
}

/// Groups the component's members by the exact source bytes their
/// fingerprint covers ([CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES]) —
/// no normalised comparison and no trivia tolerance, so a family whose
/// members differ in one byte is not a verbatim family at all.
fn verbatim_families(
    member_indices: &[usize],
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
) -> Vec<Vec<usize>> {
    families_by(member_indices, |index| {
        member_text(index, fingerprints, sources)
    })
}

/// The raw source bytes one member's fingerprint covers.
fn member_text<'a>(
    index: usize,
    fingerprints: &[Fingerprint],
    sources: &'a HashMap<FileId, Vec<u8>>,
) -> Option<&'a [u8]> {
    let member = fingerprints.get(index)?;
    sources
        .get(&member.file_id)?
        .get(member.byte_range.start..member.byte_range.end)
}

#[cfg(test)]
mod tests;

/// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] Whether a reported
/// cluster is the byte-identical copy the escape hatch protects from
/// `filter`'s suppression: every occurrence shares exact source bytes,
/// at two or more distinct locations, spanning two files where the
/// filter demands a cross-file copy.
pub(crate) fn escapes_as_copy(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    filter: NoiseFilter,
) -> bool {
    let texts: Vec<&[u8]> = members
        .iter()
        .filter_map(|member| {
            sources
                .get(&member.file_id)?
                .get(member.byte_range.start..member.byte_range.end)
        })
        .collect();
    let Some(first) = texts.first() else {
        return false;
    };
    texts.len() == members.len()
        && texts.iter().all(|text| text == first)
        && members
            .iter()
            .map(|member| {
                (
                    member.file_id,
                    member.byte_range.start,
                    member.byte_range.end,
                )
            })
            .collect::<HashSet<_>>()
            .len()
            >= MIN_FAMILY_OCCURRENCES
        && (!filter.demands_cross_file_copy()
            || spans_multiple_files(members.iter().map(|member| member.file_id)))
}
