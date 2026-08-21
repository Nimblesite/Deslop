//! Unit tests for [FUSION-SHARED-SUBTREE].
//!
//! `structural` feeds bucket routing, ranking, the duplication metric
//! and cross-cluster subsumption, so a silent error in this measurement
//! changes every report without failing anything. These isolate the
//! measurement itself: the alignment's arithmetic, the discriminator
//! that a multiset of shared hashes cannot express, and the large-tree
//! fallback's lower-bound guarantee.

use std::path::PathBuf;

use super::{
    alignment::aligned_shared_nodes, build_view, credit_shared_nodes, EndpointView,
    OverlapMeasurer, ALIGNMENT_MAX_NODES,
};
use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::{collect_fingerprints, Fingerprint},
    lang::LanguageParser,
    state::{FileId, FileRegistry},
};

/// A parsed fixture: its normalised tree and the whole-file fingerprint.
struct Parsed {
    /// Normalised root.
    tree: NormalizedNode,
    /// Fingerprint spanning the tree's own byte range.
    whole: Fingerprint,
}

/// Parses `source` as Rust and fingerprints its root.
fn parse(source: &str, file_id: FileId) -> Result<Parsed, String> {
    let tree = crate::lang::rust_lang::RustParser
        .parse_and_normalize(source.as_bytes(), file_id)
        .map_err(|error| format!("the Rust fixture must parse: {error}"))?;
    let node_count = count_nodes(&tree);
    let whole = Fingerprint {
        hash: root_hash(&tree),
        file_id,
        byte_range: tree.byte_range,
        node_count,
    };
    Ok(Parsed { tree, whole })
}

/// Total nodes in a subtree, including the root.
fn count_nodes(node: &NormalizedNode) -> usize {
    node.children
        .iter()
        .map(count_nodes)
        .fold(1, usize::saturating_add)
}

/// The tree's own Merkle hash, via the shared fingerprint walk. Uses a
/// `min_nodes` of 1 so the root itself is always emitted.
fn root_hash(tree: &NormalizedNode) -> [u8; 32] {
    collect_fingerprints(tree, 1)
        .into_iter()
        .find(|fingerprint| fingerprint.byte_range == tree.byte_range)
        .map_or([0_u8; 32], |fingerprint| fingerprint.hash)
}

/// Measures overlap between two whole-file fixtures.
fn overlap_of(left_source: &str, right_source: &str) -> Result<f64, String> {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("left.rs"));
    let right_id = registry.register(PathBuf::from("right.rs"));
    let left = parse(left_source, left_id)?;
    let right = parse(right_source, right_id)?;
    let trees = vec![left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    Ok(measurer.overlap(&left.whole, &right.whole))
}

/// The #408 shape: a method, and the same method with one extra
/// statement inserted into its loop. Every identifier is renamed too,
/// so nothing but the shape can match.
const ACCUMULATE: &str = "\
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
const AGGREGATE_WITH_INSERTION: &str = "\
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

#[test]
fn merkle_equal_endpoints_short_circuit_to_one() -> Result<(), String> {
    let overlap = overlap_of(ACCUMULATE, ACCUMULATE)?;
    assert!(
        (overlap - 1.0).abs() < f64::EPSILON,
        "two copies of one file must measure exactly 1.0, got {overlap}"
    );
    Ok(())
}

// [FUSION-SHARED-SUBTREE] The measurement #408 turns on. The enclosing
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
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("left.rs"));
    let other = registry.register(PathBuf::from("absent.rs"));
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
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("left.rs"));
    let right_id = registry.register(PathBuf::from("right.rs"));
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

// [FUSION-SHARED-SUBTREE] The large-tree fallback is only ever allowed
// to *suppress* a rescue, never to manufacture one, so it must never
// exceed the alignment it stands in for. Asserted on the same pair the
// alignment measures, which is the only way to compare them directly.
#[test]
fn the_large_tree_fallback_never_exceeds_the_alignment() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("left.rs"));
    let right_id = registry.register(PathBuf::from("right.rs"));
    let left = parse(ACCUMULATE, left_id)?;
    let right = parse(AGGREGATE_WITH_INSERTION, right_id)?;
    let trees = [left.tree, right.tree];
    let index = trees
        .iter()
        .map(|tree| (tree.file_id, tree))
        .collect::<std::collections::HashMap<FileId, &NormalizedNode>>();
    let left_view = build_view(&index, &left.whole).ok_or("the left endpoint resolves")?;
    let right_view = build_view(&index, &right.whole).ok_or("the right endpoint resolves")?;
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

// The cap is what keeps the quadratic DP bounded. It is a real number
// in the admission path, so a change to it is a performance decision
// that must be made deliberately rather than drifted into.
#[test]
fn the_alignment_cap_is_the_documented_operating_point() {
    assert_eq!(
        ALIGNMENT_MAX_NODES, 768,
        "changing the alignment cap changes which pairs get the exact measure \
         and which get the conservative bound — move the spec with it"
    );
}

/// A hand-built view over a flat kind sequence, for the arithmetic pins
/// below. Each entry is a leaf, so every node is its own leftmost leaf.
fn flat_view(kinds: &[&'static str]) -> EndpointView {
    EndpointView::from_flat_leaves(kinds)
}

#[test]
fn identical_flat_sequences_align_completely() {
    let left = flat_view(&["a", "b", "c"]);
    let right = flat_view(&["a", "b", "c"]);
    assert_eq!(
        aligned_shared_nodes(&left, &right),
        3,
        "three identical leaves under one root share all three"
    );
}

#[test]
fn one_extra_leaf_costs_exactly_one_node() {
    let left = flat_view(&["a", "b", "c", "d"]);
    let right = flat_view(&["a", "b", "c"]);
    assert_eq!(
        aligned_shared_nodes(&left, &right),
        3,
        "a single insertion costs exactly the inserted node — this is the \
         arithmetic the whole #408 rescue rests on"
    );
}

#[test]
fn a_relabelled_leaf_costs_exactly_one_node() {
    let left = flat_view(&["a", "b", "c"]);
    let right = flat_view(&["a", "x", "c"]);
    assert_eq!(
        aligned_shared_nodes(&left, &right),
        2,
        "one differing kind costs one relabel, not the whole sequence"
    );
}

#[test]
fn wholly_different_sequences_share_nothing() {
    let left = flat_view(&["a", "b"]);
    let right = flat_view(&["x", "y"]);
    assert_eq!(
        aligned_shared_nodes(&left, &right),
        0,
        "two leaves that agree on nothing share no mass"
    );
}

/// A Rust function of `statements` accumulator lines — enough nodes to
/// push an endpoint past [`ALIGNMENT_MAX_NODES`].
fn wide_function(name: &str, statements: usize, extra: &str) -> String {
    let body = (0..statements).fold(String::new(), |mut body, index| {
        use std::fmt::Write as _;
        let _written = writeln!(body, "    total = total + {index};");
        body
    });
    format!(
        "fn {name}(seed: u32) -> u32 {{\n    let mut total = seed;\n{body}{extra}    total\n}}\n"
    )
}

// [FUSION-SHARED-SUBTREE] Endpoints past the alignment cap take the
// greedy coverage bound instead of the quadratic DP. The bound is only
// ever allowed to *suppress* a rescue, so the path must still measure a
// near-copy as substantially shared — a fallback that read near zero
// would silently reinstate the #408 recall hole on exactly the large
// files where duplication costs most.
#[test]
fn endpoints_past_the_alignment_cap_still_measure_as_shared() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("wide_left.rs"));
    let right_id = registry.register(PathBuf::from("wide_right.rs"));
    let left_source = wide_function("accumulate", 260, "");
    let right_source = wide_function("aggregate", 260, "    total = total + 7;\n");
    let left = parse(&left_source, left_id)?;
    let right = parse(&right_source, right_id)?;
    assert!(
        left.whole.node_count > ALIGNMENT_MAX_NODES,
        "the fixture must exceed the alignment cap to exercise the fallback, got {}",
        left.whole.node_count
    );
    let trees = [left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    let overlap = measurer.overlap(&left.whole, &right.whole);
    assert!(
        overlap >= crate::pair::SHARED_SUBTREE_MIN_OVERLAP,
        "two 260-statement copies differing by one line must clear the admission \
         floor through the fallback bound, got {overlap}"
    );
    assert!(
        overlap < 1.0,
        "the copies are not identical, so the bound must stay below 1.0, got {overlap}"
    );
    Ok(())
}
/// The boost block: a fingerprint-worthy subtree nested inside
/// [`host_function`]'s tail and duplicated standalone by
/// [`rider_function`], so the same hash exists both nested and disjoint.
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

/// A small function whose body is exactly `block` — the disjoint second
/// copy of the nested subtree.
fn rider_function(block: &str) -> String {
    format!("fn beta(seed: u32) -> u32 {{\n{block}    boost\n}}\n")
}

// [FUSION-SHARED-SUBTREE] The fallback is documented as a conservative
// lower bound on the alignment. Adversarial shape: the left endpoint is
// `alpha` (which nests the boost block) plus a disjoint second copy of
// the block; the right endpoint is `alpha` alone. Tracking credited
// spans on the left only, with the right side as bare hash counts,
// credits the right-hand block twice — once inside `alpha`'s subtree,
// once against the left's disjoint copy — so the bound overshoots the
// alignment it stands in for and can admit pairs the alignment rejects.
#[test]
fn the_fallback_never_credits_a_nested_right_subtree_twice() -> Result<(), String> {
    let block = boost_block(40);
    let left_source = format!("{}\n{}", host_function(260, &block), rider_function(&block));
    let right_source = host_function(260, &block);
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("left.rs"));
    let right_id = registry.register(PathBuf::from("right.rs"));
    let left = parse(&left_source, left_id)?;
    let right = parse(&right_source, right_id)?;
    assert!(
        right.whole.node_count > ALIGNMENT_MAX_NODES,
        "the fixture must exceed the alignment cap so the E2E path takes the \
         fallback for this pair, got {}",
        right.whole.node_count
    );
    let trees = [left.tree, right.tree];
    let index = trees
        .iter()
        .map(|tree| (tree.file_id, tree))
        .collect::<std::collections::HashMap<FileId, &NormalizedNode>>();
    let left_view = build_view(&index, &left.whole).ok_or("the left endpoint resolves")?;
    let right_view = build_view(&index, &right.whole).ok_or("the right endpoint resolves")?;
    let aligned = aligned_shared_nodes(&left_view, &right_view);
    let credited = credit_shared_nodes(&left_view, &right_view);
    assert!(
        aligned > 0,
        "the alignment must credit the shared `alpha` mass, got {aligned}"
    );
    assert!(
        credited <= aligned,
        "the greedy bound ({credited}) must never exceed the aligned shared \
         mass ({aligned}): the right-hand boost block sits inside the credited \
         `alpha` subtree, so a second credit for it counts those nodes twice"
    );
    Ok(())
}

/// Terms in the arithmetic expression `ts-mixed-band` is built from. The
/// fixture that pins the rescue
/// (`without_embeddings_the_mid_band_pair_is_visible_without_saturating`)
/// is ninety terms wide.
const RESCUED_EXPRESSION_TERMS: usize = 90;

/// A function whose body is one `terms`-wide arithmetic expression —
/// `ts-mixed-band`'s shape, in the language these tests parse.
fn wide_expression(name: &str, terms: usize) -> String {
    let sum = (1..=terms).fold(String::from("seed"), |mut expression, index| {
        use std::fmt::Write as _;
        let _written = write!(expression, " + seed * {index}");
        expression
    });
    format!("fn {name}(seed: u32) -> u32 {{\n    {sum}\n}}\n")
}

// [FUSION-SHARED-SUBTREE] The cap is measured in nodes of the
// *normalised* tree, so a normalisation change moves what it reaches
// without the number changing. [PIPELINE-NORMALIZE-AST-OPERATOR] did
// exactly that: operator tokens became leaves, an operator-dense
// expression counts around half as many nodes again, and at 512 the
// ninety-term pair fell onto the conservative bound, scored under the
// admission floor and was reported as nothing at all. Measuring the
// expression here — rather than restating a number — is what makes this
// fail again the next time normalisation grows the tree.
#[test]
fn the_cap_still_reaches_the_expression_the_rescue_is_pinned_on() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("ledger.rs"));
    let parsed = parse(
        &wide_expression("settle", RESCUED_EXPRESSION_TERMS),
        file_id,
    )?;

    assert!(
        parsed.whole.node_count <= ALIGNMENT_MAX_NODES,
        "a {RESCUED_EXPRESSION_TERMS}-term expression must still get the exact \
         alignment: it normalises to {} nodes against a cap of \
         {ALIGNMENT_MAX_NODES}, and past the cap the conservative bound scores \
         a consistent rename under the admission floor and reports nothing",
        parsed.whole.node_count
    );
    Ok(())
}
