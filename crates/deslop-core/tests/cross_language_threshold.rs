//! Scope of the cross-language admission exception ([REPORTING-CONTEXT]
//! thresholds, RA-09).
//!
//! The 0.10 floor exists so a lower-overlap port can surface at all, but
//! it is **scoped**: only an explicitly opted-in cross-language pair with
//! no structural anchor (an LSH or direct-signature match) receives it.
//! A structurally anchored cross-language pair keeps the default 0.85
//! assigned by `finalise_pairs`, and without the opt-in a cross-language
//! pair is dropped outright. Documenting the exception more broadly than
//! this is how the public pages drifted — these pins hold the code to
//! the narrowed wording.

use std::collections::HashMap;

use deslop_core::{
    ast::ByteRange,
    fingerprint::Fingerprint,
    lsh::{Signature, SIGNATURE_LEN},
    pair::{
        candidate_pairs_for_language_policy, CandidatePair, CROSS_LANGUAGE_MIN_JACCARD,
        FUSED_THRESHOLD,
    },
    state::{FileId, FileRegistry},
};

/// A fingerprint big enough to clear every information-content floor.
fn fingerprint(hash_seed: u8, file_id: FileId) -> Fingerprint {
    Fingerprint {
        hash: [hash_seed; 32],
        file_id,
        byte_range: ByteRange { start: 0, end: 400 },
        node_count: 64,
    }
}

/// Two registered files mapped to two different languages.
fn two_language_corpus() -> (FileId, FileId, HashMap<FileId, &'static str>) {
    let mut registry = FileRegistry::new();
    let csharp = registry.register("Ledger.cs".into());
    let rust = registry.register("ledger.rs".into());
    let languages: HashMap<FileId, &'static str> =
        [(csharp, "csharp"), (rust, "rust")].into_iter().collect();
    (csharp, rust, languages)
}

/// The candidates the language policy admits. Every case here passes the
/// same empty embedding-pair list and absent cross-language signatures,
/// and respelled the seven-argument call to say so; Deslop scored the
/// copies against this repo's own corpus. Only the three axes a case
/// actually varies stay at the call site.
fn policy_pairs(
    fingerprints: &[Fingerprint],
    signatures: &[Signature],
    lsh_pairs: &[(usize, usize)],
    languages: &HashMap<FileId, &'static str>,
    allow_cross_language: bool,
) -> Vec<CandidatePair> {
    candidate_pairs_for_language_policy(
        fingerprints,
        signatures,
        lsh_pairs,
        &[],
        None,
        languages,
        allow_cross_language,
    )
}

/// Identical signatures, so `token_jaccard` measures 1.0 for the pair.
fn identical_signatures() -> Vec<Signature> {
    vec![[11_u64; SIGNATURE_LEN], [11_u64; SIGNATURE_LEN]]
}

// A cross-language LSH candidate has no structural anchor, so the
// explicit opt-in lowers its admission floor to 0.10 — the exception's
// entire scope.
#[test]
fn an_unanchored_cross_language_pair_receives_the_lowered_floor() -> Result<(), String> {
    let (csharp, rust, languages) = two_language_corpus();
    let fingerprints = [fingerprint(1, csharp), fingerprint(2, rust)];
    let signatures = identical_signatures();
    let pairs = policy_pairs(&fingerprints, &signatures, &[(0, 1)], &languages, true);
    let [pair] = pairs.as_slice() else {
        return Err(format!(
            "expected exactly the one cross-language candidate, got {pairs:?}"
        ));
    };
    assert!(
        pair.score.structural <= 0.0,
        "fixture: differing hashes must leave the pair unanchored, got {pair:?}"
    );
    assert!(
        (pair.fused_min_score - CROSS_LANGUAGE_MIN_JACCARD).abs() < f64::EPSILON,
        "an unanchored cross-language pair must be admitted at the {CROSS_LANGUAGE_MIN_JACCARD} \
         floor, got {pair:?}"
    );
    assert!(
        (pair.lsh_only_min_jaccard - CROSS_LANGUAGE_MIN_JACCARD).abs() < f64::EPSILON,
        "and its LSH-only Jaccard floor is lowered with it, got {pair:?}"
    );
    Ok(())
}

// A structurally anchored cross-language pair is already strong
// evidence; it keeps the normal bar. If this floor ever drops, the
// exception has silently widened past its documented scope.
#[test]
fn a_structurally_anchored_cross_language_pair_keeps_the_default_bar() -> Result<(), String> {
    let (csharp, rust, languages) = two_language_corpus();
    let fingerprints = [fingerprint(7, csharp), fingerprint(7, rust)];
    let signatures = identical_signatures();
    let pairs = policy_pairs(&fingerprints, &signatures, &[], &languages, true);
    let [pair] = pairs.as_slice() else {
        return Err(format!(
            "expected exactly the one structural candidate, got {pairs:?}"
        ));
    };
    assert!(
        pair.score.structural >= 1.0,
        "fixture: equal hashes must produce a structural anchor, got {pair:?}"
    );
    assert!(
        (pair.fused_min_score - FUSED_THRESHOLD).abs() < f64::EPSILON,
        "a structurally anchored cross-language pair keeps the {FUSED_THRESHOLD} bar, \
         got {pair:?}"
    );
    Ok(())
}

// Without the explicit opt-in there is no exception to scope: the
// cross-language LSH candidate is dropped before admission entirely.
#[test]
fn without_the_opt_in_a_cross_language_pair_is_dropped() {
    let (csharp, rust, languages) = two_language_corpus();
    let fingerprints = [fingerprint(1, csharp), fingerprint(2, rust)];
    let signatures = identical_signatures();
    let pairs = policy_pairs(&fingerprints, &signatures, &[(0, 1)], &languages, false);
    assert!(
        pairs.is_empty(),
        "cross-language comparison is off by default, got {pairs:?}"
    );
}
