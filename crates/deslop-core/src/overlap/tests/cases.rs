//! Alignment and large-tree fallback cases.

use super::*;

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
    let (left_view, right_view) = views_of(ALPHA_THEN_BETA, BETA_THEN_ALPHA)?;
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
    let (left_id, right_id) = pair_ids(WIDE_LEFT_RS, WIDE_RIGHT_RS);
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
    let (left, right) = parse_pair(&left_source, &right_source)?;
    assert!(
        right.whole.node_count > ALIGNMENT_MAX_NODES,
        "the fixture must exceed the alignment cap so the E2E path takes the \
         fallback for this pair, got {}",
        right.whole.node_count
    );
    let trees = [left.tree, right.tree];
    let (left_view, right_view) = endpoint_views(&trees, &left.whole, &right.whole)?;
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
