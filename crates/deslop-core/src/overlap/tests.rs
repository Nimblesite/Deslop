//! Unit tests for [FUSED-SHARED-SUBTREE].
//!
//! `structural` feeds bucket routing, ranking, the duplication metric
//! and cross-cluster subsumption, so a silent error in this measurement
//! changes every report without failing anything. These isolate the
//! measurement itself: the alignment's arithmetic, the discriminator
//! that a multiset of shared hashes cannot express, and the large-tree
//! fallback's lower-bound guarantee.

use std::{path::PathBuf, sync::Arc};

use super::{
    alignment::aligned_shared_nodes, build_view, credit::credit_shared_nodes,
    kind_shared_upper_bound, EndpointView, OverlapMeasurer, ALIGNMENT_MAX_NODES,
    ENDPOINT_VIEW_MEMO_MAX, EXACT_RESULT_MEMO_MAX,
};
use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::{collect_fingerprints, subtree_hash, Fingerprint, HashScratch},
    lang::LanguageParser,
    registry_fixtures::{pair_ids, rust_pair_ids, ABSENT_RS, LEFT_RS, WIDE_LEFT_RS, WIDE_RIGHT_RS},
    state::{FileId, FileRegistry},
};

/// A parsed fixture: its normalised tree and the whole-file fingerprint.
pub(super) struct Parsed {
    /// Normalised root.
    pub(super) tree: NormalizedNode,
    /// Fingerprint spanning the tree's own byte range.
    pub(super) whole: Fingerprint,
}

/// Parses `source` as Rust and fingerprints its root.
fn parse(source: &str, file_id: FileId) -> Result<Parsed, String> {
    let tree = crate::lang::rust_lang::RustParser
        .parse_and_normalize(source.as_bytes(), file_id)
        .map_err(|error| format!("the Rust fixture must parse: {error}"))?;
    let node_count = tree.subtree_node_count();
    let whole = Fingerprint {
        hash: subtree_hash(&tree, &mut HashScratch::default()),
        file_id,
        byte_range: tree.byte_range,
        node_count,
    };
    Ok(Parsed { tree, whole })
}

/// Parses `left_source` and `right_source` as two Rust files.
pub(super) fn parse_pair(
    left_source: &str,
    right_source: &str,
) -> Result<(Parsed, Parsed), String> {
    let (left_id, right_id) = rust_pair_ids();
    Ok((parse(left_source, left_id)?, parse(right_source, right_id)?))
}

/// Both endpoints' views over `trees`, which must hold each endpoint's
/// file.
fn endpoint_views(
    trees: &[NormalizedNode],
    left: &Fingerprint,
    right: &Fingerprint,
) -> Result<(EndpointView, EndpointView), String> {
    let index = trees
        .iter()
        .map(|tree| (tree.file_id, tree))
        .collect::<std::collections::HashMap<FileId, &NormalizedNode>>();
    let left_view = build_view(&index, left).ok_or("the left endpoint resolves")?;
    let right_view = build_view(&index, right).ok_or("the right endpoint resolves")?;
    Ok((left_view, right_view))
}

/// Parses two files and returns their whole-file endpoint views.
fn views_of(left_source: &str, right_source: &str) -> Result<(EndpointView, EndpointView), String> {
    let (left, right) = parse_pair(left_source, right_source)?;
    let trees = [left.tree, right.tree];
    endpoint_views(&trees, &left.whole, &right.whole)
}

/// A nested block large enough to exercise the credited fallback.
fn boost_block(statements: usize) -> String {
    let body = (0..statements).fold(String::new(), |mut body, index| {
        use std::fmt::Write as _;
        let _written = writeln!(body, "        inner = inner + {index};");
        body
    });
    format!("    let boost = {{\n        let mut inner = seed;\n{body}        inner\n    }};\n")
}

/// A function past the alignment cap whose tail nests `block`.
fn host_function(statements: usize, block: &str) -> String {
    let body = (0..statements).fold(String::new(), |mut body, index| {
        use std::fmt::Write as _;
        let _written = writeln!(body, "    total = total + {index};");
        body
    });
    format!(
        "fn alpha(seed: u32) -> u32 {{\n    let mut total = seed;\n{body}{block}    total + boost\n}}\n"
    )
}

/// A small function whose body is exactly `block`.
fn rider_function(block: &str) -> String {
    format!("fn beta(seed: u32) -> u32 {{\n{block}    boost\n}}\n")
}

/// Measures overlap between two whole-file fixtures.
fn overlap_of(left_source: &str, right_source: &str) -> Result<f64, String> {
    let (left, right) = parse_pair(left_source, right_source)?;
    let trees = vec![left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    Ok(measurer.overlap(&left.whole, &right.whole))
}

/// The #408 shape: a method, and the same method with one extra
/// statement inserted into its loop. Every identifier is renamed too,
/// so nothing but the shape can match.
pub(super) const ACCUMULATE: &str = "\
fn accumulate(bound: u32) -> u32 {
    if bound == 0 {
        return 0;
    }
    let mut running = 0;
    for step in 0..bound {
        running = running + step;
    }
    running
}
";

/// `ACCUMULATE` with one inserted statement and a full rename.
pub(super) const AGGREGATE_WITH_INSERTION: &str = "\
fn aggregate(limit: u32) -> u32 {
    if limit == 0 {
        return 0;
    }
    let mut total = 0;
    for cursor in 0..limit {
        total = total + cursor;
        total = total + 2;
    }
    total
}
";

/// Same statement vocabulary, different program: an `if`, a `let`, a
/// `for` and a return, assembled to compute something unrelated. A
/// multiset of shared subtree hashes cannot separate this from the
/// genuine copy above; an ordered alignment can.
const UNRELATED_SAME_VOCABULARY: &str = "\
fn describe(flag: u32) -> u32 {
    let mut label = 0;
    for entry in 0..3 {
        if entry == flag {
            return entry;
        }
        label = label + entry;
    }
    label
}
";

const EXACT_OVERLAP: f64 = 1.0;
const NO_OVERLAP: f64 = 0.0;
const NO_ALIGNMENTS: u64 = 0;
const UNRESOLVABLE_OFFSET: usize = usize::MAX;

#[test]
fn merkle_equal_endpoints_short_circuit_to_one() -> Result<(), String> {
    let (left, right) = parse_pair(ACCUMULATE, ACCUMULATE)?;
    let trees = [left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    let overlap = measurer.overlap(&left.whole, &right.whole);
    assert_eq!(overlap.to_bits(), EXACT_OVERLAP.to_bits());
    assert_eq!(measurer.stats().alignments, NO_ALIGNMENTS);
    Ok(())
}

// [FUSED-SHARED-SUBTREE] Equal hashes prove an exact overlap only after
// each byte range resolves, even when a valid copy was measured first.
#[test]
fn merkle_equal_unresolvable_range_still_measures_zero() -> Result<(), String> {
    let (left, right) = parse_pair(ACCUMULATE, ACCUMULATE)?;
    let mut missing = right.whole.clone();
    missing.byte_range = ByteRange {
        start: UNRESOLVABLE_OFFSET,
        end: UNRESOLVABLE_OFFSET,
    };
    let trees = [left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    assert_eq!(
        measurer.overlap(&left.whole, &right.whole).to_bits(),
        EXACT_OVERLAP.to_bits()
    );
    assert_eq!(
        measurer.overlap(&left.whole, &missing).to_bits(),
        NO_OVERLAP.to_bits()
    );
    assert_eq!(measurer.stats().alignments, NO_ALIGNMENTS);
    Ok(())
}

// [FUSED-SHARED-SUBTREE] The measurement #408 turns on. The enclosing
// method pair carried a literal `structural = 0.0` because the inserted
// statement rehashes every ancestor Merkle node; it must now measure
// high enough to clear `SHARED_SUBTREE_MIN_OVERLAP`, or the whole-method
// clone stays invisible in four of five languages.
#[test]
fn one_inserted_statement_still_measures_as_mostly_shared() -> Result<(), String> {
    let overlap = overlap_of(ACCUMULATE, AGGREGATE_WITH_INSERTION)?;
    assert!(
        overlap >= crate::pair::SHARED_SUBTREE_MIN_OVERLAP,
        "a renamed one-statement Type-3 near-miss must clear the admission floor \
         {floor}, got {overlap}",
        floor = crate::pair::SHARED_SUBTREE_MIN_OVERLAP,
    );
    assert!(
        overlap < 1.0,
        "it must still be bounded below 1.0 — the statement really was inserted, \
         got {overlap}"
    );
    Ok(())
}

// The discriminator, and the reason the measure is an alignment rather
// than a bag of matching subtree hashes. This pair shares the same
// statement vocabulary as the pair above — `if`, `let`, `for`, a
// compound assignment, a return — so their shared-hash multisets are
// comparable. Only the order and nesting of the matches separate a real
// copy from an unrelated program, and only the alignment reads those.
#[test]
fn shared_statement_vocabulary_alone_does_not_reach_the_floor() -> Result<(), String> {
    let genuine = overlap_of(ACCUMULATE, AGGREGATE_WITH_INSERTION)?;
    let unrelated = overlap_of(ACCUMULATE, UNRELATED_SAME_VOCABULARY)?;
    assert!(
        unrelated < crate::pair::SHARED_SUBTREE_MIN_OVERLAP,
        "an unrelated program built from the same statement kinds must stay under \
         the admission floor {floor}, got {unrelated}",
        floor = crate::pair::SHARED_SUBTREE_MIN_OVERLAP,
    );
    assert!(
        genuine > unrelated,
        "the genuine near-miss ({genuine}) must measure strictly above the \
         vocabulary-only match ({unrelated}) — if these are equal the measure is \
         reading a multiset, not an alignment"
    );
    Ok(())
}

#[test]
fn an_unresolvable_endpoint_measures_zero() -> Result<(), String> {
    let (file_id, other) = pair_ids(LEFT_RS, ABSENT_RS);
    let left = parse(ACCUMULATE, file_id)?;
    let trees = vec![left.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    let ghost = Fingerprint {
        hash: [9_u8; 32],
        file_id: other,
        byte_range: ByteRange { start: 0, end: 10 },
        node_count: 40,
    };
    let overlap = measurer.overlap(&left.whole, &ghost);
    assert!(
        overlap.abs() < f64::EPSILON,
        "an endpoint whose file has no tree measures 0.0, not a guess: {overlap}"
    );
    Ok(())
}

#[test]
fn repeated_measurement_of_one_pair_is_stable() -> Result<(), String> {
    let (left_id, right_id) = rust_pair_ids();
    let left = parse(ACCUMULATE, left_id)?;
    let right = parse(AGGREGATE_WITH_INSERTION, right_id)?;
    let trees = vec![left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    let first = measurer.overlap(&left.whole, &right.whole);
    let cached = measurer.overlap(&left.whole, &right.whole);
    let reversed = measurer.overlap(&right.whole, &left.whole);
    assert!(
        (first - cached).abs() < f64::EPSILON,
        "the memoised second read must equal the first: {first} vs {cached}"
    );
    assert!(
        (first - reversed).abs() < f64::EPSILON,
        "overlap is symmetric and its cache key is order-insensitive: \
         {first} vs {reversed}"
    );
    Ok(())
}

// [FUSED-SHARED-SUBTREE] The large-tree fallback is only ever allowed
// to *suppress* a rescue, never to manufacture one, so it must never
// exceed the alignment it stands in for. Asserted on the same pair the
// alignment measures, which is the only way to compare them directly.
#[test]
fn the_large_tree_fallback_never_exceeds_the_alignment() -> Result<(), String> {
    let (left_view, right_view) = views_of(ACCUMULATE, AGGREGATE_WITH_INSERTION)?;
    let aligned = aligned_shared_nodes(&left_view, &right_view);
    let credited = credit_shared_nodes(&left_view, &right_view);
    assert!(
        credited <= aligned,
        "the greedy shared-hash bound ({credited}) must never exceed the aligned \
         shared mass ({aligned}) — a bound that overshoots would admit pairs the \
         alignment rejects"
    );
    assert!(
        aligned > 0,
        "the alignment must credit real shared mass on this pair, got {aligned}"
    );
    Ok(())
}

mod bounds;
mod cases;
mod exact_bounds;
