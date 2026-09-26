//! [FUSED-SHARED-SUBTREE-MEMO] Repeated rescue seeds must not evict unrelated exact work.

use super::*;

const TWO_ALIGNMENTS: u64 = 2;
const ONE_HIT: u64 = 1;
const MIN_FINGERPRINT_NODES: usize = 1;

fn alternate_right(
    tree: &crate::ast::NormalizedNode,
    whole: &crate::fingerprint::Fingerprint,
    left: &crate::fingerprint::Fingerprint,
) -> Result<crate::fingerprint::Fingerprint, String> {
    crate::fingerprint::collect_fingerprints(tree, MIN_FINGERPRINT_NODES)
        .into_iter()
        .find(|fingerprint| fingerprint.hash != whole.hash && fingerprint.hash != left.hash)
        .ok_or_else(|| "a distinct right subtree must exist".to_owned())
}

/// Re-seeding one exact pair at capacity must preserve another pair's
/// cache hit, keeping its alignment count unchanged.
#[test]
fn repeated_rescue_seed_at_capacity_keeps_other_exact_hit() -> Result<(), String> {
    let (left, right) = parse_pair(ACCUMULATE, AGGREGATE_WITH_INSERTION)?;
    let alternate = alternate_right(&right.tree, &right.whole, &left.whole)?;
    let trees = [left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    let rescued = measurer.overlap(&left.whole, &right.whole);
    assert!(rescued >= ADMISSION_FLOOR, "rescue score must be exact");
    let independent = measurer.overlap(&left.whole, &alternate);
    fill_exact_memo(&mut measurer);
    assert_eq!(measurer.exact_results.len(), EXACT_RESULT_MEMO_MAX);
    measurer.remember_rescued_exact(&left.whole, &right.whole, rescued);
    let repeated = measurer.overlap(&left.whole, &alternate);
    assert_eq!(repeated.to_bits(), independent.to_bits());
    assert_eq!(measurer.stats().alignments, TWO_ALIGNMENTS);
    assert_eq!(measurer.stats().exact_hits, ONE_HIT);
    Ok(())
}
