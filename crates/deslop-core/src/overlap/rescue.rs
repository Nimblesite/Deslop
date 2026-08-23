//! Applies the shared-subtree rescue over the candidate set
//! ([FUSION-SHARED-SUBTREE], gh #408).
//!
//! Only pairs the fused threshold would otherwise drop — despite
//! corroborating token evidence — are measured: aligning two subtrees for
//! all candidates would repeat the admission-cost mistake
//! [FUSION-CONTENT-GATE] deliberately avoids, and a pair that already
//! survives needs no rescue.
//!
//! The pass is measured work over a corpus-scale population, so it runs
//! sharded across the available cores ([PERF-FLUTTER-TODO-RESCUE]): each
//! shard owns a disjoint slice of the candidate list and its own
//! [`OverlapMeasurer`]. Every measurement is a pure function of the
//! corpus, so sharding changes no value — only which thread computes it.
//! Shard results merge in shard order, keeping the reported counters
//! deterministic.
//!
//! Observability is the aggregate gate counters in [`super::tally`] plus
//! fixed-interval progress records, never per-pair events
//! ([PERF-FLUTTER-TODO-OBSERVABILITY]).

use crate::{
    ast::NormalizedNode,
    fingerprint::Fingerprint,
    pair::{crosses_files, rescue_eligible, CandidatePair, SHARED_SUBTREE_MIN_OVERLAP},
};

use super::{tally::RescueTally, OverlapMeasurer};

/// Fewest candidate pairs worth sharding at all — below this the thread
/// spawn costs more than the measurements.
const MIN_SHARD_WORK: usize = 4_096;

/// Measures shared-subtree overlap onto every rescue-eligible candidate
/// pair, in parallel when the population justifies it.
pub fn apply_shared_subtree_rescue(
    pairs: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
) {
    let workers = worker_count(pairs.len());
    if workers <= 1 {
        let mut measurer = OverlapMeasurer::new(trees);
        let mut tally = RescueTally::new();
        for pair in pairs.iter_mut() {
            tally.scan();
            measure_one(pair, fingerprints, &mut measurer, &mut tally);
        }
        tally.report_total(measurer.stats());
        return;
    }
    let shard_size = pairs.len().div_ceil(workers);
    let mut shards: Vec<(RescueTally, super::MeasureStats)> = Vec::with_capacity(workers);
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for chunk in pairs.chunks_mut(shard_size) {
            handles.push(scope.spawn(move || run_shard(chunk, fingerprints, trees)));
        }
        for handle in handles {
            if let Ok(shard) = handle.join() {
                shards.push(shard);
            }
        }
    });
    report_shards(&shards);
}

/// Measures one shard of the candidate list with its own measurer.
fn run_shard(
    chunk: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
) -> (RescueTally, super::MeasureStats) {
    let mut measurer = OverlapMeasurer::new(trees);
    let mut tally = RescueTally::new();
    for pair in chunk.iter_mut() {
        tally.scan();
        measure_one(pair, fingerprints, &mut measurer, &mut tally);
    }
    let stats = measurer.stats();
    (tally, stats)
}

/// Emits the merged, deterministic totals for a sharded run.
fn report_shards(shards: &[(RescueTally, super::MeasureStats)]) {
    let Some((first, stats)) = shards.first() else {
        return;
    };
    let mut merged = first.clone();
    let mut totals = *stats;
    for (tally, stats) in shards.iter().skip(1) {
        merged.absorb(tally);
        totals = totals.add(*stats);
    }
    merged.report_total(totals);
}

/// How many worker threads the rescue uses for `pairs` candidates:
/// the available parallelism, capped so every shard carries real work.
fn worker_count(pairs: usize) -> usize {
    if pairs < MIN_SHARD_WORK {
        return 1;
    }
    let available = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get);
    available.min(pairs / MIN_SHARD_WORK).max(1)
}

/// Measures one pair when it is eligible, resolvable, and cross-file,
/// recording every gate it passes.
fn measure_one(
    pair: &mut CandidatePair,
    fingerprints: &[Fingerprint],
    measurer: &mut OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    if !rescue_eligible(pair) {
        return;
    }
    tally.eligible();
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return;
    };
    if !crosses_files(left, right) {
        return;
    }
    tally.cross_file();
    pair.shared_subtree_overlap = measurer.rescue_overlap(left, right);
    tally.measure(
        pair.shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP,
        measurer.stats(),
    );
}
