//! Applies the shared-subtree rescue over the candidate set
//! ([FUSED-SHARED-SUBTREE], gh #408).
//!
//! Only pairs the fused threshold would otherwise drop — despite
//! corroborating token evidence — are measured: aligning two subtrees for
//! all candidates would repeat the admission-cost mistake
//! [FUSED-CONTENT-GATE] deliberately avoids, and a pair that already
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

use std::{collections::HashMap, hash::BuildHasher, num::NonZeroUsize};

use crate::{
    ast::NormalizedNode,
    buckets::CONTENT_SUPPORT_FLOOR,
    content::pair_content_agreement,
    fingerprint::Fingerprint,
    pair::{crosses_files, rescue_eligible, CandidatePair, SHARED_SUBTREE_MIN_OVERLAP},
    state::FileId,
};

use super::{tally::RescueTally, OverlapMeasurer};

/// Fewest candidate pairs worth sharding at all — below this the thread
/// spawn costs more than the measurements.
///
/// The `None` arm is compile-time dead: the literal is non-zero, and
/// [`std::num::NonZeroUsize::new`] is the only stable way to say so in a
/// `const` without `unsafe`.
const MIN_SHARD_WORK: NonZeroUsize = match NonZeroUsize::new(4_096) {
    Some(floor) => floor,
    None => NonZeroUsize::MIN,
};

/// Candidate pairs per claimed chunk. Small enough that a worker which
/// draws a run of expensive endpoints cannot hold the stage open, large
/// enough that claiming a chunk costs nothing beside measuring it.
const RESCUE_CHUNK_PAIRS: usize = 512;

/// Measures shared-subtree overlap onto every rescue-eligible candidate
/// pair, in parallel when the population justifies it.
///
/// `sources` and `languages` let the pass apply the per-edge content
/// gate ([FUSED-CONTENT-GATE], gh #458) to pairs whose overlap cleared
/// the floor: a Merkle-identical signature alone must not admit a pair
/// whose bodies share nothing.
pub fn apply_shared_subtree_rescue<S: BuildHasher + Sync, L: BuildHasher + Sync>(
    pairs: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) {
    // Content agreement needs every member's tree, resolved once for the
    // whole pass; each measurement then reads its endpoints' collapsed
    // leaves through this index.
    let tree_index: HashMap<FileId, &NormalizedNode> =
        trees.iter().map(|tree| (tree.file_id, tree)).collect();
    tracing::trace!(
        tree_roster = ?trees
            .iter()
            .map(|tree| (tree.file_id, tree.byte_range))
            .collect::<Vec<_>>(),
        "rescue tree roster"
    );
    let workers = crate::shard::worker_count(pairs.len(), MIN_SHARD_WORK);
    if workers <= 1 {
        let mut measurer = OverlapMeasurer::new(trees);
        let mut tally = RescueTally::new();
        for pair in pairs.iter_mut() {
            tally.scan();
            measure_one(
                pair,
                fingerprints,
                &tree_index,
                sources,
                languages,
                &mut measurer,
                &mut tally,
            );
        }
        tally.report_total(measurer.stats());
        return;
    }
    // [PERF-FLUTTER-TODO-RESCUE] Many small chunks handed out on
    // demand, not one contiguous block per worker: a measurement's
    // cost grows with its endpoint size, so contiguous blocks of a
    // sorted candidate list leave one worker running long after the
    // rest have finished (20.8 s against a 5.9 s balanced ideal on the
    // Flutter framework slice). Each worker keeps one measurer across
    // every chunk it claims, so the alignment memos still accumulate.
    let (_measured, shards) = crate::shard::map_chunks(
        pairs.chunks_mut(RESCUE_CHUNK_PAIRS),
        workers,
        || (RescueTally::new(), OverlapMeasurer::new(trees)),
        |(tally, measurer), chunk| {
            measure_chunk(
                chunk,
                fingerprints,
                &tree_index,
                sources,
                languages,
                measurer,
                tally,
            );
        },
    );
    report_shards(&shards);
}

/// Measures one claimed chunk onto the worker's own tally and measurer.
fn measure_chunk<S: BuildHasher, L: BuildHasher>(
    chunk: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
    measurer: &mut OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    for pair in chunk.iter_mut() {
        tally.scan();
        measure_one(
            pair,
            fingerprints,
            tree_index,
            sources,
            languages,
            measurer,
            tally,
        );
    }
}

/// Emits the merged, deterministic totals for a sharded run.
///
/// Every counter merged here is additive, so the totals are the same
/// whichever worker claimed which chunk ([PIPELINE-DETERMINISM]).
fn report_shards(shards: &[(RescueTally, OverlapMeasurer<'_>)]) {
    let Some((first, measurer)) = shards.first() else {
        return;
    };
    let mut merged = first.clone();
    let mut totals = measurer.stats();
    for (tally, measurer) in shards.iter().skip(1) {
        merged.absorb(tally);
        totals = totals.add(measurer.stats());
    }
    merged.report_total(totals);
}

/// Measures one pair when it is eligible, resolvable, and cross-file,
/// recording every gate it passes.
///
/// A pair whose overlap cleared the floor still has to carry its own
/// content through the gate ([FUSED-CONTENT-GATE], gh #458): the
/// overlap floor is a *structural* claim and the token corroboration a
/// *token* claim, and neither knows whether the endpoints' collapsed
/// leaves agree. When the pair's own content agreement falls below
/// [`CONTENT_SUPPORT_FLOOR`] the rescue refuses it — the overlap is
/// left unset so survival drops the pair exactly as if the rescue had
/// never measured it.
fn measure_one<S: BuildHasher, L: BuildHasher>(
    pair: &mut CandidatePair,
    fingerprints: &[Fingerprint],
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
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
    let clears_overlap = pair.shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP;
    let content_agreement = pair_content_agreement(left, right, tree_index, sources, languages);
    let clears_content = !clears_overlap || content_agreement >= CONTENT_SUPPORT_FLOOR;
    tracing::trace!(
        left_file = ?left.file_id,
        right_file = ?right.file_id,
        left_range = ?left.byte_range,
        right_range = ?right.byte_range,
        left_nodes = left.node_count,
        right_nodes = right.node_count,
        overlap = pair.shared_subtree_overlap,
        content_agreement,
        clears_overlap,
        clears_content,
        "rescue per-pair gate"
    );
    if clears_overlap && !clears_content {
        tally.content_gate_rejected();
        pair.shared_subtree_overlap = 0.0;
    }
    tally.measure(clears_overlap && clears_content, measurer.stats());
}

#[cfg(test)]
mod shard_equivalence_tests {
    //! [PERF-FLUTTER-TODO-RESCUE] The sharded rescue must produce the
    //! byte-identical pair outcomes and counters as the serial path:
    //! every measurement is a pure function of the corpus, so sharding
    //! may change which thread computes a value but never the value
    //! (`docs/release-audit.md`, "parallel rescue").

    use std::path::PathBuf;

    use super::{apply_shared_subtree_rescue, measure_chunk, RescueTally, MIN_SHARD_WORK};
    use crate::{
        ast::NormalizedNode,
        fingerprint::Fingerprint,
        lang::LanguageParser,
        pair::{
            CandidatePair, PairScore, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD,
            LSH_ONLY_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_JACCARD,
        },
        state::{FileId, FileRegistry},
    };

    /// One serial shard over `chunk`: the reference a single worker
    /// computes, assembled from the very `measure_chunk` the workers
    /// run so the reference can never drift from the live path.
    fn run_shard<S: std::hash::BuildHasher, L: std::hash::BuildHasher>(
        chunk: &mut [CandidatePair],
        fingerprints: &[Fingerprint],
        trees: &[NormalizedNode],
        sources: &std::collections::HashMap<crate::state::FileId, Vec<u8>, S>,
        languages: &std::collections::HashMap<crate::state::FileId, &'static str, L>,
    ) -> (RescueTally, crate::overlap::MeasureStats) {
        let mut measurer = crate::overlap::OverlapMeasurer::new(trees);
        let mut tally = RescueTally::new();
        measure_chunk(
            chunk,
            fingerprints,
            &tree_index(trees),
            sources,
            languages,
            &mut measurer,
            &mut tally,
        );
        let stats = measurer.stats();
        (tally, stats)
    }

    /// The content-gate index: every tree resolved by file id, exactly
    /// as the live pass builds it.
    fn tree_index(
        trees: &[NormalizedNode],
    ) -> std::collections::HashMap<crate::state::FileId, &NormalizedNode> {
        trees.iter().map(|tree| (tree.file_id, tree)).collect()
    }

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
        node.children
            .iter()
            .map(count_nodes)
            .fold(1, usize::saturating_add)
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

    /// Twice [`MIN_SHARD_WORK`] pairs must measure identically whether
    /// the population runs through the (thread-pooling) entry point or
    /// one `run_shard` over the whole list — the serial reference a
    /// worker computes. Shard boundaries are pinned separately: two
    /// disjoint `run_shard` calls over the halves must reproduce the
    /// whole-list values, and their merged tallies must account for
    /// every pair. A blind rescue (zero measured) fails, it does not
    /// pass vacuously.
    #[test]
    fn sharded_rescue_matches_serial_outcomes() -> Result<(), String> {
        let pair_count = MIN_SHARD_WORK.get().saturating_mul(2);
        let mut registry = FileRegistry::new();
        let left_id = registry.register(PathBuf::from("left.rs"));
        let right_id = registry.register(PathBuf::from("right.rs"));
        let left_source = wide_function(120);
        let right_source = wide_function(121);
        let left = parse(&left_source, left_id)?;
        let right = parse(&right_source, right_id)?;
        let nodes = left.1.node_count;
        let fingerprints = [left.1.clone(), right.1.clone()];
        let trees = [left.0, right.0];
        let sources = std::collections::HashMap::from([
            (left_id, left_source.into_bytes()),
            (right_id, right_source.into_bytes()),
        ]);
        let languages = std::collections::HashMap::from([(left_id, "rust"), (right_id, "rust")]);
        let fixture = || {
            (0..pair_count)
                .map(|_| eligible_pair(nodes))
                .collect::<Vec<_>>()
        };

        // The threaded entry point — whichever core count routes it.
        let mut sharded = fixture();
        apply_shared_subtree_rescue(&mut sharded, &fingerprints, &trees, &sources, &languages);

        // The serial reference: one measurer, one tally, every pair.
        let mut serial = fixture();
        let (serial_tally, serial_stats) =
            run_shard(&mut serial, &fingerprints, &trees, &sources, &languages);

        for (index, (shard_pair, serial_pair)) in sharded.iter().zip(&serial).enumerate() {
            assert!(
                (shard_pair.shared_subtree_overlap - serial_pair.shared_subtree_overlap).abs()
                    < f64::EPSILON,
                "pair {index}: sharded overlap {} must equal serial {}",
                shard_pair.shared_subtree_overlap,
                serial_pair.shared_subtree_overlap
            );
            assert!(
                shard_pair.shared_subtree_overlap > 0.0,
                "pair {index}: the fixture is a real near-duplicate — a rescue that measures \
                 nothing is blind, and overlap was {}",
                shard_pair.shared_subtree_overlap
            );
        }

        // Shard boundaries change nothing: halves measured as separate
        // shards reproduce the whole-list values exactly.
        let mut halved = fixture();
        let midpoint = pair_count / 2;
        let (head, tail) = halved.split_at_mut(midpoint);
        let (head_tally, head_stats) = run_shard(head, &fingerprints, &trees, &sources, &languages);
        let (tail_tally, tail_stats) = run_shard(tail, &fingerprints, &trees, &sources, &languages);
        for (index, (half_pair, serial_pair)) in halved.iter().zip(&serial).enumerate() {
            assert!(
                (half_pair.shared_subtree_overlap - serial_pair.shared_subtree_overlap).abs()
                    < f64::EPSILON,
                "pair {index}: shard-split overlap {} must equal whole-list {}",
                half_pair.shared_subtree_overlap,
                serial_pair.shared_subtree_overlap
            );
        }

        // The merged shard counters account for every pair, exactly as
        // the module contract promises: absorb halves, compare to the
        // whole-list tally, and check the stats fold.
        let mut merged = head_tally;
        merged.absorb(&tail_tally);
        assert_eq!(
            merged.scanned, serial_tally.scanned,
            "merged shard tallies must count every scanned pair"
        );
        assert_eq!(
            merged.eligible, serial_tally.eligible,
            "merged shard tallies must count every eligible pair"
        );
        assert_eq!(
            merged.cross_file, serial_tally.cross_file,
            "merged shard tallies must count every cross-file pair"
        );
        assert_eq!(
            merged.measured, serial_tally.measured,
            "merged shard tallies must count every measured pair"
        );
        let u64_count = u64::try_from(pair_count).unwrap_or(u64::MAX);
        assert_eq!(
            merged.measured, u64_count,
            "every fixture pair is eligible and cross-file: all {pair_count} must be measured, \
             got {}",
            merged.measured
        );
        let folded_stats = head_stats.add(tail_stats);
        assert_eq!(
            folded_stats.alignments, serial_stats.alignments,
            "merged measurement stats must fold to the whole-list stats"
        );
        Ok(())
    }
}
