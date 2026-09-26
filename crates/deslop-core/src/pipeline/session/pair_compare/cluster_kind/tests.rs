//! Exact-byte shortcut controls where the normalised tree omits source spans.

use std::collections::HashMap;

use super::exact_text_kind;
use crate::{
    ast::{ByteRange, NormalizedNode},
    buckets::CONTENT_SUPPORT_FLOOR,
    content::{ContentMeasurer, PairScope},
    fingerprint::{subtree_hash, Fingerprint, HashScratch},
    lang::shared::IDENTIFIER_KIND,
    state::{FileId, FileRegistry},
};

const SOURCE: &[u8] = b"abcd";
const LEFT_PATH: &str = "left.rs";
const RIGHT_PATH: &str = "right.rs";
const LANGUAGE: &str = "rust";
const ROOT_KIND: &str = "block";
const START: usize = 0;
const LEFT_DIVIDE: usize = 2;
const RIGHT_DIVIDE: usize = 1;

fn node(
    kind: &'static str,
    file_id: FileId,
    range: ByteRange,
    children: Vec<NormalizedNode>,
) -> NormalizedNode {
    NormalizedNode {
        kind,
        children,
        byte_range: range,
        file_id,
    }
}

fn partitioned_tree(file_id: FileId, divide: usize) -> NormalizedNode {
    let range = ByteRange {
        start: START,
        end: SOURCE.len(),
    };
    let first = node(
        IDENTIFIER_KIND,
        file_id,
        ByteRange {
            start: START,
            end: divide,
        },
        Vec::new(),
    );
    let second = node(
        IDENTIFIER_KIND,
        file_id,
        ByteRange {
            start: divide,
            end: SOURCE.len(),
        },
        Vec::new(),
    );
    node(ROOT_KIND, file_id, range, vec![first, second])
}

fn fingerprint(tree: &NormalizedNode) -> Fingerprint {
    Fingerprint {
        hash: subtree_hash(tree, &mut HashScratch::default()),
        file_id: tree.file_id,
        byte_range: tree.byte_range,
        node_count: tree.subtree_node_count(),
    }
}

/// [CLONE-KIND-FOLD] Matching whole bytes and shape do not prove the
/// authored leaves agree when their byte spans partition differently.
#[test]
fn equal_hash_and_bytes_can_have_disagreeing_content_frontiers() {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(LEFT_PATH.into());
    let right_id = registry.register(RIGHT_PATH.into());
    let left_tree = partitioned_tree(left_id, LEFT_DIVIDE);
    let right_tree = partitioned_tree(right_id, RIGHT_DIVIDE);
    let left = fingerprint(&left_tree);
    let right = fingerprint(&right_tree);
    assert_eq!(left.hash, right.hash);
    let trees = HashMap::from([(left_id, &left_tree), (right_id, &right_tree)]);
    let sources = HashMap::from([(left_id, SOURCE.to_vec()), (right_id, SOURCE.to_vec())]);
    let languages = HashMap::from([(left_id, LANGUAGE), (right_id, LANGUAGE)]);
    let mut measurer = ContentMeasurer::default();
    let pair = measurer.pair((&left, &right), &trees, &sources, &languages);
    let scope = PairScope {
        same_file: false,
        interior: false,
        core: false,
    };
    let evidence = pair.whole(&sources, scope);
    assert!(pair.resolved(), "both source frontiers resolve");
    assert!(
        !pair.same_frontier_keys(),
        "authored leaf boundaries differ"
    );
    assert!(
        evidence.support() < CONTENT_SUPPORT_FLOOR,
        "authored leaf content differs"
    );
    assert_eq!(
        exact_text_kind(&left, &right, &sources, &languages, || pair
            .exact_frontier_match(&sources)),
        None
    );
}
