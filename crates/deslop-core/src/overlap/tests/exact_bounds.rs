//! Exact overlap certificates and their credited-mass preconditions.

use super::*;

/// Zero-width grammar wrappers may share one byte position.
const EMPTY_SPAN: ByteRange = ByteRange { start: 0, end: 0 };
const LEFT_ROOT: &str = "left_root";
const RIGHT_ROOT: &str = "right_root";
const SHARED_KINDS: [&str; 4] = ["outer", "middle", "inner", "leaf"];
const SHARED_NODES: usize = 4;
const SHARED_SPAN: ByteRange = ByteRange { start: 1, end: 10 };
const SHELL_SPAN: ByteRange = ByteRange { start: 0, end: 11 };
const SHELL_KIND: &str = "shell";
const ONE_ALIGNMENT: u64 = 1;
const NO_ALIGNMENTS: u64 = 0;
const WRAPPED_OVERLAP: f64 = 0.8;

/// One node in a zero-width wrapper chain.
fn wrapper(
    kind: &'static str,
    child: Option<NormalizedNode>,
    file_id: FileId,
    span: ByteRange,
) -> NormalizedNode {
    NormalizedNode {
        kind,
        children: child.into_iter().collect(),
        byte_range: span,
        file_id,
    }
}

/// Four nested nodes with one shape and an arbitrary source extent.
fn shared_tree(file_id: FileId, span: ByteRange) -> NormalizedNode {
    let [outer, middle, inner, leaf] = SHARED_KINDS;
    let leaf = wrapper(leaf, None, file_id, span);
    let inner = wrapper(inner, Some(leaf), file_id, span);
    let middle = wrapper(middle, Some(inner), file_id, span);
    wrapper(outer, Some(middle), file_id, span)
}

/// A changed root over the same four-node zero-width subtree.
fn zero_width_tree(root_kind: &'static str, file_id: FileId) -> NormalizedNode {
    wrapper(
        root_kind,
        Some(shared_tree(file_id, EMPTY_SPAN)),
        file_id,
        EMPTY_SPAN,
    )
}

/// The root's exact fingerprint, independent of ambiguous byte range.
fn root_fingerprint(tree: &NormalizedNode) -> Fingerprint {
    Fingerprint {
        hash: subtree_hash(tree, &mut HashScratch::default()),
        file_id: tree.file_id,
        byte_range: tree.byte_range,
        node_count: tree.subtree_node_count(),
    }
}

// [FUSED-SHARED-SUBTREE-CREDIT-DISJOINT] Byte-position cursors alone
// cannot prove two credited nested subtrees are disjoint when both spans
// have zero width. The fallback must not credit the same nodes twice.
#[test]
fn zero_width_nested_subtrees_are_not_credited_twice() -> Result<(), String> {
    let (left_id, right_id) = rust_pair_ids();
    let left = zero_width_tree(LEFT_ROOT, left_id);
    let right = zero_width_tree(RIGHT_ROOT, right_id);
    let left_fingerprint = root_fingerprint(&left);
    let right_fingerprint = root_fingerprint(&right);
    let trees = [left, right];
    let (left_view, right_view) = endpoint_views(&trees, &left_fingerprint, &right_fingerprint)?;
    let aligned = aligned_shared_nodes(&left_view, &right_view);
    let credited = credit_shared_nodes(&left_view, &right_view);
    assert_eq!(
        aligned, SHARED_NODES,
        "only the shared wrapper chain aligns"
    );
    assert!(
        credited <= aligned,
        "credit {credited} exceeds exact {aligned}"
    );
    Ok(())
}

// [FUSED-SHARED-SUBTREE-BOUND-EXACT] A complete subtree of the larger
// endpoint supplies a lower bound equal to the ordered upper bound.
// That equality proves the exact score without invoking alignment.
#[test]
fn matching_overlap_bounds_skip_exact_alignment() -> Result<(), String> {
    let (left_id, right_id) = rust_pair_ids();
    let left = shared_tree(left_id, SHARED_SPAN);
    let right = wrapper(
        SHELL_KIND,
        Some(shared_tree(right_id, SHARED_SPAN)),
        right_id,
        SHELL_SPAN,
    );
    let left_fingerprint = root_fingerprint(&left);
    let right_fingerprint = root_fingerprint(&right);
    let trees = [left, right];
    let (left_view, right_view) = endpoint_views(&trees, &left_fingerprint, &right_fingerprint)?;
    assert_eq!(aligned_shared_nodes(&left_view, &right_view), SHARED_NODES);
    let mut measurer = OverlapMeasurer::new(&trees);
    let overlap = measurer.rescue_overlap(&left_fingerprint, &right_fingerprint);
    assert_eq!(overlap.to_bits(), WRAPPED_OVERLAP.to_bits());
    assert_eq!(measurer.stats().alignments, NO_ALIGNMENTS);
    Ok(())
}

// [FUSED-SHARED-SUBTREE-BOUND-EXACT] A gap between the two bounds
// cannot certify an exact score and still needs ordered alignment.
#[test]
fn differing_overlap_bounds_keep_exact_alignment() {
    let (left_id, right_id) = rust_pair_ids();
    let left = zero_width_tree(LEFT_ROOT, left_id);
    let right = zero_width_tree(RIGHT_ROOT, right_id);
    let left_fingerprint = root_fingerprint(&left);
    let right_fingerprint = root_fingerprint(&right);
    let trees = [left, right];
    let mut measurer = OverlapMeasurer::new(&trees);
    let overlap = measurer.rescue_overlap(&left_fingerprint, &right_fingerprint);
    assert_eq!(overlap.to_bits(), WRAPPED_OVERLAP.to_bits());
    assert_eq!(measurer.stats().alignments, ONE_ALIGNMENT);
}
