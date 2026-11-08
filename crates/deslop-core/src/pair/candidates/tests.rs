//! [FUSED-CANDIDATE-BUCKET-STAR] The structural pass pairs a bucket as a
//! star, inside one file as across files. A file holding `n` identically
//! shaped declarations once produced `n(n-1)/2` candidates (gh #506): a
//! generated parser table of 1,353 same-shape windows cost 915k pairs
//! and a 2,290 s content gate. Copy recovery, not enumeration, now adds
//! the pairs an unrelated first member hides.

use super::{candidate_pairs, CandidatePair};
use crate::{
    ast::ByteRange,
    fingerprint::Fingerprint,
    lsh::{SignatureIndex, ZEROED_SIGNATURE},
    state::{FileId, FileRegistry},
};

/// A bucket large enough that quadratic pairing is unmistakable.
const SAME_FILE_MEMBERS: usize = 400;
/// Every member of the fixture bucket is Merkle-equal.
const SHARED_HASH: [u8; 32] = [7; 32];
/// Each member spans this many bytes, laid out back to back.
const MEMBER_BYTES: usize = 64;
/// Clears every admission floor on its own.
const NODE_COUNT: usize = 40;
/// Members of the two-file bucket, in index order: the first two share a
/// file and the third sits in another.
const SPANNING_FILES: [usize; 3] = [0, 0, 1];
/// The spanning bucket's star plus the cross-file pair every same-file
/// member is owed: `(0,1)`, `(0,2)` and `(1,2)`.
const SPANNING_PAIRS: [(usize, usize); 3] = [(0, 1), (0, 2), (1, 2)];

fn member(file_id: FileId, position: usize) -> Fingerprint {
    let start = position.saturating_mul(MEMBER_BYTES);
    Fingerprint {
        hash: SHARED_HASH,
        file_id,
        byte_range: ByteRange {
            start,
            end: start.saturating_add(MEMBER_BYTES),
        },
        node_count: NODE_COUNT,
    }
}

fn structural_pairs(fingerprints: &[Fingerprint]) -> Vec<(usize, usize)> {
    let signatures = vec![ZEROED_SIGNATURE; fingerprints.len()];
    let index = SignatureIndex::from_segments([signatures.as_slice()]);
    let no_lsh_pairs: Vec<(usize, usize)> = Vec::new();
    candidate_pairs(fingerprints, &index, &no_lsh_pairs, &[])
        .iter()
        .map(|pair: &CandidatePair| (pair.left, pair.right))
        .collect()
}

#[test]
fn a_same_file_bucket_is_a_star_not_a_clique() {
    let mut registry = FileRegistry::new();
    let file = registry.register("generated.cs".into());
    let fingerprints: Vec<Fingerprint> = (0..SAME_FILE_MEMBERS)
        .map(|position| member(file, position))
        .collect();
    let pairs = structural_pairs(&fingerprints);
    let expected: Vec<(usize, usize)> = (1..SAME_FILE_MEMBERS).map(|other| (0, other)).collect();
    assert_eq!(
        pairs, expected,
        "one file of {SAME_FILE_MEMBERS} same-shape members must pair as a star from the first member"
    );
}

#[test]
fn a_bucket_spanning_files_owes_every_member_a_cross_file_pair() {
    let mut registry = FileRegistry::new();
    let files = [
        registry.register("left.cs".into()),
        registry.register("right.cs".into()),
    ];
    let fingerprints: Vec<Fingerprint> = SPANNING_FILES
        .iter()
        .enumerate()
        .filter_map(|(position, file)| files.get(*file).map(|file| member(*file, position)))
        .collect();
    assert_eq!(structural_pairs(&fingerprints), SPANNING_PAIRS.to_vec());
}
