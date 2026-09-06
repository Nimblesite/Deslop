//! [PERF-FLUTTER-TODO-RESCUE] The sharded rescue must produce the
//! byte-identical pair outcomes and counters as the serial path:
//! every measurement is a pure function of the corpus, so sharding
//! may change which thread computes a value but never the value
//! (`docs/release-audit.md`, "parallel rescue").

use std::path::PathBuf;

use super::super::{
    rescue::{apply_shared_subtree_rescue, measure_chunk, RescueContext, MIN_SHARD_WORK},
    tally::RescueTally,
};
use crate::{
    ast::NormalizedNode,
    fingerprint::Fingerprint,
    lang::LanguageParser,
    pair::{
        CandidatePair, PairScore, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD, LSH_ONLY_MIN_NODE_COUNT,
        SHARED_SUBTREE_MIN_JACCARD,
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
