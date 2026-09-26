//! [FUSED-SHARED-SUBTREE-HARD-CONTENT] Hard whole-endpoint contradictions
//! are terminal even when an aligned core would otherwise look copied.

use std::collections::HashMap;

use super::{
    core_verdict,
    shard_equivalence_tests::{eligible_pair, parse, run_shard},
    RescueContext,
};
use crate::{
    content::{ContentContradiction, ContentMeasurer},
    overlap::{core::CoreVerdict, tally::RescueTally, OverlapMeasurer},
    registry_fixtures::rust_pair_ids,
    state::FileId,
};

const LEFT_SOURCE: &str = "fn compute(seed: i32) -> i32 { let mut total = seed; total = total + 1; total = total + 2; total = total + 3; total = total + 4; total = total + 5; total = total + 6; total = total + 7; total = total + 8; total }";
const OPERATOR_DRIFT_SOURCE: &str = "fn compute(seed: i32) -> i32 { let mut total = seed; total = total + 1; total = total + 2; total = total + 3; total = total - 4; total = total + 5; total = total + 6; total = total + 7; total = total + 8; total }";
const COPIED_EXTENSION_SOURCE: &str = "fn compute(seed: i32) -> i32 { let mut total = seed; total = total + 1; total = total + 2; total = total + 3; total = total + 4; total = total + 5; total = total + 6; total = total + 7; total = total + 8; total = total + 9; total }";
const SOFT_REFUSAL_SOURCE: &str = "fn collect(base: i32) -> i32 { let mut result = base; result = base + 11; result = result + 12; result = base + 13; result = result + 14; result = base + 15; result = result + 16; result = base + 17; result = result + 18; result = base + 19; result }";
const NO_ALIGNMENTS: u64 = 0;
const ONE_ALIGNMENT: u64 = 1;
const NO_RESCUES: u64 = 0;
const ONE_RESCUE: u64 = 1;
const ONE_SKIP: u64 = 1;
const ZERO_OVERLAP: f64 = 0.0;
const CORE_FLOOR: usize = 1;
const LEFT_INDEX: usize = 0;
const RIGHT_INDEX: usize = 1;

fn measure_pair(
    right_source: &str,
) -> Result<
    (
        crate::pair::CandidatePair,
        RescueTally,
        crate::overlap::MeasureStats,
        CoreVerdict,
    ),
    String,
> {
    let (left_id, right_id) = rust_pair_ids();
    let left = parse(LEFT_SOURCE, left_id)?;
    let right = parse(right_source, right_id)?;
    let mut pair = eligible_pair(left.1.node_count);
    pair.endpoint_node_counts = (
        left.1.node_count.min(right.1.node_count),
        left.1.node_count.max(right.1.node_count),
    );
    let mut pairs = [pair];
    let fingerprints = [left.1, right.1];
    let trees = [left.0, right.0];
    let sources = HashMap::<FileId, Vec<u8>>::from([
        (left_id, LEFT_SOURCE.as_bytes().to_vec()),
        (right_id, right_source.as_bytes().to_vec()),
    ]);
    let languages = HashMap::from([(left_id, "rust"), (right_id, "rust")]);
    let context = RescueContext::new(
        &pairs,
        &fingerprints,
        &trees,
        &sources,
        &languages,
        CORE_FLOOR,
    );
    let mut overlap = OverlapMeasurer::new(&trees);
    let mut contents = ContentMeasurer::default();
    let direct_core = core_verdict(
        &fingerprints[LEFT_INDEX],
        &fingerprints[RIGHT_INDEX],
        &context,
        &mut overlap,
        &mut contents,
    );
    let (tally, stats) = run_shard(&mut pairs, &fingerprints, &trees, &sources, &languages);
    Ok((pairs[LEFT_INDEX], tally, stats, direct_core))
}

#[test]
fn changed_operator_needs_no_alignment_and_stays_refused() -> Result<(), String> {
    let (pair, tally, stats, core) = measure_pair(OPERATOR_DRIFT_SOURCE)?;
    assert!(
        !core.copy,
        "direct core judgement refuses the operator change"
    );
    assert_eq!(
        core.evidence.contradiction,
        ContentContradiction::OperatorSubstitution
    );
    assert_eq!(
        stats.alignments, NO_ALIGNMENTS,
        "a terminal hard contradiction must not pay for tree alignment"
    );
    assert_eq!(
        pair.shared_subtree_overlap.to_bits(),
        ZERO_OVERLAP.to_bits(),
        "the changed computation must not survive rescue"
    );
    assert_eq!(
        tally.rescued, NO_RESCUES,
        "operator drift is not an accepted copy"
    );
    assert_eq!(
        tally.hard_content_skipped, ONE_SKIP,
        "one terminal content refusal is counted"
    );
    Ok(())
}

#[test]
fn copied_extension_still_aligns_and_survives() -> Result<(), String> {
    let (pair, tally, stats, core) = measure_pair(COPIED_EXTENSION_SOURCE)?;
    assert!(
        core.copy,
        "direct core judgement accepts the copied extension"
    );
    assert_eq!(core.evidence.contradiction, ContentContradiction::None);
    assert_eq!(
        stats.alignments, ONE_ALIGNMENT,
        "a true edited copy still reaches exact alignment"
    );
    assert!(
        pair.shared_subtree_overlap >= crate::pair::SHARED_SUBTREE_MIN_OVERLAP,
        "the true copy keeps its measured overlap"
    );
    assert_eq!(
        tally.rescued, ONE_RESCUE,
        "the copied extension remains admitted"
    );
    assert_eq!(
        tally.hard_content_skipped, NO_RESCUES,
        "no copied core may be preflight-refused"
    );
    assert_eq!(tally.core_preflight_skipped, NO_RESCUES);
    Ok(())
}

// [FUSED-SHARED-SUBTREE-CORE-PREFLIGHT] An aligned core that fails the
// canonical content verdict cannot be rescued even by high tree overlap.
#[test]
fn soft_core_refusal_skips_exact_alignment() -> Result<(), String> {
    let (pair, tally, stats, core) = measure_pair(SOFT_REFUSAL_SOURCE)?;
    assert!(
        !core.copy,
        "changed authored values are not a copied core: {core:?}"
    );
    assert_eq!(core.evidence.contradiction, ContentContradiction::None);
    assert_eq!(
        stats.alignments, NO_ALIGNMENTS,
        "refused core avoids the tree DP"
    );
    assert_eq!(
        pair.shared_subtree_overlap.to_bits(),
        ZERO_OVERLAP.to_bits()
    );
    assert_eq!(tally.rescued, NO_RESCUES);
    assert_eq!(tally.core_preflight_skipped, ONE_SKIP);
    Ok(())
}
