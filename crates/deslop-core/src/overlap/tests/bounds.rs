//! Bounds and memo reuse cases.

use super::*;

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
    let (left_id, right_id) = rust_pair_ids();
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
        let (left_id, right_id) = rust_pair_ids();
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

// [FUSED-SHARED-SUBTREE-BOUND] The kind fold may use a higher configured
// information floor when neither tokens nor embeddings can admit a pair.
// Its answer must preserve that floor decision on every source pair.
#[test]
fn the_kind_floor_bound_preserves_exact_classification() -> Result<(), String> {
    let floor = crate::config::RoutingTuning::default().nearly_identical_min_shape;
    let cases = [
        (ACCUMULATE, AGGREGATE_WITH_INSERTION),
        (ACCUMULATE, UNRELATED_SAME_VOCABULARY),
        (ACCUMULATE, DISJOINT_KINDS),
    ];
    for (left_source, right_source) in cases {
        let (left, right) = parse_pair(left_source, right_source)?;
        let trees = [left.tree, right.tree];
        let bounded =
            OverlapMeasurer::new(&trees).bounded_overlap(&left.whole, &right.whole, floor);
        let exact = OverlapMeasurer::new(&trees).overlap(&left.whole, &right.whole);
        assert_eq!(bounded >= floor, exact >= floor, "{bounded} vs {exact}");
        assert!(bounded >= exact, "a bound must not undercut exact overlap");
        if exact >= floor {
            assert_eq!(
                bounded.to_bits(),
                exact.to_bits(),
                "above-floor overlap must be exact"
            );
        }
    }
    Ok(())
}

// [FUSED-SHARED-SUBTREE-BOUND] The fingerprint already knows each
// endpoint's node count. A short and long range cannot share enough nodes
// to reach the kind floor, so no endpoint views should be built at all.
#[test]
fn node_count_bound_refuses_before_building_views() -> Result<(), String> {
    const TINY_BINDING: &str = "pub fn tiny() { let value = 1; }";
    let (left, right) = parse_pair(ACCUMULATE, TINY_BINDING)?;
    let trees = [left.tree, right.tree];
    let floor = crate::config::RoutingTuning::default().nearly_identical_min_shape;
    let mut measurer = OverlapMeasurer::new(&trees);
    let bound = measurer.bounded_overlap(&left.whole, &right.whole, floor);
    assert!(bound < floor, "unequal sizes cannot clear {floor}: {bound}");
    assert!(
        measurer.endpoints.is_empty(),
        "the count bound must decide before expensive endpoint views"
    );
    Ok(())
}

// [FUSED-SHARED-SUBTREE-BOUND] The pre-view bound is sound only when
// emitted fingerprint counts equal resolved view counts, including the
// synthetic sibling windows that dominate real corpus scans.
#[test]
fn fingerprint_counts_match_resolved_views() -> Result<(), String> {
    const MIN_NODES: usize = 1;
    let source = format!("{ACCUMULATE}{AGGREGATE_WITH_INSERTION}");
    let parsed = parse(&source, rust_pair_ids().0)?;
    let trees = [parsed.tree];
    let siblings = crate::sibling::collect_sibling_fingerprints(&trees[0], MIN_NODES);
    assert!(
        !siblings.is_empty(),
        "fixture must exercise sibling windows"
    );
    let fingerprints = collect_fingerprints(&trees[0], MIN_NODES)
        .into_iter()
        .chain(siblings);
    let index = std::collections::HashMap::from([(trees[0].file_id, &trees[0])]);
    for fingerprint in fingerprints {
        let view = build_view(&index, &fingerprint)
            .ok_or_else(|| format!("emitted fingerprint must resolve: {fingerprint:?}"))?;
        assert_eq!(view.total, fingerprint.node_count, "{fingerprint:?}");
        let tokens = crate::tokens::token_stream_for_fingerprint(&trees[0], &fingerprint)
            .ok_or_else(|| format!("emitted fingerprint must tokenize: {fingerprint:?}"))?;
        assert_eq!(tokens.len(), fingerprint.node_count, "{fingerprint:?}");
    }
    Ok(())
}

// [FUSED-SHARED-SUBTREE-MEMO] Grammar wrappers can share byte ranges with
// their child. The endpoint memo must not reuse one view for the other.
#[test]
fn same_range_fingerprints_keep_distinct_overlap_views() -> Result<(), String> {
    const EVERY_SUBTREE: usize = 1;
    let source = format!("{ACCUMULATE}{AGGREGATE_WITH_INSERTION}");
    let parsed = parse(&source, rust_pair_ids().0)?;
    let fingerprints = collect_fingerprints(&parsed.tree, EVERY_SUBTREE);
    let pair = same_range_pair(&fingerprints)?;
    for (first, second) in [pair, (pair.1, pair.0)] {
        let mut measurer = OverlapMeasurer::new(std::slice::from_ref(&parsed.tree));
        let first_view = measurer.view(first).ok_or("first view resolves")?;
        let second_view = measurer.view(second).ok_or("second view resolves")?;
        assert_eq!(first_view.total(), first.node_count);
        assert_eq!(second_view.total(), second.node_count);
    }
    Ok(())
}

fn same_range_pair(fingerprints: &[Fingerprint]) -> Result<(&Fingerprint, &Fingerprint), String> {
    fingerprints
        .iter()
        .find_map(|first| {
            fingerprints
                .iter()
                .find(|second| {
                    first.byte_range == second.byte_range && first.node_count != second.node_count
                })
                .map(|second| (first, second))
        })
        .ok_or_else(|| "fixture must emit different same-range nodes".into())
}

const MEMO_LEAF_NODES: usize = 1;

fn memo_leaf(index: usize, file_id: FileId) -> NormalizedNode {
    NormalizedNode {
        kind: "item",
        children: Vec::new(),
        byte_range: ByteRange {
            start: index,
            end: index.saturating_add(MEMO_LEAF_NODES),
        },
        file_id,
    }
}

fn memo_tree(file_id: FileId) -> NormalizedNode {
    let leaf_count = ENDPOINT_VIEW_MEMO_MAX + MEMO_LEAF_NODES;
    NormalizedNode {
        kind: "source_file",
        children: (0..leaf_count)
            .map(|index| memo_leaf(index, file_id))
            .collect(),
        byte_range: ByteRange {
            start: 0,
            end: leaf_count,
        },
        file_id,
    }
}

fn fill_memo(
    measurer: &mut OverlapMeasurer<'_>,
    fingerprints: &[Fingerprint],
) -> Result<(), String> {
    for fingerprint in fingerprints
        .iter()
        .skip(MEMO_LEAF_NODES)
        .take(ENDPOINT_VIEW_MEMO_MAX)
    {
        let _view = measurer
            .view(fingerprint)
            .ok_or("earlier endpoint resolves")?;
    }
    Ok(())
}

// [FUSED-SHARED-SUBTREE-MEMO] Later recovery families must still reuse
// their endpoint views after an earlier family fills the bounded memo.
#[test]
fn endpoint_views_remain_reusable_after_the_memo_fills() -> Result<(), String> {
    let tree = memo_tree(rust_pair_ids().0);
    let fingerprints = collect_fingerprints(&tree, MEMO_LEAF_NODES);
    let mut measurer = OverlapMeasurer::new(std::slice::from_ref(&tree));
    fill_memo(&mut measurer, &fingerprints)?;
    let last = fingerprints.last().ok_or("later endpoint exists")?;
    let first = measurer.view(last).ok_or("later endpoint resolves")?;
    let second = measurer.view(last).ok_or("later endpoint resolves again")?;
    assert!(
        Arc::ptr_eq(&first, &second),
        "later endpoints must be cached"
    );
    Ok(())
}

fn fill_exact_memo(measurer: &mut OverlapMeasurer<'_>) {
    const DUMMY_OVERLAP: f64 = 0.0;
    const DUMMY_PARTNER: [u8; 32] = [0; 32];
    for index in 0..EXACT_RESULT_MEMO_MAX {
        if measurer.exact_results.len() == EXACT_RESULT_MEMO_MAX {
            break;
        }
        let hash = blake3::hash(&index.to_le_bytes()).into();
        let _previous = measurer
            .exact_results
            .insert((hash, DUMMY_PARTNER), DUMMY_OVERLAP);
    }
}

// [FUSED-SHARED-SUBTREE-MEMO] A later structural pair still reuses its
// exact alignment once an earlier population has filled the bounded memo.
#[test]
fn exact_results_remain_cached_after_the_memo_fills() -> Result<(), String> {
    const ONE_ALIGNMENT: u64 = 1;
    const ONE_HIT: u64 = 1;
    let (left, right) = parse_pair(ACCUMULATE, AGGREGATE_WITH_INSERTION)?;
    let trees = [left.tree, right.tree];
    let mut measurer = OverlapMeasurer::new(&trees);
    fill_exact_memo(&mut measurer);
    let first = measurer.overlap(&left.whole, &right.whole);
    let second = measurer.overlap(&left.whole, &right.whole);
    assert_eq!(
        first.to_bits(),
        second.to_bits(),
        "memo preserves exact overlap"
    );
    assert_eq!(measurer.stats().alignments, ONE_ALIGNMENT);
    assert_eq!(measurer.stats().exact_hits, ONE_HIT);
    Ok(())
}

mod rotation;

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
    let (small, large) = parse_pair(&small_source, &large_source)?;
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
    let (small_view, large_view) = endpoint_views(&trees, &small.whole, &large.whole)?;
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
