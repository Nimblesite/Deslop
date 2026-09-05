//! Applies the shared-subtree rescue over the candidate set
//! ([FUSED-SHARED-SUBTREE], gh #408).
//!
//! Two populations are measured, and no other: pairs the fused threshold
//! would otherwise drop despite corroborating token evidence, and pairs
//! the token axis carries alone, whose content floor under
//! [FUSED-CONTENT-GATE] depends on whether their alignment clears the
//! overlap floor ([`alignment_required`]). Aligning two subtrees for every
//! candidate would repeat the admission-cost mistake the content gate
//! deliberately avoids.
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
    cluster::scope::DeclarationScopes,
    content::pair_content_agreement,
    fingerprint::Fingerprint,
    pair::{
        alignment_required, crosses_files, CandidatePair, ExactFunctionAnchors,
        RESCUE_MIN_CONTENT_AGREEMENT, SHARED_SUBTREE_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_OVERLAP,
    },
    state::FileId,
};

use super::{tally::RescueTally, OverlapMeasurer};

/// Everything a rescue measurement reads besides the pair itself,
/// resolved once per pass and shared read-only by every shard.
pub(super) struct RescueContext<'a, S, L: BuildHasher> {
    /// Every member's normalised tree by file, for content agreement.
    tree_index: HashMap<FileId, &'a NormalizedNode>,
    /// Raw source per file.
    sources: &'a HashMap<FileId, Vec<u8>, S>,
    /// Language per file.
    languages: &'a HashMap<FileId, &'static str, L>,
    /// The exact whole-function clones a container may not merely wrap
    /// ([FUSED-SHARED-SUBTREE-ECHO]).
    anchors: ExactFunctionAnchors,
}

impl<'a, S: BuildHasher, L: BuildHasher> RescueContext<'a, S, L> {
    /// Resolves the pass-wide inputs for `pairs`.
    pub(super) fn new(
        pairs: &[CandidatePair],
        fingerprints: &[Fingerprint],
        trees: &'a [NormalizedNode],
        sources: &'a HashMap<FileId, Vec<u8>, S>,
        languages: &'a HashMap<FileId, &'static str, L>,
    ) -> Self {
        let scopes = DeclarationScopes::new(trees, languages);

        Self {
            tree_index: trees.iter().map(|tree| (tree.file_id, tree)).collect(),
            sources,
            languages,
            anchors: ExactFunctionAnchors::index(pairs, fingerprints, &scopes),
        }
    }
}

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
    // Content agreement needs every member's tree and the echo rule
    // needs every exact function pair, both resolved once for the whole
    // pass; each measurement then reads through this context.
    let context = RescueContext::new(pairs, fingerprints, trees, sources, languages);
    let workers = crate::shard::worker_count(pairs.len(), MIN_SHARD_WORK);
    if workers <= 1 {
        let mut measurer = OverlapMeasurer::new(trees);
        let mut tally = RescueTally::new();
        for pair in pairs.iter_mut() {
            tally.scan();
            measure_one(pair, fingerprints, &context, &mut measurer, &mut tally);
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
            measure_chunk(chunk, fingerprints, &context, measurer, tally);
        },
    );
    report_shards(&shards);
}

/// Measures one claimed chunk onto the worker's own tally and measurer.
fn measure_chunk<S: BuildHasher, L: BuildHasher>(
    chunk: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    context: &RescueContext<'_, S, L>,
    measurer: &mut OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    for pair in chunk.iter_mut() {
        tally.scan();
        measure_one(pair, fingerprints, context, measurer, tally);
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
/// content through the gate ([FUSED-CONTENT-GATE]): the overlap floor
/// is a *structural* claim and the token corroboration a *token*
/// claim, and neither knows whether the endpoints' collapsed leaves
/// agree. When the pair's own content agreement falls below
/// [`RESCUE_MIN_CONTENT_AGREEMENT`] the rescue refuses it — the overlap
/// is left unset so survival drops the pair exactly as if the rescue had
/// never measured it. The same refusal applies to a container that only
/// echoes an exact clone it wraps ([FUSED-SHARED-SUBTREE-ECHO]).
fn measure_one<S: BuildHasher, L: BuildHasher>(
    pair: &mut CandidatePair,
    fingerprints: &[Fingerprint],
    context: &RescueContext<'_, S, L>,
    measurer: &mut OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    if !alignment_required(pair) {
        return;
    }
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return;
    };
    tally.eligible();
    if !crosses_files(left, right) {
        return;
    }
    tally.cross_file();
    pair.shared_subtree_overlap = measurer.rescue_overlap(left, right);
    record_rescue_verdict(pair, left, right, context, measurer, tally);
}

/// Applies content and echo guards to one measured rescue candidate.
fn record_rescue_verdict<S: BuildHasher, L: BuildHasher>(
    pair: &mut CandidatePair,
    left: &Fingerprint,
    right: &Fingerprint,
    context: &RescueContext<'_, S, L>,
    measurer: &OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    let clears_overlap = pair.shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP;
    let clears_content = !clears_overlap
        || pair_content_agreement(
            left,
            right,
            &context.tree_index,
            context.sources,
            context.languages,
        ) >= RESCUE_MIN_CONTENT_AGREEMENT;
    if clears_overlap && !clears_content {
        tally.content_gate_rejected();
        pair.shared_subtree_overlap = 0.0;
    }
    let echoes = clears_overlap && clears_content && is_container_echo(pair, left, right, context);
    if echoes {
        tally.container_echo_rejected();
        pair.shared_subtree_overlap = 0.0;
    }
    tally.measure(
        clears_overlap && clears_content && !echoes,
        measurer.stats(),
    );
}

/// [FUSED-SHARED-SUBTREE-ECHO] Whether the pair's shared mass, beyond
/// the largest exact whole-function clone both endpoints enclose, is too
/// small to rescue on its own.
///
/// Shared mass is the overlap share of the larger endpoint
/// ([FUSED-SHARED-SUBTREE]: `S = shared / max(n)`). A container whose
/// remainder falls below [`SHARED_SUBTREE_MIN_NODE_COUNT`] — the floor
/// every rescued endpoint already has to clear — is not a near-miss the
/// anchor axis missed; it is the class shell or preamble around a clone
/// the anchor axis already proved, and admitting it only hands
/// subsumption a wider, byte-divergent view of that clone.
fn is_container_echo<S: BuildHasher, L: BuildHasher>(
    pair: &CandidatePair,
    left: &Fingerprint,
    right: &Fingerprint,
    context: &RescueContext<'_, S, L>,
) -> bool {
    let Some(claimed) = context.anchors.claimed_nodes(left, right) else {
        return false;
    };
    let larger = left.node_count.max(right.node_count);
    let shared = pair.shared_subtree_overlap * usize_to_f64(larger);
    shared - usize_to_f64(claimed) < usize_to_f64(SHARED_SUBTREE_MIN_NODE_COUNT)
}

/// Node counts as the `f64` the overlap share is measured in.
fn usize_to_f64(nodes: usize) -> f64 {
    u32::try_from(nodes).map_or(f64::MAX, f64::from)
}

#[cfg(test)]
mod shard_equivalence_tests {
    //! [PERF-FLUTTER-TODO-RESCUE] The sharded rescue must produce the
    //! byte-identical pair outcomes and counters as the serial path:
    //! every measurement is a pure function of the corpus, so sharding
    //! may change which thread computes a value but never the value
    //! (`docs/release-audit.md`, "parallel rescue").

    use std::path::PathBuf;

    use super::{
        apply_shared_subtree_rescue, measure_chunk, RescueContext, RescueTally, MIN_SHARD_WORK,
    };
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
        let context = RescueContext::new(chunk, fingerprints, trees, sources, languages);
        measure_chunk(chunk, fingerprints, &context, &mut measurer, &mut tally);
        let stats = measurer.stats();
        (tally, stats)
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
