//! [FUSED-SHARED-SUBTREE-ECHO-BOUND] An exact-clone shell with too little
//! room beyond the clone is refused before it pays for tree alignment.

use std::collections::HashMap;

use super::{
    core_verdict,
    shard_equivalence_tests::{eligible_pair, parse, run_shard},
    RescueContext,
};
use crate::{
    ast::NormalizedNode,
    content::ContentMeasurer,
    fingerprint::{subtree_hash, Fingerprint, HashScratch},
    overlap::OverlapMeasurer,
    pair::{CandidatePair, SHARED_SUBTREE_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_OVERLAP},
    registry_fixtures::rust_pair_ids,
    state::FileId,
};

const SHARED_FUNCTION: &str = "fn shared(seed: u32) -> u32 {\n    let mut total = seed;\n    total += 1;\n    total += 2;\n    total += 3;\n    total += 4;\n    total\n}\n";
const SHORT_LEFT: &str = "fn marker() {}\n";
const SHORT_RIGHT: &str = "fn marker() { let value = true; }\n";
const LONG_LEFT: &str = "fn extension(seed: u32) -> u32 {\n    let mut total = seed;\n    total += 5;\n    total += 6;\n    total += 7;\n    total += 8;\n    total += 9;\n    total\n}\n";
const LONG_RIGHT: &str = "fn extension(seed: u32) -> u32 {\n    let mut total = seed;\n    total += 5;\n    total += 6;\n    total += 7;\n    total += 8;\n    total += 9;\n    total += 10;\n    total\n}\n";
const ANCHOR_LEFT: usize = 0;
const ANCHOR_RIGHT: usize = 1;
const CONTAINER_LEFT: usize = 2;
const CONTAINER_RIGHT: usize = 3;
const CONTAINER_PAIR: usize = 1;
const CORE_FLOOR: usize = 1;
const NO_ALIGNMENTS: u64 = 0;
const ONE_ALIGNMENT: u64 = 1;
const NO_RESCUES: u64 = 0;
const ONE_RESCUE: u64 = 1;
const NO_OVERLAP: f64 = 0.0;
const REUSED_ENDPOINTS: usize = 2;

struct Case {
    pairs: [CandidatePair; 2],
    fingerprints: [Fingerprint; 4],
    trees: [NormalizedNode; 2],
    sources: HashMap<FileId, Vec<u8>>,
    languages: HashMap<FileId, &'static str>,
}

impl Case {
    fn new(left_tail: &str, right_tail: &str) -> Result<Self, String> {
        let (left_id, right_id) = rust_pair_ids();
        let left_source = format!("{SHARED_FUNCTION}{left_tail}");
        let right_source = format!("{SHARED_FUNCTION}{right_tail}");
        let left = parsed_parts(&left_source, left_id)?;
        let right = parsed_parts(&right_source, right_id)?;
        let fingerprints = [left.1, right.1, left.2, right.2];
        let pairs = candidate_pairs(&fingerprints);
        Ok(Self {
            pairs,
            fingerprints,
            trees: [left.0, right.0],
            sources: HashMap::from([
                (left_id, left_source.into_bytes()),
                (right_id, right_source.into_bytes()),
            ]),
            languages: HashMap::from([(left_id, "rust"), (right_id, "rust")]),
        })
    }

    fn wraps_within(&self) -> bool {
        let context = RescueContext::new(
            &self.pairs,
            &self.fingerprints,
            &self.trees,
            &self.sources,
            &self.languages,
            CORE_FLOOR,
        );
        context.anchors.wraps_within(
            &self.fingerprints[CONTAINER_LEFT],
            &self.fingerprints[CONTAINER_RIGHT],
            SHARED_SUBTREE_MIN_NODE_COUNT,
        )
    }

    fn measure(&mut self) -> (super::super::tally::RescueTally, super::super::MeasureStats) {
        run_shard(
            &mut self.pairs,
            &self.fingerprints,
            &self.trees,
            &self.sources,
            &self.languages,
        )
    }
}

fn parsed_parts(
    source: &str,
    file_id: FileId,
) -> Result<(NormalizedNode, Fingerprint, Fingerprint), String> {
    let (tree, whole) = parse(source, file_id)?;
    let function = tree
        .children
        .iter()
        .find(|child| child.kind == "function_item")
        .ok_or("the fixture needs its shared function")?;
    let anchor = Fingerprint {
        hash: subtree_hash(function, &mut HashScratch::default()),
        file_id,
        byte_range: function.byte_range,
        node_count: function.subtree_node_count(),
    };
    Ok((tree, anchor, whole))
}

fn candidate_pairs(fingerprints: &[Fingerprint; 4]) -> [CandidatePair; 2] {
    let mut anchor = eligible_pair(fingerprints[ANCHOR_LEFT].node_count);
    anchor.left = ANCHOR_LEFT;
    anchor.right = ANCHOR_RIGHT;
    anchor.score.structural = 1.0;
    let mut container = eligible_pair(fingerprints[CONTAINER_LEFT].node_count);
    container.left = CONTAINER_LEFT;
    container.right = CONTAINER_RIGHT;
    let left = fingerprints[CONTAINER_LEFT].node_count;
    let right = fingerprints[CONTAINER_RIGHT].node_count;
    container.endpoint_node_counts = (left.min(right), left.max(right));
    [anchor, container]
}

// [FUSED-CONTENT-GATE-MEMO] A rescue worker reuses both whole-frontier
// endpoints when the same near-copy receives another core judgement.
#[test]
fn rescue_core_reuses_resolved_content_frontiers() -> Result<(), String> {
    let case = Case::new(LONG_LEFT, LONG_RIGHT)?;
    let context = RescueContext::new(
        &case.pairs,
        &case.fingerprints,
        &case.trees,
        &case.sources,
        &case.languages,
        CORE_FLOOR,
    );
    let left = &case.fingerprints[CONTAINER_LEFT];
    let right = &case.fingerprints[CONTAINER_RIGHT];
    let mut overlap = OverlapMeasurer::new(&case.trees);
    let mut contents = ContentMeasurer::default();
    let first = core_verdict(left, right, &context, &mut overlap, &mut contents);
    let second = core_verdict(left, right, &context, &mut overlap, &mut contents);
    assert!(first.copy, "fixture must keep a copied aligned core");
    assert_eq!(first.copy, second.copy);
    assert_eq!(first.evidence.contradiction, second.evidence.contradiction);
    assert_eq!(
        contents.hits(),
        REUSED_ENDPOINTS,
        "repeat endpoints reuse frontiers"
    );
    Ok(())
}

#[test]
fn exact_clone_shell_skips_alignment() -> Result<(), String> {
    let mut echo = Case::new(SHORT_LEFT, SHORT_RIGHT)?;
    assert!(
        echo.wraps_within(),
        "the exact function claims the tiny shell"
    );
    let (echo_tally, echo_stats) = echo.measure();
    assert_eq!(
        echo_stats.alignments, NO_ALIGNMENTS,
        "an inevitable echo needs no alignment"
    );
    assert_eq!(
        echo_tally.rescued, NO_RESCUES,
        "an echo is not another clone"
    );
    assert_eq!(echo_tally.echo_bound_skipped, ONE_ALIGNMENT);
    assert_eq!(echo_tally.measured, NO_ALIGNMENTS);
    assert_eq!(
        echo.pairs[CONTAINER_PAIR].shared_subtree_overlap.to_bits(),
        NO_OVERLAP.to_bits()
    );
    Ok(())
}

#[test]
fn copied_extension_still_measures() -> Result<(), String> {
    let mut near_copy = Case::new(LONG_LEFT, LONG_RIGHT)?;
    assert!(
        !near_copy.wraps_within(),
        "the edited extension exceeds the echo bound"
    );
    let (copy_tally, copy_stats) = near_copy.measure();
    assert_eq!(
        copy_stats.alignments, ONE_ALIGNMENT,
        "the real near-copy still measures"
    );
    assert!(near_copy.pairs[CONTAINER_PAIR].shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP);
    assert_eq!(copy_tally.rescued, ONE_RESCUE, "the copy stays admitted");
    assert_eq!(copy_tally.echo_bound_skipped, NO_ALIGNMENTS);
    assert_eq!(copy_tally.measured, ONE_ALIGNMENT);
    Ok(())
}
