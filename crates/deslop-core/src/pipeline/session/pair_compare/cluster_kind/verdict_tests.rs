//! Memoised ordered-pair verdicts and the exact-text shortcut ([CLONE-KIND-FOLD]).

use super::*;
use crate::{
    ast::ByteRange,
    state::{FileId, FileRegistry},
};

const LEFT: usize = 7;
const RIGHT: usize = 11;
const COLD_LEFT: usize = 8;
const RETAINED_AFTER_ROTATION: usize = 2;
const EXACT_SOURCE: &[u8] = b"fn total() { accept(); }";
const CHANGED_SOURCE: &[u8] = b"fn total() { reject(); }";
const LEFT_PATH: &str = "left.rs";
const RIGHT_PATH: &str = "right.rs";
const LANGUAGE: &str = "rust";
const OTHER_LANGUAGE: &str = "typescript";
const EXACT_HASH: [u8; 32] = [1; 32];
const DIFFERENT_HASH: [u8; 32] = [2; 32];
const RANGE_START: usize = 0;
const NODE_COUNT: usize = 40;

struct ExactFixture {
    left: Fingerprint,
    right: Fingerprint,
    sources: HashMap<FileId, Vec<u8>>,
    languages: HashMap<FileId, &'static str>,
}

fn exact_fingerprint(file_id: FileId) -> Fingerprint {
    Fingerprint {
        hash: EXACT_HASH,
        file_id,
        byte_range: ByteRange {
            start: RANGE_START,
            end: EXACT_SOURCE.len(),
        },
        node_count: NODE_COUNT,
    }
}

impl ExactFixture {
    fn new() -> Self {
        let mut registry = FileRegistry::new();
        let left_id = registry.register(LEFT_PATH.into());
        let right_id = registry.register(RIGHT_PATH.into());
        Self {
            left: exact_fingerprint(left_id),
            right: exact_fingerprint(right_id),
            sources: HashMap::from([
                (left_id, EXACT_SOURCE.to_vec()),
                (right_id, EXACT_SOURCE.to_vec()),
            ]),
            languages: HashMap::from([(left_id, LANGUAGE), (right_id, LANGUAGE)]),
        }
    }
}

/// [CLONE-KIND-FOLD] Matching bytes and Merkle shape need no pair algebra.
#[test]
fn exact_text_verdict_requires_resolved_content() {
    let fixture = ExactFixture::new();
    let classify = |resolved| {
        exact_text_kind(
            &fixture.left,
            &fixture.right,
            &fixture.sources,
            &fixture.languages,
            || resolved,
        )
    };
    assert_eq!(classify(true), Some(ClusterKind::Identical));
    assert_eq!(classify(false), None);
}

#[test]
fn exact_text_verdict_ignores_changed_bytes_shape_and_language() {
    let mut fixture = ExactFixture::new();
    let classify = |fixture: &ExactFixture| {
        exact_text_kind(
            &fixture.left,
            &fixture.right,
            &fixture.sources,
            &fixture.languages,
            || true,
        )
    };
    let _previous = fixture
        .sources
        .insert(fixture.right.file_id, CHANGED_SOURCE.to_vec());
    assert_eq!(classify(&fixture), None);
    let _previous = fixture
        .sources
        .insert(fixture.right.file_id, EXACT_SOURCE.to_vec());
    fixture.right.hash = DIFFERENT_HASH;
    assert_eq!(classify(&fixture), None);
    fixture.right.hash = EXACT_HASH;
    let _previous = fixture
        .languages
        .insert(fixture.right.file_id, OTHER_LANGUAGE);
    assert_eq!(classify(&fixture), None);
}

/// [CLONE-KIND-FOLD] Repeated pair grading reuses the exact ordered verdict.
#[test]
fn ordered_pair_verdicts_include_rejection_and_do_not_flip_direction() {
    let mut cache = PairKindMemo::default();
    assert_eq!(cache.get((LEFT, RIGHT)), PairKindLookup::Unseen);
    cache.remember((LEFT, RIGHT), Some(ClusterKind::NearlyIdentical));
    assert_eq!(
        cache.get((LEFT, RIGHT)),
        PairKindLookup::Seen(Some(ClusterKind::NearlyIdentical))
    );
    assert_eq!(cache.get((RIGHT, LEFT)), PairKindLookup::Unseen);
    cache.remember((RIGHT, LEFT), None);
    assert_eq!(cache.get((RIGHT, LEFT)), PairKindLookup::Seen(None));
}

/// [CLONE-KIND-FOLD] A full memo admits later keys without growing forever.
#[test]
fn pair_kind_memo_rotates_after_capacity() {
    let mut cache = PairKindMemo::default();
    for left in 0..PAIR_KIND_MEMO_MAX {
        cache.remember((left, RIGHT), Some(ClusterKind::Identical));
    }
    cache.remember(
        (PAIR_KIND_MEMO_MAX, RIGHT),
        Some(ClusterKind::NearlyIdentical),
    );
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get((LEFT, RIGHT)), PairKindLookup::Unseen);
    assert_eq!(
        cache.get((PAIR_KIND_MEMO_MAX, RIGHT)),
        PairKindLookup::Seen(Some(ClusterKind::NearlyIdentical))
    );
}

/// [CLONE-KIND-FOLD] A reused verdict survives rotation without retaining cold pairs.
#[test]
fn pair_kind_memo_keeps_reused_verdicts_across_rotation() {
    let mut cache = PairKindMemo::default();
    for left in 0..PAIR_KIND_MEMO_MAX {
        cache.remember((left, RIGHT), Some(ClusterKind::Identical));
    }
    assert_eq!(
        cache.get((LEFT, RIGHT)),
        PairKindLookup::Seen(Some(ClusterKind::Identical))
    );
    cache.remember(
        (PAIR_KIND_MEMO_MAX, RIGHT),
        Some(ClusterKind::NearlyIdentical),
    );
    assert_eq!(
        cache.get((LEFT, RIGHT)),
        PairKindLookup::Seen(Some(ClusterKind::Identical))
    );
    assert_eq!(cache.get((COLD_LEFT, RIGHT)), PairKindLookup::Unseen);
    assert_eq!(cache.len(), RETAINED_AFTER_ROTATION);
}
