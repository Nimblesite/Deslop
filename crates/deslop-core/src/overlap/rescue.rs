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
        join_all_pushing(handles, &mut shards);
    });
    report_shards(&shards);
}

/// Joins every worker handle, pushing each shard result in spawn order.
/// A panicked worker is re-raised on the caller (`resume_unwind`), never
/// swallowed: dropping an `Err` join would silently omit that shard's
/// candidates from the analysis while the report still rendered — an
/// incomplete scan masquerading as a complete one
/// (`a_panicked_shard_poisons_the_whole_rescue`).
fn join_all_pushing<T: Send>(
    handles: Vec<std::thread::ScopedJoinHandle<'_, T>>,
    out: &mut Vec<T>,
) {
    for handle in handles {
        match handle.join() {
            Ok(shard) => out.push(shard),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
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

/// A panicked worker must poison the whole rescue rather than vanish:
/// `join_all_pushing` re-raises the payload, so the scan fails loudly
/// instead of reporting totals computed from the surviving shards only.
#[test]
fn a_panicked_shard_poisons_the_whole_rescue() {
    let payloads = [Ok(1_u32), Err("shard exploded"), Ok(2_u32)];
    let result = std::panic::catch_unwind(|| {
        std::thread::scope(|scope| {
            let handles: Vec<_> = payloads
                .into_iter()
                .map(|payload| {
                    scope.spawn(move || match payload {
                        Ok(value) => value,
                        Err(message) => panic!("{message}"),
                    })
                })
                .collect();
            let mut collected = Vec::new();
            join_all_pushing(handles, &mut collected);
            collected
        })
    });
    assert!(
        result.is_err(),
        "the panicked shard's payload must propagate — a swallowed Err join \
         would report an incomplete analysis as complete"
    );
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

#[cfg(test)]
mod shard_equivalence_tests {
    //! [PERF-FLUTTER-TODO-RESCUE] The sharded rescue must produce the
    //! byte-identical pair outcomes and counters as the serial path:
    //! every measurement is a pure function of the corpus, so sharding
    //! may change which thread computes a value but never the value
    //! (`docs/performance-branch-review.md`, "parallel rescue").

    use std::{collections::HashMap, path::PathBuf};

    use super::{apply_shared_subtree_rescue, MIN_SHARD_WORK};
    use crate::{
        ast::NormalizedNode,
        fingerprint::Fingerprint,
        pair::{CandidatePair, PairScore, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD,
            LSH_ONLY_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_JACCARD},
        lang::LanguageParser,
        state::{FileId, FileRegistry},
    };

    /// Parses `source` as Rust and fingerprints its root.
    fn parse(source: &str, file_id: FileId) -> Result<(NormalizedNode, Fingerprint), String> {
        let tree = crate::lang::rust_lang::RustParser
            .parse_and_normalize(source.as_bytes(), file_id)
            .map_err(|error| format!("the Rust fixture must parse: {error}"))?;
        let whole = Fingerprint {
            hash: [0_u8; 32],
            file_id,
            byte_range: tree.byte_range,
            node_count: count_nodes(&tree),
        };
        Ok((tree, whole))
    }

    /// Total nodes in a subtree, including the root.
    fn count_nodes(node: &NormalizedNode) -> usize {
        node.children.iter().map(count_nodes).fold(1, usize::saturating_add)
    }

    /// A wide function past every gate: well over the LSH-only floor
    /// and large enough for real overlap measurement.
    fn wide_function(statements: usize) -> String {
        let body = (0..statements).fold(String::new(), |mut body, index| {
            use std::fmt::Write as _;
            let _written = writeln!(body, "    total = total + {index};");
            body
        });
        format!("fn alpha(seed: u32) -> u32 {{\n    let mut total = seed;\n{body}    total\n}}\n")
    }

    /// A rescue-eligible cross-file pair over two whole-file endpoints.
    fn eligible_pair(nodes: usize) -> CandidatePair {
        CandidatePair {
            left: 0,
            right: 1,
            endpoint_node_counts: (nodes, nodes),
            lsh_only_node_floor: LSH_ONLY_MIN_NODE_COUNT,
            lsh_only_min_jaccard: LSH_ONLY_MIN_JACCARD,
            fused_min_score: FUSED_THRESHOLD,
            shared_subtree_overlap: 0.0,
            score: PairScore {
                structural: 0.0,
                token_jaccard: SHARED_SUBTREE_MIN_JACCARD,
                embedding_cos: 0.0,
            },
        }
    }

    /// Twice [`MIN_SHARD_WORK`] pairs — enough for at least two shards
    /// on every machine — must measure identically to running the same
    /// population serially in halves below the shard threshold. Every
    /// eligible pair mutates identically and no pair is skipped by
    /// either path.
    #[test]
    fn sharded_rescue_matches_serial_outcomes() -> Result<(), String> {
        let pair_count = MIN_SHARD_WORK.saturating_mul(2);
        let mut registry = FileRegistry::new();
        let left_id = registry.register(PathBuf::from("left.rs"));
        let right_id = registry.register(PathBuf::from("right.rs"));
        let left = parse(&wide_function(120), left_id)?;
        let right = parse(&wide_function(121), right_id)?;
        let nodes = left.1.node_count;
        let fingerprints = [left.1.clone(), right.1.clone()];
        let trees = [left.0, right.0];

        let mut sharded: Vec<CandidatePair> = (0..pair_count).map(|_| eligible_pair(nodes)).collect();
        apply_shared_subtree_rescue(&mut sharded, &fingerprints, &trees);

        let serial_halves = MIN_SHARD_WORK / 2;
        let mut serial: Vec<CandidatePair> = (0..pair_count).map(|_| eligible_pair(nodes)).collect();
        let (head, tail) = serial.split_at_mut(serial_halves);
        apply_shared_subtree_rescue(head, &fingerprints, &trees);
        apply_shared_subtree_rescue(tail, &fingerprints, &trees);

        for (index, (shard_pair, serial_pair)) in sharded.iter().zip(&serial).enumerate() {
            assert!(
                (shard_pair.shared_subtree_overlap - serial_pair.shared_subtree_overlap).abs()
                    < f64::EPSILON,
                "pair {index}: sharded overlap {} must equal serial {}",
                shard_pair.shared_subtree_overlap,
                serial_pair.shared_subtree_overlap
            );
        }
        let measured = sharded
            .iter()
            .filter(|pair| pair.shared_subtree_overlap > 0.0)
            .count();
        assert!(
            measured == pair_count || measured == 0,
            "every eligible pair is identical, so measurement must agree across all of them: \
             {measured} of {pair_count} measured"
        );
        let _ = HashMap::<FileId, ()>::new();
        Ok(())
    }
}
