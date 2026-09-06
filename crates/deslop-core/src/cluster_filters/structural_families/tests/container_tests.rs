//! Unit pins for [PIPELINE-CLUSTER-ELECT-CONTAINER] — the container
//! election of [`super::super::containers`]. Shares [`super`]'s
//! fingerprint builders; each case is a fixture geometry the E2E
//! suites cannot isolate.

use super::*;
use crate::cluster::VERBATIM_OVERTURN_MIN_NODES;

/// [`placed_corpus`] with an explicit node count per member, so a test
/// can hold a family under the idiom-mass floor.
fn placed_with_nodes(members: &[(u8, usize, usize, usize, usize)]) -> Vec<Fingerprint> {
    let placed: Vec<(u8, usize, usize, usize)> = members
        .iter()
        .map(|(tag, file, start, end, _)| (*tag, *file, *start, *end))
        .collect();
    placed_corpus(&placed)
        .into_iter()
        .zip(members.iter())
        .map(|(mut fingerprint, (_, _, _, _, nodes))| {
            fingerprint.node_count = *nodes;
            fingerprint
        })
        .collect()
}

// [PIPELINE-CLUSTER-ELECT-CONTAINER] — `rank_structural_only_policy` in
// miniature. Two class files hold seven shape-identical methods between
// them. The singleton classes, and the windows spanning two consecutive
// methods, are concatenations: each strictly encloses two or more
// occurrences of the method family — or one, with the family
// continuing past it in its own file — that family covers over two
// thirds of their bytes, and it strictly outnumbers them. Welded into
// one component, the same-file overlap collapse would elect one
// container occurrence per file and publish the seven-method family as
// a two-occurrence class view — dropping five findings and counting
// the constructors and fields between the methods as duplicated.
#[test]
fn a_container_concatenating_a_larger_family_is_elected_out() {
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 100, 200),
        (SUM_HASH, 0, 210, 310),
        (SUM_HASH, 0, 320, 420),
        (SUM_HASH, 1, 100, 200),
        (SUM_HASH, 1, 210, 310),
        (SUM_HASH, 1, 320, 420),
        (SUM_HASH, 1, 430, 530),
        (PRODUCT_HASH, 0, 100, 310),
        (PRODUCT_HASH, 1, 100, 310),
        (QUOTIENT_HASH, 0, 0, 430),
        (REMAINDER_HASH, 1, 0, 540),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(11)], &fingerprints)),
        vec![vec![0, 1, 2, 3, 4, 5, 6]],
        "the seven methods are the duplication; the singleton class views \
         and the two-method windows concatenate them and must not glue \
         the family into one occurrence per file"
    );
}

// The boundary the container election must not cross: an encloser most
// of whose bytes are its own code is a finding, not a concatenation. A
// copied method that happens to repeat a small statement block twice is
// the method-level clone [PIPELINE-CLUSTER-SUBSUME] elects between
// views — deleting it here would replace the extractable method pair
// with statement noise.
#[test]
fn an_encloser_with_code_of_its_own_is_not_a_container() {
    let fingerprints = placed_corpus(&[
        (QUOTIENT_HASH, 0, 0, 600),
        (QUOTIENT_HASH, 1, 0, 600),
        (SUM_HASH, 0, 50, 150),
        (SUM_HASH, 0, 200, 300),
        (SUM_HASH, 1, 50, 150),
        (SUM_HASH, 1, 200, 300),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(6)], &fingerprints)),
        vec![vec![0, 1, 2, 3, 4, 5]],
        "the statement family covers a third of each method, so the \
         methods carry code of their own and the component is a \
         nesting, not a concatenation"
    );
}

// The count-of-two bar has a second route: a window family that pads one
// enclosed occurrence with shared scaffolding — constructor plus fields
// plus the first method — encloses one occurrence of a family that
// continues right past it in the same file, and is mostly made of it.
// Left in the component, it out-widths the method in the same-file
// collapse and glues the scaffolding into the elected occurrence.
#[test]
fn a_padded_window_over_the_same_files_is_a_container() {
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 100, 200),
        (SUM_HASH, 0, 210, 310),
        (SUM_HASH, 1, 100, 200),
        (SUM_HASH, 1, 210, 310),
        (PRODUCT_HASH, 0, 80, 200),
        (PRODUCT_HASH, 1, 80, 200),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(6)], &fingerprints)),
        vec![vec![0, 1, 2, 3]],
        "the padded window covers the same two files as the four-member \
         family, encloses one of its occurrences with the family \
         continuing past it, and is five sixths that occurrence — a \
         concatenation of one, not a finding"
    );
}

// An idiom-sized enclosed family confers no container standing, however
// much of the encloser it covers: four byte-equal one-line asserts are
// most of a small test helper, and electing them out republishes the
// noise family their umbrella suppresses (`python-issue-71`,
// [CLONE-NOISE-LITERAL-VARIATION-CALLS]).
#[test]
fn an_idiom_sized_family_does_not_unseat_its_umbrella() {
    let idiom_nodes = VERBATIM_OVERTURN_MIN_NODES - 1;
    let fingerprints = placed_with_nodes(&[
        (QUOTIENT_HASH, 0, 0, 300, NODE_COUNT),
        (QUOTIENT_HASH, 1, 0, 300, NODE_COUNT),
        (SUM_HASH, 0, 10, 80, idiom_nodes),
        (SUM_HASH, 0, 90, 160, idiom_nodes),
        (SUM_HASH, 0, 170, 240, idiom_nodes),
        (SUM_HASH, 1, 10, 80, idiom_nodes),
        (SUM_HASH, 1, 90, 160, idiom_nodes),
        (SUM_HASH, 1, 170, 240, idiom_nodes),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(8)], &fingerprints)),
        vec![vec![0, 1, 2, 3, 4, 5, 6, 7]],
        "six sub-block idiom lines cover seventy percent of each helper, \
         but idiom mass has no standing to delete the umbrella that \
         suppresses it"
    );
}

// The inverse boundary (#339's sibling window): a family wholly made of
// another family's occurrences — two per-binding shapes repeating inside
// every window occurrence, nowhere else — is that window's fine
// structure. The window is the finding, and electing it out would
// publish the fragments in its place.
#[test]
fn a_window_wholly_containing_the_enclosed_family_is_kept() {
    let fingerprints = placed_corpus(&[
        (PRODUCT_HASH, 0, 0, 480),
        (PRODUCT_HASH, 1, 0, 480),
        (SUM_HASH, 0, 10, 240),
        (SUM_HASH, 0, 242, 470),
        (SUM_HASH, 1, 10, 240),
        (SUM_HASH, 1, 242, 470),
    ]);

    assert_eq!(
        member_lists(&elect(vec![component(6)], &fingerprints)),
        vec![vec![0, 1, 2, 3, 4, 5]],
        "every occurrence of the nested shape sits inside a window \
         occurrence, so the window is the duplication and the shapes \
         are its fine structure"
    );
}

/// Index of the 300-byte encloser the self-overlapping family must not
/// be able to elect out.
const ENCLOSER_MEMBER: usize = 4;

// [PIPELINE-CLUSTER-ELECT-CONTAINER] The share test is a *coverage*
// claim — `is_container`'s own contract says the enclosed occurrences
// must "supply at least 2/3 of its bytes". Summing their lengths is
// only that measure while they are disjoint. A self-overlapping family
// — a shape that repeats at sliding offsets, the same geometry
// `family_overflows` already refuses to grant the no-overflow exemption
// to — is counted once per occurrence, so three 100-byte occurrences
// spanning 150 real bytes read as 300 and any encloser up to 450 bytes
// long is elected out on bytes it never lost. Here the encloser spans
// 300 bytes, of which the family covers 10..160 — half — leaving 140
// bytes of its own code. `an_encloser_with_code_of_its_own_is_not_a_container`
// keeps exactly that view when the family is disjoint; overlap must not
// be able to delete it.
#[test]
fn an_encloser_is_not_a_container_of_a_family_that_overlaps_itself() {
    let fingerprints = placed_corpus(&[
        (SUM_HASH, 0, 10, 110),
        (SUM_HASH, 0, 20, 120),
        (SUM_HASH, 0, 60, 160),
        (SUM_HASH, 1, 10, 110),
        (PRODUCT_HASH, 0, 0, 300),
        (PRODUCT_HASH, 1, 400, 700),
    ]);

    let elected = elect(vec![component(6)], &fingerprints);

    assert_eq!(
        member_lists(&elected),
        vec![vec![0, 1, 2, 3, 4, 5]],
        "the overlapping shapes cover 150 of the encloser's 300 bytes, \
         not 300 — the encloser keeps 140 bytes of its own code, is not a \
         concatenation of them, and their ranges overlap its own, so this \
         is one duplication fingerprinted at two depths and the pass must \
         leave it whole for [PIPELINE-CLUSTER-SUBSUME] to elect between"
    );
    assert!(
        elected
            .iter()
            .any(|cluster| cluster.members.contains(&ENCLOSER_MEMBER)),
        "the encloser was deleted from the report — summing the overlapping \
         occurrences credited the family with 300 of its 300 bytes when it \
         covers 150, so the share test elected out a real finding"
    );
}
