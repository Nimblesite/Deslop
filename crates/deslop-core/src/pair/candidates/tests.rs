//! [FUSED-CANDIDATE-BUCKET-STAR] The structural pass pairs a bucket as a
//! star, inside one file as across files. A file holding `n` identically
//! shaped declarations once produced `n(n-1)/2` candidates (gh #506): a
//! generated parser table of 1,353 same-shape windows cost 915k pairs
//! and a 2,290 s content gate. Copy recovery, not enumeration, now adds
//! the pairs an unrelated first member hides.

use std::collections::HashMap;

use super::{candidate_pairs, candidate_pairs_for_language_policy, CandidatePair, LshPairs};
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
/// Complete structural evidence, as a Merkle-equal pair scores.
const STRUCTURAL_EVIDENCE: f64 = 1.0;
/// Members of the two-file bucket, in index order: the first two share a
/// file and the third sits in another.
const SPANNING_FILES: [usize; 3] = [0, 0, 1];
/// The spanning bucket's star plus the cross-file pair every same-file
/// member is owed: `(0,1)`, `(0,2)` and `(1,2)`.
const SPANNING_PAIRS: [(usize, usize); 3] = [(0, 1), (0, 2), (1, 2)];
/// A bucket whose first member is written in another language: one C
/// file, then three windows of one Python file.
const LANGUAGE_FILES: [(&str, &str); 2] = [("table.c", "c"), ("rows.py", "python")];
/// Members in index order: the C window first, then the Python windows.
const LANGUAGE_MEMBERS: [usize; 4] = [0, 1, 1, 1];
/// The Python star, drawn from the first Python member; the C window has
/// no admissible edge and is nobody's centre.
const PYTHON_STAR: [(usize, usize); 2] = [(1, 2), (1, 3)];
/// Cross-language comparison is off by default ([CONFIG-CROSS-LANGUAGE]).
const CROSS_LANGUAGE_OFF: bool = false;

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
    let no_lsh_pairs: Vec<(usize, usize)> = Vec::new();
    pairs_with_lsh(fingerprints, &no_lsh_pairs)
        .iter()
        .map(|pair| (pair.left, pair.right))
        .collect()
}

fn pairs_with_lsh(fingerprints: &[Fingerprint], lsh_pairs: &dyn LshPairs) -> Vec<CandidatePair> {
    let signatures = vec![ZEROED_SIGNATURE; fingerprints.len()];
    let index = SignatureIndex::from_segments([signatures.as_slice()]);
    candidate_pairs(fingerprints, &index, lsh_pairs, &[])
}

/// [FUSED-CANDIDATE-BUCKET-STAR] A band collision between two members of
/// one bucket that the star never paired — the second and third of a
/// three-member same-file bucket — is a structural pair, not an LSH-only
/// one: Merkle-equal endpoints are complete structural evidence.
#[test]
fn a_band_collision_between_merkle_equal_members_carries_structural_evidence() {
    let mut registry = FileRegistry::new();
    let file = registry.register("generated.cs".into());
    let fingerprints: Vec<Fingerprint> = (0..3).map(|position| member(file, position)).collect();
    let collided: Vec<(usize, usize)> = vec![(1, 2)];
    let pairs = pairs_with_lsh(&fingerprints, &collided);
    let keys: Vec<(usize, usize)> = pairs.iter().map(|pair| (pair.left, pair.right)).collect();
    assert_eq!(keys, vec![(0, 1), (0, 2), (1, 2)]);
    assert!(
        pairs
            .iter()
            .all(|pair| pair.score.structural == STRUCTURAL_EVIDENCE),
        "every Merkle-equal pair scores full structural evidence: {pairs:?}"
    );
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

/// [FUSED-CANDIDATE-BUCKET-STAR] A bucket spanning languages is one star
/// per language when cross-language comparison is off: a centre in
/// another language would be refused on every edge, and the members of
/// each remaining language would never be paired at all.
#[test]
fn a_bucket_whose_first_member_is_another_language_still_stars_each_language() {
    let mut registry = FileRegistry::new();
    let files: Vec<FileId> = LANGUAGE_FILES
        .iter()
        .map(|(path, _)| registry.register((*path).into()))
        .collect();
    let languages: HashMap<FileId, &'static str> = files
        .iter()
        .copied()
        .zip(LANGUAGE_FILES.iter().map(|(_, language)| *language))
        .collect();
    let fingerprints: Vec<Fingerprint> = LANGUAGE_MEMBERS
        .iter()
        .enumerate()
        .filter_map(|(position, file)| files.get(*file).map(|file| member(*file, position)))
        .collect();
    let signatures = vec![ZEROED_SIGNATURE; fingerprints.len()];
    let index = SignatureIndex::from_segments([signatures.as_slice()]);
    let no_lsh_pairs: Vec<(usize, usize)> = Vec::new();
    let pairs = candidate_pairs_for_language_policy(
        &fingerprints,
        &index,
        &no_lsh_pairs,
        &[],
        None,
        &languages,
        CROSS_LANGUAGE_OFF,
    );
    let keys: Vec<(usize, usize)> = pairs.iter().map(|pair| (pair.left, pair.right)).collect();
    assert_eq!(
        keys,
        PYTHON_STAR.to_vec(),
        "the Python windows must be paired among themselves although a C window sorts first"
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
