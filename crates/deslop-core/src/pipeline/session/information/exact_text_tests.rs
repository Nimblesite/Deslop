//! Informational eligibility when shape equality hides different authored spans.

use std::collections::HashMap;

use super::*;
use crate::{
    ast::ByteRange,
    buckets::CONTENT_SUPPORT_FLOOR,
    config::RoutingTuning,
    content::{ContentMeasurer, PairScope},
    fingerprint::{subtree_hash, HashScratch},
    lang::shared::IDENTIFIER_KIND,
    state::FileRegistry,
};

const SOURCE: &[u8] = b"abcd";
const CHANGED_SOURCE: &[u8] = b"wxyz";
const LEFT_PATH: &str = "left.rs";
const RIGHT_PATH: &str = "right.rs";
const LANGUAGE: &str = "rust";
const OTHER_LANGUAGE: &str = "typescript";
const ROOT_KIND: &str = "block";
const START: usize = 0;
const LEFT_DIVIDE: usize = 2;
const SHIFTED_DIVIDE: usize = 1;
const FAMILY_MEMBERS: &[usize] = &[0, 1];
const NO_INFORMATION: usize = 0;
const ONE_INFORMATION: usize = 1;

struct TextFixture {
    fingerprints: [Fingerprint; 2],
    trees: [NormalizedNode; 2],
    sources: HashMap<FileId, Vec<u8>>,
    languages: HashMap<FileId, &'static str>,
}

fn tree(file_id: FileId, divide: usize) -> NormalizedNode {
    let range = ByteRange {
        start: START,
        end: SOURCE.len(),
    };
    let leaf = |start, end| NormalizedNode {
        kind: IDENTIFIER_KIND,
        children: Vec::new(),
        byte_range: ByteRange { start, end },
        file_id,
    };
    NormalizedNode {
        kind: ROOT_KIND,
        children: vec![leaf(START, divide), leaf(divide, SOURCE.len())],
        byte_range: range,
        file_id,
    }
}

fn fingerprint(tree: &NormalizedNode) -> Fingerprint {
    Fingerprint {
        hash: subtree_hash(tree, &mut HashScratch::default()),
        file_id: tree.file_id,
        byte_range: tree.byte_range,
        node_count: tree.subtree_node_count(),
    }
}

fn fixture(right_source: &[u8], right_language: &'static str, divide: usize) -> TextFixture {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(LEFT_PATH.into());
    let right_id = registry.register(RIGHT_PATH.into());
    let trees = [tree(left_id, LEFT_DIVIDE), tree(right_id, divide)];
    let fingerprints = trees.each_ref().map(fingerprint);
    TextFixture {
        fingerprints,
        trees,
        sources: HashMap::from([
            (left_id, SOURCE.to_vec()),
            (right_id, right_source.to_vec()),
        ]),
        languages: HashMap::from([(left_id, LANGUAGE), (right_id, right_language)]),
    }
}

fn family() -> FusedCluster {
    FusedCluster {
        members: FAMILY_MEMBERS.to_vec(),
        edges: Vec::new(),
        shape_family: None,
    }
}

fn eligible(text: &TextFixture) -> Vec<FusedCluster> {
    eligible_information(
        &[family()],
        &text.fingerprints,
        &text.trees,
        &text.sources,
        &text.languages,
        RoutingTuning::default().nearly_identical_min_shape,
    )
}

/// [CLONE-BUCKETS-STRUCTURAL-ONLY-COUNT-BOUND] Proven identical
/// authored frontiers have no informational relation to publish.
#[test]
fn informational_eligibility_skips_exact_text_with_equal_shape() {
    let text = fixture(SOURCE, LANGUAGE, LEFT_DIVIDE);
    assert_eq!(eligible(&text).len(), NO_INFORMATION);
}

/// [CLONE-BUCKETS-STRUCTURAL-ONLY-COUNT-BOUND] Changed authored text
/// can still produce a Type-III informational relation.
#[test]
fn informational_eligibility_retains_changed_text_with_equal_shape() {
    let text = fixture(CHANGED_SOURCE, LANGUAGE, LEFT_DIVIDE);
    let retained = eligible(&text);
    assert_eq!(retained.len(), ONE_INFORMATION);
    assert_eq!(
        retained.first().map(|found| found.members.as_slice()),
        Some(FAMILY_MEMBERS)
    );
}

#[test]
fn informational_eligibility_retains_cross_language_text() {
    let text = fixture(SOURCE, OTHER_LANGUAGE, LEFT_DIVIDE);
    let retained = eligible(&text);
    assert_eq!(retained.len(), ONE_INFORMATION);
    assert_eq!(
        retained.first().map(|found| found.members.as_slice()),
        Some(FAMILY_MEMBERS)
    );
}

/// [CLONE-BUCKETS-STRUCTURAL-ONLY-COUNT-BOUND] Equal whole text cannot
/// erase a possible Type-III pair when its authored leaves disagree.
#[test]
fn equal_hash_and_text_with_shifted_leaves_remains_informational() {
    let text = fixture(SOURCE, LANGUAGE, SHIFTED_DIVIDE);
    let [left, right] = &text.fingerprints;
    assert_eq!(left.hash, right.hash);
    let trees: HashMap<_, _> = text.trees.iter().map(|tree| (tree.file_id, tree)).collect();
    let mut measurer = ContentMeasurer::default();
    let pair = measurer.pair((left, right), &trees, &text.sources, &text.languages);
    let scope = PairScope {
        same_file: false,
        interior: false,
        core: false,
    };
    assert!(pair.resolved());
    assert!(pair.whole(&text.sources, scope).support() < CONTENT_SUPPORT_FLOOR);
    assert_eq!(eligible(&text).len(), ONE_INFORMATION);
}
