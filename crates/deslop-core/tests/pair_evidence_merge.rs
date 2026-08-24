//! [PERF-FLUTTER-TODO-PAIRS] Evidence-merge pins for the streamed pair
//! builder (`docs/performance-branch-review.md`, "first-seen pair
//! deduplication drops stronger evidence"). A pair discovered by more
//! than one pass must carry the merged evidence of every discovery:
//! the structural axis from the Merkle pass and the strongest cosine
//! from the embedding pass, regardless of arrival order. A first-seen
//! key set that refuses later arrivals silently drops evidence and can
//! hide a real duplicate.

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};

use deslop_core::{
    ast::ByteRange,
    embedding::EmbeddingPair,
    fingerprint::Fingerprint,
    lsh::{Signature, SignatureIndex, SIGNATURE_LEN},
    pair::candidate_pairs,
    state::{FileId, FileRegistry},
};

/// Strong cosine: clears the fused threshold on its own evidence.
const STRONG_COSINE: f64 = 0.99;
/// Weak cosine: cannot clear the fused threshold for this pair shape.
const WEAK_COSINE: f64 = 0.30;
/// Subtree size large enough for every LSH-only floor in the fixture.
const NODE_COUNT: usize = 80;

/// Two fingerprints in different files. `structural_clone` decides
/// whether the Merkle hashes collide (a structural star pair) or are
/// distinct (embedding-only discovery).
fn fixture(structural_clone: bool) -> (Vec<Fingerprint>, Vec<Signature>) {
    let mut registry = FileRegistry::new();
    let left = registry.register(PathBuf::from("left.py"));
    let right = registry.register(PathBuf::from("right.py"));
    let left_hash = [7_u8; 32];
    let right_hash = if structural_clone {
        left_hash
    } else {
        [9_u8; 32]
    };
    (
        vec![fingerprint(left, left_hash), fingerprint(right, right_hash)],
        // Identical signatures: token Jaccard 1.0 either way, isolating
        // the structural/embedding axes under test.
        vec![[0; SIGNATURE_LEN], [0; SIGNATURE_LEN]],
    )
}

/// The language map over the same two files `fixture` registers.
fn fixture_languages() -> HashMap<FileId, &'static str> {
    let mut registry = FileRegistry::new();
    let left = registry.register(PathBuf::from("left.py"));
    let right = registry.register(PathBuf::from("right.py"));
    HashMap::from([(left, "python"), (right, "python")])
}

fn fingerprint(file_id: FileId, hash: [u8; 32]) -> Fingerprint {
    Fingerprint {
        hash,
        file_id,
        byte_range: ByteRange { start: 0, end: 50 },
        node_count: NODE_COUNT,
    }
}

/// A structural pair followed by an embedding discovery of the same
/// pair must keep both: `structural = 1.0` from the Merkle pass and the
/// measured cosine from the embedding pass. The streamed builder may
/// not let first-seen keys discard the later cosine.
#[test]
fn structural_and_embedding_discovery_merge_into_one_pair() -> Result<()> {
    let (fingerprints, signatures) = fixture(true);
    let index = SignatureIndex::from_slice(&signatures);
    let embedding_pairs = vec![EmbeddingPair {
        left: 0,
        right: 1,
        cosine: STRONG_COSINE,
    }];

    let no_lsh_pairs: Vec<(usize, usize)> = Vec::new();
    let candidates = candidate_pairs(&fingerprints, &index, &no_lsh_pairs, &embedding_pairs);

    assert_eq!(
        candidates.len(),
        1,
        "one pair, one key — discovery passes merge, they never duplicate"
    );
    let candidate = *candidates.first().context("the merged pair")?;
    assert_eq!((candidate.left, candidate.right), (0, 1));
    assert!(
        (candidate.score.structural - 1.0).abs() < f64::EPSILON,
        "the Merkle pass discovered the clone; structural must stay 1.0, got {}",
        candidate.score.structural
    );
    assert!(
        (candidate.score.embedding_cos - STRONG_COSINE).abs() < 1e-5,
        "the embedding pass measured this same pair; its cosine must be recorded, got {}",
        candidate.score.embedding_cos
    );
    Ok(())
}

/// Duplicate embedding discoveries arriving weakest-first must not
/// poison the pair: the weak cosine's refusal is re-evaluated when the
/// strong cosine arrives, and the merged pair keeps the strongest
/// evidence.
#[test]
fn weakest_first_duplicate_embeddings_still_admit_the_strongest() -> Result<()> {
    let (fingerprints, signatures) = fixture(false);
    let index = SignatureIndex::from_slice(&signatures);
    let embedding_pairs = vec![
        EmbeddingPair {
            left: 0,
            right: 1,
            cosine: WEAK_COSINE,
        },
        EmbeddingPair {
            left: 0,
            right: 1,
            cosine: STRONG_COSINE,
        },
    ];

    let no_lsh_pairs: Vec<(usize, usize)> = Vec::new();
    let candidates = candidate_pairs(&fingerprints, &index, &no_lsh_pairs, &embedding_pairs);

    assert_eq!(
        candidates.len(),
        1,
        "the strong cosine must admit the pair the weak one could not"
    );
    let candidate = *candidates.first().context("the admitted pair")?;
    assert_eq!((candidate.left, candidate.right), (0, 1));
    assert!(
        (candidate.score.embedding_cos - STRONG_COSINE).abs() < 1e-5,
        "merged evidence keeps the strongest cosine, got {}",
        candidate.score.embedding_cos
    );
    assert!(
        candidate.score.structural.abs() < f64::EPSILON,
        "no structural discovery in this fixture; structural must stay 0.0, got {}",
        candidate.score.structural
    );
    Ok(())
}

/// The language-policy entry point merges identically: an ANN cosine
/// for a pair the structural pass already kept is evidence about the
/// pair, not telemetry to discard.
#[test]
fn language_policy_path_merges_structural_and_embedding_too() -> Result<()> {
    let (fingerprints, signatures) = fixture(true);
    let index = SignatureIndex::from_slice(&signatures);
    let languages = fixture_languages();
    let embedding_pairs = vec![EmbeddingPair {
        left: 0,
        right: 1,
        cosine: STRONG_COSINE,
    }];

    let candidates = deslop_core::pair::candidate_pairs_for_language_policy(
        &fingerprints,
        &index,
        &Vec::new(),
        &embedding_pairs,
        None,
        &languages,
        false,
    );

    assert_eq!(candidates.len(), 1, "same-language pair, admitted once");
    let candidate = *candidates.first().context("the merged pair")?;
    assert!(
        (candidate.score.structural - 1.0).abs() < f64::EPSILON,
        "structural axis from the Merkle pass, got {}",
        candidate.score.structural
    );
    assert!(
        (candidate.score.embedding_cos - STRONG_COSINE).abs() < 1e-5,
        "cosine from the embedding pass, got {}",
        candidate.score.embedding_cos
    );
    Ok(())
}
