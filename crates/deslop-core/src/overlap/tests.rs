//! Unit tests for [FUSED-SHARED-SUBTREE].
//!
//! `structural` feeds bucket routing, ranking, the duplication metric
//! and cross-cluster subsumption, so a silent error in this measurement
//! changes every report without failing anything. These isolate the
//! measurement itself: the alignment's arithmetic, the discriminator
//! that a multiset of shared hashes cannot express, and the large-tree
//! fallback's lower-bound guarantee.

use std::path::PathBuf;

use super::{
    alignment::aligned_shared_nodes, build_view, credit::credit_shared_nodes,
    kind_shared_upper_bound, EndpointView, OverlapMeasurer, ALIGNMENT_MAX_NODES,
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

// [FUSED-SHARED-SUBTREE] The large-tree fallback is only ever allowed
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

/// Two distinct functions, in one order.
const ALPHA_THEN_BETA: &str = "\
fn alpha(seed: u32) -> u32 {
    let mut total = seed;
    total = total + 1;
    total
}
fn beta(seed: u32) -> u32 {
    let mut count = seed;
    while count > 0 {
        count = count - 1;
    }
    count
}
";

/// The same two functions, in the other order. Nothing else differs.
const BETA_THEN_ALPHA: &str = "\
fn beta(seed: u32) -> u32 {
    let mut count = seed;
    while count > 0 {
        count = count - 1;
    }
    count
}
fn alpha(seed: u32) -> u32 {
    let mut total = seed;
    total = total + 1;
    total
}
";

/// [FUSED-SHARED-SUBTREE] The greedy fallback must never credit shared
/// mass that no ordered alignment could achieve.
///
/// `credit_shared_nodes` claims to be a conservative lower bound on the
/// alignment: "node mass matched under a bijection of disjoint
/// identical subtrees is achievable by an alignment". A tree alignment
/// is *ordered* — a Tai mapping preserves post-order on both sides — but
/// the greedy bijection does not, so two endpoints holding the same
/// subtrees in swapped order are credited their full mass while the
/// alignment must delete and reinsert one of them. The fallback then
/// reports an overlap the honest measure never reaches, and the rescue
/// admits a pair on it.
///
/// This is the same property `the_large_tree_fallback_never_exceeds_the_alignment`
/// asserts, on the case that separates a bijection from an alignment.
#[test]
fn the_fallback_never_credits_mass_no_ordered_alignment_can_reach() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("left.rs"));
    let right_id = registry.register(PathBuf::from("right.rs"));
    let left = parse(ALPHA_THEN_BETA, left_id)?;
    let right = parse(BETA_THEN_ALPHA, right_id)?;
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
        "the two files share both functions, so the alignment must credit real mass"
    );
    assert!(
        credited <= aligned,
        "swapped-order endpoints: the greedy fallback credited {credited} shared \
         nodes but no ordered alignment reaches more than {aligned} — the fallback \
         reports overlap the measure it stands in for cannot achieve"
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

// [FUSED-SHARED-SUBTREE] Endpoints past the alignment cap take the
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

// [FUSED-SHARED-SUBTREE] The fallback is documented as a conservative
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

// [FUSED-SHARED-SUBTREE] The cap is measured in nodes of the
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

/// Files per structure in the repeated-window fleet below: six copies
/// of each side make 36 candidate pairs that are all the same logical
/// measurement.
const FLEET_FILES_PER_STRUCTURE: usize = 6;

/// The floor every rescue admission compares against.
const ADMISSION_FLOOR: f64 = crate::pair::SHARED_SUBTREE_MIN_OVERLAP;

/// Rust source whose normalised kinds barely intersect `ACCUMULATE`'s —
/// a struct, an impl and a match instead of a loop over an accumulator
/// — and roughly twice its node mass. The shape the admission bound
/// must refuse without paying for an alignment.
const DISJOINT_KINDS: &str = "\
struct Widget {
    name: String,
    width: u32,
    height: u32,
}

impl Widget {
    fn label(&self) -> String {
        match (self.width, self.height) {
            (0, 0) => String::new(),
            (0, tall) => format!(\"tall {tall}\"),
            (wide, 0) => format!(\"wide {wide}\"),
            (wide, tall) => format!(\"{wide} by {tall} {name}\", name = self.name),
        }
    }
}
";

/// Parses `left_source` and `right_source` into two files and returns
/// their whole-file endpoint views.
fn views_of(left_source: &str, right_source: &str) -> Result<(EndpointView, EndpointView), String> {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("left.rs"));
    let right_id = registry.register(PathBuf::from("right.rs"));
    let left = parse(left_source, left_id)?;
    let right = parse(right_source, right_id)?;
    let trees = [left.tree, right.tree];
    let index = trees
        .iter()
        .map(|tree| (tree.file_id, tree))
        .collect::<std::collections::HashMap<FileId, &NormalizedNode>>();
    let left_view = build_view(&index, &left.whole).ok_or("the left endpoint resolves")?;
    let right_view = build_view(&index, &right.whole).ok_or("the right endpoint resolves")?;
    Ok((left_view, right_view))
}

// [FUSED-SHARED-SUBTREE-MEMO] The Flutter-scale blowup, captured at
// unit scale. A corpus holds many byte-offset copies of one window, and
// every cross pair of the two structures is the same logical
// measurement: Merkle hash equality pins the whole normalised
// structure, which is the exact premise the `1.0` short-circuit already
// stands on. Six copies of each side form 36 candidate pairs; the
// measurer must run one alignment and answer the other 35 from the
// memo. Keyed by byte range instead, this shape scales as copies², and
// on the Flutter corpus it reached 793,076 serial alignments without
// finishing the stage.
#[test]
fn a_fleet_of_identical_windows_costs_one_alignment() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let mut trees = Vec::new();
    let mut lefts = Vec::new();
    let mut rights = Vec::new();
    for index in 0..FLEET_FILES_PER_STRUCTURE {
        let left_id = registry.register(PathBuf::from(format!("left_{index}.rs")));
        let right_id = registry.register(PathBuf::from(format!("right_{index}.rs")));
        let left = parse(ACCUMULATE, left_id)?;
        let right = parse(AGGREGATE_WITH_INSERTION, right_id)?;
        lefts.push(left.whole);
        rights.push(right.whole);
        trees.push(left.tree);
        trees.push(right.tree);
    }
    let first_left = lefts.first().ok_or("the fleet built no left copies")?;
    let first_right = rights.first().ok_or("the fleet built no right copies")?;
    assert!(
        lefts.iter().all(|left| left.hash == first_left.hash)
            && rights.iter().all(|right| right.hash == first_right.hash)
            && first_left.hash != first_right.hash,
        "fixture guard: every copy of one source must Merkle-equal its siblings \
         across files, and the two structures must differ"
    );
    let mut measurer = OverlapMeasurer::new(&trees);
    let mut values = Vec::new();
    for left in &lefts {
        for right in &rights {
            values.push(measurer.overlap(left, right));
        }
    }
    let first = values
        .first()
        .copied()
        .ok_or("the fleet measured nothing")?;
    assert!(
        values
            .iter()
            .all(|value| (value - first).abs() < f64::EPSILON),
        "all {count} structurally identical pairs must measure the same overlap",
        count = values.len(),
    );
    assert!(
        first >= ADMISSION_FLOOR,
        "fixture guard: the fleet pair is the #408 near-miss and must clear the \
         floor, got {first}"
    );
    let stats = measurer.stats();
    let pair_count = u64::try_from(values.len()).unwrap_or(u64::MAX);
    assert_eq!(
        stats.alignments,
        1,
        "one distinct structural pair must cost exactly one alignment — \
         {pair_count} byte-range pairs collapsed by the Merkle-hash memo, \
         measured {alignments}",
        alignments = stats.alignments,
    );
    assert_eq!(
        stats.exact_hits,
        pair_count.saturating_sub(1),
        "every pair after the first must be a memo hit"
    );
    Ok(())
}

// [FUSED-SHARED-SUBTREE-BOUND] The prefilter is sound only while the
// kind-multiset bound never undercuts the alignment: an undercut would
// veto a rescue the exact measure grants — a manufactured false
// negative. Held across a genuine near-miss, a vocabulary-only match,
// and a kind-disjoint pair.
#[test]
fn the_kind_multiset_bound_never_undercuts_the_alignment() -> Result<(), String> {
    let cases = [
        (ACCUMULATE, AGGREGATE_WITH_INSERTION),
        (ACCUMULATE, UNRELATED_SAME_VOCABULARY),
        (ACCUMULATE, DISJOINT_KINDS),
        (AGGREGATE_WITH_INSERTION, UNRELATED_SAME_VOCABULARY),
    ];
    for (left_source, right_source) in cases {
        let (left_view, right_view) = views_of(left_source, right_source)?;
        let bound = kind_shared_upper_bound(&left_view, &right_view);
        let aligned = aligned_shared_nodes(&left_view, &right_view);
        assert!(
            bound >= aligned,
            "the kind-multiset bound ({bound}) must never undercut the aligned \
             shared mass ({aligned}) — an undercut would let the prefilter veto \
             a rescue the alignment grants"
        );
    }
    Ok(())
}

// [FUSED-SHARED-SUBTREE-BOUND] The other half of the capture: when the
// cheap bound already proves a pair cannot clear the floor, the
// quadratic alignment must not run at all. This is what detaches rescue
// cost from the raw candidate population.
#[test]
fn a_pair_the_bound_refuses_never_pays_for_an_alignment() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from("left.rs"));
    let right_id = registry.register(PathBuf::from("right.rs"));
    let left = parse(ACCUMULATE, left_id)?;
    let right = parse(DISJOINT_KINDS, right_id)?;
    let trees = [left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    let overlap = measurer.rescue_overlap(&left.whole, &right.whole);
    assert!(
        overlap < ADMISSION_FLOOR,
        "a kind-disjoint pair must stay under the admission floor, got {overlap}"
    );
    let stats = measurer.stats();
    assert_eq!(
        stats.alignments, 0,
        "the bound must refuse this pair before any alignment runs"
    );
    assert_eq!(
        stats.bound_skips, 1,
        "the refusal must be recorded as a bound skip"
    );
    let again = measurer.rescue_overlap(&left.whole, &right.whole);
    assert!(
        (again - overlap).abs() < f64::EPSILON && measurer.stats().bound_hits == 1,
        "a repeated refusal must come from the bound memo, not a re-walk"
    );
    Ok(())
}

// [FUSED-SHARED-SUBTREE-BOUND] The rescue path must agree with the
// exact measure on every admission decision, return exactly the exact
// value whenever the pair clears the floor, and never sit below the
// exact value (its skip answer is an upper bound).
#[test]
fn the_rescue_path_agrees_with_the_exact_measure_on_admission() -> Result<(), String> {
    let cases = [
        (ACCUMULATE, AGGREGATE_WITH_INSERTION),
        (ACCUMULATE, UNRELATED_SAME_VOCABULARY),
        (ACCUMULATE, DISJOINT_KINDS),
    ];
    for (left_source, right_source) in cases {
        let mut registry = FileRegistry::new();
        let left_id = registry.register(PathBuf::from("left.rs"));
        let right_id = registry.register(PathBuf::from("right.rs"));
        let left = parse(left_source, left_id)?;
        let right = parse(right_source, right_id)?;
        let trees = [left.tree, right.tree];
        let mut rescue_measurer = OverlapMeasurer::new(&trees);
        let mut exact_measurer = OverlapMeasurer::new(&trees);
        let rescue = rescue_measurer.rescue_overlap(&left.whole, &right.whole);
        let exact = exact_measurer.overlap(&left.whole, &right.whole);
        assert_eq!(
            rescue >= ADMISSION_FLOOR,
            exact >= ADMISSION_FLOOR,
            "the rescue path and the exact measure must make the same admission \
             decision: rescue {rescue}, exact {exact}"
        );
        assert!(
            rescue >= exact - f64::EPSILON,
            "the rescue value may only sit at or above the exact value — it is \
             an upper bound when it skips: rescue {rescue}, exact {exact}"
        );
        if exact >= ADMISSION_FLOOR {
            assert!(
                (rescue - exact).abs() < f64::EPSILON,
                "at or above the floor the rescue must return the exact value: \
                 rescue {rescue}, exact {exact}"
            );
        }
    }
    Ok(())
}

// [FUSED-SHARED-SUBTREE] Mixed-size boundary: the fallback is selected
// by the LARGER endpoint's node count, but its credit walk reads BOTH
// endpoints' creditable-entry lists. A small endpoint whose whole body
// is a subtree also nested inside the large endpoint must still be
// credited — building entries only for endpoints past
// [`ALIGNMENT_MAX_NODES`] leaves the small side empty, the credit at
// zero, and a real rescue silently dropped (review:
// docs/release-audit.md, "mixed-size overlap fallback").
#[test]
fn a_small_endpoint_still_gets_credit_against_a_large_one() -> Result<(), String> {
    // Calibrated against the Rust grammar's node yield (~7 nodes per
    // `inner = inner + n;` statement) so the block alone stays under
    // the alignment cap while the host passes it.
    const BLOCK_STATEMENTS: usize = 100;
    const HOST_STATEMENTS: usize = 15;
    const MIN_EXPECTED_SHARED_NODES: usize = 690;
    let block = boost_block(BLOCK_STATEMENTS);
    let small_source = rider_function(&block);
    let large_source = host_function(HOST_STATEMENTS, &block);
    let mut registry = FileRegistry::new();
    let small_id = registry.register(PathBuf::from("small.rs"));
    let large_id = registry.register(PathBuf::from("large.rs"));
    let small = parse(&small_source, small_id)?;
    let large = parse(&large_source, large_id)?;
    assert!(
        small.whole.node_count <= ALIGNMENT_MAX_NODES,
        "the fixture's small endpoint must stay at or under the alignment cap, got {}",
        small.whole.node_count
    );
    assert!(
        large.whole.node_count > ALIGNMENT_MAX_NODES,
        "the fixture's large endpoint must exceed the alignment cap so the pair \
         selects the fallback, got {}",
        large.whole.node_count
    );
    let trees = [small.tree, large.tree];
    let index = trees
        .iter()
        .map(|tree| (tree.file_id, tree))
        .collect::<std::collections::HashMap<FileId, &NormalizedNode>>();
    let small_view = build_view(&index, &small.whole).ok_or("the small endpoint resolves")?;
    let large_view = build_view(&index, &large.whole).ok_or("the large endpoint resolves")?;
    let credited = credit_shared_nodes(&small_view, &large_view);
    assert!(
        credited >= MIN_EXPECTED_SHARED_NODES,
        "the small endpoint's body block is nested in the large endpoint, so \
         the fallback must credit nearly all of it, got {credited}"
    );
    let mut measurer = OverlapMeasurer::new(&trees);
    let overlap = measurer.overlap(&small.whole, &large.whole);
    assert!(
        overlap >= crate::pair::SHARED_SUBTREE_MIN_OVERLAP,
        "the duplicated block is nearly all of the larger endpoint, so the pair \
         must clear the admission floor, got {overlap}"
    );
    Ok(())
}
