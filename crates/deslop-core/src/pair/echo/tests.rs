//! [FUSED-SHARED-SUBTREE-INDEX] The exact-clone index answers by range.
//!
//! Every answer below is defined by containment alone, so it must not
//! move with the order clones were recorded in or with how many
//! unrelated clones the file holds. The scenarios mirror the shape
//! that exposed the scan-per-question cost: one file of look-alike
//! functions whose every same-position statement is Merkle-equal.

use super::{range_index::RangeIndex, ExactClones};
use crate::{
    ast::ByteRange,
    fingerprint::Fingerprint,
    pair::{
        CandidatePair, PairScore, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD, LSH_ONLY_MIN_NODE_COUNT,
        SHARED_SUBTREE_MIN_JACCARD,
    },
    state::FileRegistry,
};

/// The one file every fingerprint below lives in.
const FILE: &str = "ledger.js";
/// A lookup outside the fixture corpus is a broken test, not a verdict.
const OUTSIDE_CORPUS: &str = "fingerprint index outside the corpus";
/// A range the whole tree fits inside.
const WHOLE: ByteRange = range(0, 1000);
/// Query ranges over the labelled index.
const QUERY_INNER: ByteRange = range(10, 30);
const QUERY_SIBLING: ByteRange = range(40, 50);
const QUERY_FAR: ByteRange = range(250, 260);
const QUERY_BEYOND: ByteRange = range(400, 410);
/// Ranges the wide-early scenario reaches past.
const WRAPPER: ByteRange = range(0, 1000);
const SCRAP_COUNT: usize = 100;
const LATE_QUERY: ByteRange = range(900, 950);
const ONE_SCRAP: ByteRange = range(50, 51);
const TWO_SCRAPS: ByteRange = range(50, 52);

/// Fingerprint indices of the one-file corpus: three whole functions
/// with two inner statements each, a file wrapper, and one stray leaf.
const OUTER_ONE: usize = 0;
const INNER_ONE_A: usize = 1;
const INNER_ONE_B: usize = 2;
const OUTER_TWO: usize = 3;
const INNER_TWO_A: usize = 4;
const INNER_TWO_B: usize = 5;
const OUTER_THREE: usize = 6;
const WRAPPER_NODE: usize = 7;
const STRAY: usize = 8;
const FINGERPRINT_COUNT: usize = 9;
/// Node counts the corpus claims.
const OUTER_NODES: usize = 50;
const INNER_A_NODES: usize = 20;
const INNER_B_NODES: usize = 8;
const WRAPPER_NODES: usize = 200;
const STRAY_NODES: usize = 30;
/// One node past and one node short of the largest wrapped clone's
/// remainder, `OUTER_NODES - INNER_A_NODES`.
const REMAINDER_PLUS_ONE: usize = 31;
const REMAINDER: usize = 30;

/// The look-alike family: functions of equal shape whose statements
/// are Merkle-equal across every member.
const FAMILY_MEMBERS: usize = 12;
const STATEMENTS_PER_MEMBER: usize = 3;
const MEMBER_STRIDE: usize = 100;
const STATEMENT_STRIDE: usize = 20;
const STATEMENT_WIDTH: usize = 10;
const MEMBER_WIDTH: usize = 80;
const MEMBER_NODES: usize = 40;
/// Statement node counts by position, so the largest is unambiguous.
const STATEMENT_NODES: [usize; STATEMENTS_PER_MEMBER] = [6, 9, 7];
const LARGEST_STATEMENT_NODES: usize = 9;

const fn range(start: usize, end: usize) -> ByteRange {
    ByteRange { start, end }
}

/// Labels of the related entries, sorted so the assertion reads as a set.
fn names<'a>(related: impl Iterator<Item = &'a (ByteRange, &'static str)>) -> Vec<&'static str> {
    let mut names: Vec<_> = related.map(|(_, name)| *name).collect();
    names.sort_unstable();
    names
}

fn labelled() -> Vec<(ByteRange, &'static str)> {
    vec![
        (range(0, 100), "file"),
        (range(10, 20), "inner"),
        (range(10, 30), "equal"),
        (range(5, 15), "straddles the start"),
        (range(0, 25), "straddles the end"),
        (range(40, 50), "later sibling"),
        (range(10, 300), "same start, wider"),
        (range(200, 300), "far"),
    ]
}

#[test]
fn related_ranges_are_those_inside_or_around_the_query_and_no_other() {
    let index = RangeIndex::new(labelled());
    assert_eq!(
        names(index.related(QUERY_INNER)),
        ["equal", "file", "inner", "same start, wider"],
        "inside: the equal and nested ranges; around: the file and the wider range sharing its start"
    );
    assert_eq!(
        names(index.related(QUERY_SIBLING)),
        ["file", "later sibling", "same start, wider"]
    );
    assert_eq!(
        names(index.related(QUERY_FAR)),
        ["far", "same start, wider"]
    );
    assert!(names(index.related(QUERY_BEYOND)).is_empty());
    assert_eq!(
        names(index.related(WHOLE)).len(),
        labelled().len(),
        "a query covering everything relates to every entry"
    );
}

#[test]
fn a_wide_early_range_is_reached_past_many_short_ones() {
    let mut entries = vec![(WRAPPER, "wrapper")];
    entries.extend((1..SCRAP_COUNT).map(|start| (range(start, start.saturating_add(1)), "scrap")));
    let index = RangeIndex::new(entries);
    assert_eq!(names(index.related(LATE_QUERY)), ["wrapper"]);
    assert_eq!(names(index.related(ONE_SCRAP)), ["scrap", "wrapper"]);
    assert_eq!(
        names(index.related(TWO_SCRAPS)),
        ["scrap", "scrap", "wrapper"]
    );
}

#[test]
fn insertion_order_does_not_change_the_answer() {
    let forward = RangeIndex::new(labelled());
    let mut reversed = labelled();
    reversed.reverse();
    let reversed = RangeIndex::new(reversed);
    for query in [QUERY_INNER, QUERY_SIBLING, QUERY_FAR, QUERY_BEYOND, WHOLE] {
        assert_eq!(
            names(forward.related(query)),
            names(reversed.related(query))
        );
    }
}

fn fingerprint(
    file_id: crate::state::FileId,
    byte_range: ByteRange,
    node_count: usize,
    hash: u8,
) -> Fingerprint {
    Fingerprint {
        hash: [hash; 32],
        file_id,
        byte_range,
        node_count,
    }
}

/// A Merkle-equal candidate pair between two fingerprint indices.
fn exact_pair(left: usize, right: usize, nodes: usize) -> CandidatePair {
    CandidatePair {
        left,
        right,
        endpoint_node_counts: (nodes, nodes),
        lsh_only_node_floor: LSH_ONLY_MIN_NODE_COUNT,
        lsh_only_min_jaccard: LSH_ONLY_MIN_JACCARD,
        fused_min_score: FUSED_THRESHOLD,
        shared_subtree_overlap: 0.0,
        verified_async_core: false,
        score: PairScore {
            structural: 1.0,
            token_jaccard: SHARED_SUBTREE_MIN_JACCARD,
            embedding_cos: 0.0,
        },
    }
}

/// Three functions in one file. The first two share both inner
/// statements exactly; the third shares only the outer shape.
fn corpus() -> ([Fingerprint; FINGERPRINT_COUNT], Vec<CandidatePair>) {
    let mut registry = FileRegistry::new();
    let file = registry.register(FILE.into());
    let fingerprints = [
        fingerprint(file, range(0, 100), OUTER_NODES, 1),
        fingerprint(file, range(10, 30), INNER_A_NODES, 2),
        fingerprint(file, range(40, 60), INNER_B_NODES, 3),
        fingerprint(file, range(200, 300), OUTER_NODES, 1),
        fingerprint(file, range(210, 230), INNER_A_NODES, 2),
        fingerprint(file, range(240, 260), INNER_B_NODES, 3),
        fingerprint(file, range(400, 500), OUTER_NODES, 1),
        fingerprint(file, range(0, 500), WRAPPER_NODES, 9),
        fingerprint(file, range(600, 700), STRAY_NODES, 11),
    ];
    let pairs = vec![
        exact_pair(OUTER_ONE, OUTER_TWO, OUTER_NODES),
        exact_pair(INNER_ONE_A, INNER_TWO_A, INNER_A_NODES),
        exact_pair(INNER_ONE_B, INNER_TWO_B, INNER_B_NODES),
        exact_pair(OUTER_ONE, OUTER_THREE, OUTER_NODES),
        exact_pair(OUTER_TWO, OUTER_THREE, OUTER_NODES),
    ];
    (fingerprints, pairs)
}

#[test]
fn enclosed_nodes_is_the_largest_clone_both_declarations_wrap_besides_themselves(
) -> Result<(), &'static str> {
    let (fingerprints, pairs) = corpus();
    let clones = ExactClones::within_one_file(&pairs, &fingerprints);
    let at = |index: usize| fingerprints.get(index).ok_or(OUTSIDE_CORPUS);
    assert_eq!(
        clones.enclosed_nodes(at(OUTER_ONE)?, at(OUTER_TWO)?),
        INNER_A_NODES
    );
    assert_eq!(
        clones.enclosed_nodes(at(OUTER_TWO)?, at(OUTER_ONE)?),
        INNER_A_NODES,
        "endpoint order is immaterial"
    );
    assert_eq!(clones.enclosed_nodes(at(OUTER_ONE)?, at(OUTER_THREE)?), 0);
    assert_eq!(
        clones.enclosed_nodes(at(INNER_ONE_A)?, at(INNER_TWO_A)?),
        0,
        "a clone is never its own interior"
    );
    assert_eq!(
        clones.enclosed_nodes(at(OUTER_ONE)?, at(WRAPPER_NODE)?),
        OUTER_NODES,
        "the wrapper wraps every partner of the first function"
    );
    assert_eq!(clones.enclosed_nodes(at(OUTER_THREE)?, at(STRAY)?), 0);
    Ok(())
}

#[test]
fn claimed_nodes_reads_wrapped_enclosing_and_mixed_clones_alike() -> Result<(), &'static str> {
    let (fingerprints, pairs) = corpus();
    let clones = ExactClones::within_one_file(&pairs, &fingerprints);
    let at = |index: usize| fingerprints.get(index).ok_or(OUTSIDE_CORPUS);
    assert_eq!(
        clones.claimed_nodes(at(OUTER_ONE)?, at(OUTER_TWO)?),
        Some(INNER_A_NODES),
        "wrapped on both sides: the largest inner clone"
    );
    assert_eq!(
        clones.claimed_nodes(at(INNER_ONE_A)?, at(INNER_TWO_A)?),
        Some(INNER_A_NODES),
        "inside the outer clone on both sides: the smaller endpoint"
    );
    assert_eq!(
        clones.claimed_nodes(at(INNER_ONE_A)?, at(OUTER_TWO)?),
        Some(INNER_A_NODES),
        "inside the outer clone on one side and the outer clone itself on the other: still \
         inside on both, so the smaller endpoint"
    );
    assert_eq!(
        clones.claimed_nodes(at(WRAPPER_NODE)?, at(INNER_TWO_A)?),
        Some(WRAPPER_NODES),
        "wrapping the outer clone on one side while inside it on the other: the larger endpoint"
    );
    assert_eq!(clones.claimed_nodes(at(OUTER_THREE)?, at(STRAY)?), None);
    assert!(clones.wraps_within(at(OUTER_ONE)?, at(OUTER_TWO)?, REMAINDER_PLUS_ONE));
    assert!(!clones.wraps_within(at(OUTER_ONE)?, at(OUTER_TWO)?, REMAINDER));
    Ok(())
}

/// The look-alike family, every same-position statement Merkle-equal
/// across every pair of members.
fn family() -> (Vec<Fingerprint>, Vec<CandidatePair>) {
    let mut registry = FileRegistry::new();
    let file = registry.register(FILE.into());
    let mut fingerprints = Vec::new();
    for member in 0..FAMILY_MEMBERS {
        let start = member.saturating_mul(MEMBER_STRIDE);
        fingerprints.push(fingerprint(
            file,
            range(start, start.saturating_add(MEMBER_WIDTH)),
            MEMBER_NODES,
            1,
        ));
        for (position, nodes) in STATEMENT_NODES.iter().enumerate() {
            let statement = start
                .saturating_add(position.saturating_mul(STATEMENT_STRIDE))
                .saturating_add(1);
            let hash = u8::try_from(position.saturating_add(2)).unwrap_or(u8::MAX);
            fingerprints.push(fingerprint(
                file,
                range(statement, statement.saturating_add(STATEMENT_WIDTH)),
                *nodes,
                hash,
            ));
        }
    }
    let per_member = STATEMENTS_PER_MEMBER.saturating_add(1);
    let mut pairs = Vec::new();
    for left in 0..FAMILY_MEMBERS {
        for right in left.saturating_add(1)..FAMILY_MEMBERS {
            for slot in 0..per_member {
                let nodes = fingerprints.get(slot).map_or(0, |found| found.node_count);
                pairs.push(exact_pair(
                    left.saturating_mul(per_member).saturating_add(slot),
                    right.saturating_mul(per_member).saturating_add(slot),
                    nodes,
                ));
            }
        }
    }
    (fingerprints, pairs)
}

#[test]
fn a_family_of_look_alike_functions_answers_each_pair_from_its_own_interior() {
    let (fingerprints, pairs) = family();
    let clones = ExactClones::within_one_file(&pairs, &fingerprints);
    let per_member = STATEMENTS_PER_MEMBER.saturating_add(1);
    let member = |index: usize| fingerprints.get(index.saturating_mul(per_member));
    let mut judged: usize = 0;
    for left in 0..FAMILY_MEMBERS {
        for right in left.saturating_add(1)..FAMILY_MEMBERS {
            let (Some(first), Some(second)) = (member(left), member(right)) else {
                continue;
            };
            assert_eq!(
                clones.enclosed_nodes(first, second),
                LARGEST_STATEMENT_NODES
            );
            assert_eq!(
                clones.claimed_nodes(first, second),
                Some(LARGEST_STATEMENT_NODES)
            );
            judged = judged.saturating_add(1);
        }
    }
    assert_eq!(
        judged,
        FAMILY_MEMBERS.saturating_mul(FAMILY_MEMBERS.saturating_sub(1)) / 2
    );
    let (Some(statement), Some(partner)) = (
        fingerprints.get(1),
        fingerprints.get(per_member.saturating_add(1)),
    ) else {
        return;
    };
    assert_eq!(
        clones.enclosed_nodes(statement, partner),
        0,
        "two statements enclose nothing of their own"
    );
    assert_eq!(
        clones.claimed_nodes(statement, partner),
        Some(STATEMENT_NODES[0]),
        "the members around them claim the smaller statement"
    );
}
