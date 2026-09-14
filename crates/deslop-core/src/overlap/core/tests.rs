//! Unit tests for the aligned core ([FUSED-SHARED-SUBTREE-CORE]): the
//! pairing must preserve order and nesting, reach the leaves a mass
//! floor cannot see, and keep the loop a moved statement crossed.

use super::super::{
    tests::{parse_pair, ACCUMULATE, AGGREGATE_WITH_INSERTION},
    OverlapMeasurer,
};
use crate::{ast::ByteRange, fingerprint::Fingerprint};

/// One core, one verdict, whichever surface asks
/// ([FUSED-SHARED-SUBTREE-CORE]).
mod caller_parity;
/// The core over spans a grammar spells twice ([FUSED-SHARED-SUBTREE-CORE]).
mod wrapped_spans;

/// `ACCUMULATE` with `carried` declared after the loop instead of before
/// it — one statement moved across the loop, nothing else changed.
const ACCUMULATE_REORDERED: &str = "\
fn accumulate(bound: u32) -> u32 {
    if bound == 0 {
        return 0;
    }
    let mut running = 0;
    for step in 0..bound {
        running = running + step;
    }
    let carried = 1;
    running + carried
}
";

/// `ACCUMULATE_REORDERED`'s statements in their original order.
const ACCUMULATE_CARRIED: &str = "\
fn accumulate(bound: u32) -> u32 {
    if bound == 0 {
        return 0;
    }
    let mut running = 0;
    let carried = 1;
    for step in 0..bound {
        running = running + step;
    }
    running + carried
}
";

/// The loop both orderings share, verbatim.
const SHARED_LOOP: &str = "for step in 0..bound {\n        running = running + step;\n    }";

// [FUSED-SHARED-SUBTREE-CORE] The aligned core is the code two endpoints
// provably share: the spans ascend and never overlap on either side, the
// whole `if` the insertion never touched is one pair, and the pairing
// reaches the tail expression — one identifier wide — that no mass-floor
// walk could see, which is where a renamed near-miss keeps its
// corroboration.
#[test]
fn the_aligned_core_pairs_the_shared_subtrees_in_order() -> Result<(), String> {
    let (left, right) = parse_pair(ACCUMULATE, AGGREGATE_WITH_INSERTION)?;
    let trees = [left.tree, right.tree];
    let core = OverlapMeasurer::new(&trees).aligned_core(&left.whole, &right.whole);
    assert!(
        !core.is_empty(),
        "a one-statement insertion leaves whole statements Merkle-equal, so the core \
         cannot be empty"
    );
    assert_core_is_ordered(&core);
    assert_core_pairs(
        &core,
        span_of(ACCUMULATE, "if bound == 0 {\n        return 0;\n    }"),
        span_of(
            AGGREGATE_WITH_INSERTION,
            "if limit == 0 {\n        return 0;\n    }",
        ),
    );
    assert_core_pairs(
        &core,
        tail_of(ACCUMULATE, "running"),
        tail_of(AGGREGATE_WITH_INSERTION, "total"),
    );
    Ok(())
}

// [FUSED-SHARED-SUBTREE-CORE] A statement moved across a loop is paired
// with itself, and the loop it crossed stays in the core: a pairing that
// advanced a cursor past the loop to claim the moved statement lost the
// largest shared block of the reordered `ledger` pair and read a verbatim
// reorder as 0.6 agreement.
#[test]
fn the_aligned_core_keeps_the_loop_a_moved_statement_crossed() -> Result<(), String> {
    let (left, right) = parse_pair(ACCUMULATE_CARRIED, ACCUMULATE_REORDERED)?;
    let trees = [left.tree, right.tree];
    let core = OverlapMeasurer::new(&trees).aligned_core(&left.whole, &right.whole);
    assert_core_is_ordered(&core);
    assert_core_pairs(
        &core,
        span_of(ACCUMULATE_CARRIED, SHARED_LOOP),
        span_of(ACCUMULATE_REORDERED, SHARED_LOOP),
    );
    Ok(())
}

/// The spans ascend without overlapping on either side — the order a
/// Tai mapping needs — and every pair is one normalised shape.
fn assert_core_is_ordered(core: &[(Fingerprint, Fingerprint)]) {
    let mut left_end = 0_usize;
    let mut right_end = 0_usize;
    for (span, partner) in core {
        assert_eq!(
            span.hash, partner.hash,
            "every core pair is one normalised shape"
        );
        assert!(
            span.byte_range.start >= left_end && partner.byte_range.start >= right_end,
            "core spans ascend and never overlap on either side: {span:?} against {partner:?}"
        );
        left_end = span.byte_range.end;
        right_end = partner.byte_range.end;
    }
}

/// The core holds exactly the pair `(left, right)`.
fn assert_core_pairs(core: &[(Fingerprint, Fingerprint)], left: ByteRange, right: ByteRange) {
    assert!(
        core.iter()
            .any(|(span, partner)| span.byte_range == left && partner.byte_range == right),
        "the core must pair {left:?} with {right:?}; it holds {:?}",
        core.iter()
            .map(|(span, partner)| (span.byte_range, partner.byte_range))
            .collect::<Vec<_>>()
    );
}

/// The byte range of `needle`'s first occurrence in `source`.
fn span_of(source: &str, needle: &str) -> ByteRange {
    let start = source.find(needle).unwrap_or_default();
    ByteRange {
        start,
        end: start.saturating_add(needle.len()),
    }
}

/// The byte range of `needle`'s last occurrence in `source`.
fn tail_of(source: &str, needle: &str) -> ByteRange {
    let start = source.rfind(needle).unwrap_or_default();
    ByteRange {
        start,
        end: start.saturating_add(needle.len()),
    }
}
