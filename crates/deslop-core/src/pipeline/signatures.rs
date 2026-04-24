//! `MinHash` signature construction shared by [`super::run`] and
//! [`super::session`]. Feeds the token-LSH pass in [`crate::lsh`].

use std::{collections::HashMap, hash::BuildHasher};

use blake3::Hasher;

use crate::{
    ast::NormalizedNode,
    fingerprint::Fingerprint,
    lsh::{minhash_signature, Signature, SIGNATURE_LEN},
    state::FileId,
    tokens::{
        kgrams, token_stream_for_fingerprint, token_stream_for_fingerprint_with_language,
        KGRAM_WIDTH,
    },
};

/// Language-aware signature builder. When the fingerprint's file has
/// a known language in `file_languages`, import/prologue boilerplate
/// is stripped from the token stream so shared import patterns stop
/// feeding the LSH false-positive path described in
/// [PIPELINE-BOILERPLATE-FILTER] — the structural pass already applies
/// the same filter, so the two signals now share the same corpus.
#[must_use]
pub fn build_signatures_with_languages<S: BuildHasher>(
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Vec<Signature> {
    let mut signatures: Vec<Signature> = Vec::with_capacity(fingerprints.len());
    for fingerprint in fingerprints {
        let language = file_languages.get(&fingerprint.file_id).copied();
        let Some(root) = tree_for_file(trees, fingerprint) else {
            signatures.push(empty_signature(fingerprint, language));
            continue;
        };
        let tokens = match language {
            Some(language @ "python") => {
                token_stream_for_fingerprint_with_language(root, fingerprint, language)
            }
            _ => token_stream_for_fingerprint(root, fingerprint),
        };
        let signature = tokens.map_or_else(
            || empty_signature(fingerprint, language),
            |tokens| signature_for_tokens(&tokens, fingerprint, language),
        );
        signatures.push(signature);
    }
    signatures
}

/// Returns the normalised AST root for `fingerprint`'s file by scanning
/// the per-run tree list. O(n) per lookup; acceptable because the number
/// of files is small compared to the number of fingerprints.
fn tree_for_file<'a>(
    trees: &'a [NormalizedNode],
    fingerprint: &Fingerprint,
) -> Option<&'a NormalizedNode> {
    trees
        .iter()
        .find(|tree| tree.file_id == fingerprint.file_id)
}

/// Produces a signature from a prepared token stream using the configured
/// k-gram width.
fn signature_for_tokens(
    tokens: &[&'static str],
    fingerprint: &Fingerprint,
    language: Option<&str>,
) -> Signature {
    if tokens.is_empty() {
        return empty_signature(fingerprint, language);
    }
    let grams = kgrams(tokens, KGRAM_WIDTH);
    if grams.is_empty() {
        return empty_signature(fingerprint, language);
    }
    let gram_slices: Vec<&[&'static str]> = grams.into_iter().collect();
    minhash_signature(&gram_slices)
}

/// Empty-token signatures stay legacy-compatible except for Python prologue
/// ranges, where unique fallbacks prevent issue #34 false-positive clusters.
fn empty_signature(fingerprint: &Fingerprint, language: Option<&str>) -> Signature {
    if matches!(language, Some("python")) {
        fallback_signature(fingerprint)
    } else {
        default_signature()
    }
}

/// Fingerprint-scoped signature used when no k-grams are available. This
/// avoids treating unrelated empty token sets as perfect LSH matches.
fn fallback_signature(fingerprint: &Fingerprint) -> Signature {
    let mut signature = [0_u64; SIGNATURE_LEN];
    for (index, slot) in signature.iter_mut().enumerate() {
        *slot = fallback_slot(fingerprint, index);
    }
    signature
}

/// Derives one deterministic fallback slot from stable fingerprint data.
fn fallback_slot(fingerprint: &Fingerprint, index: usize) -> u64 {
    let mut hasher = Hasher::new();
    let _ = hasher.update(&fingerprint.hash);
    let _ = hasher.update(&fingerprint.byte_range.start.to_le_bytes());
    let _ = hasher.update(&fingerprint.byte_range.end.to_le_bytes());
    let _ = hasher.update(&index.to_le_bytes());
    let digest = hasher.finalize();
    let mut narrow = [0_u8; 8];
    let slice = digest.as_bytes().get(..8).unwrap_or(&[0_u8; 8]);
    narrow.copy_from_slice(slice);
    u64::from_le_bytes(narrow)
}

/// Default signature used for legacy non-Python empty token streams.
fn default_signature() -> Signature {
    [u64::MAX; SIGNATURE_LEN]
}
