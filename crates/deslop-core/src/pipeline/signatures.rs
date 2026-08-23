//! `MinHash` signature construction. Feeds the token-LSH pass in
//! [`crate::lsh`].
//!
//! Per-language signatures are built once per file at parse/load time
//! by [`signatures_for_file`] and persisted in the parse store beside
//! the fingerprints they were built from
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]); the render pass consumes
//! the flattened per-file lists instead of reconstructing them.
//! Cross-language signatures stay render-time — they exist only for
//! the opt-in audit mode ([CONFIG-CROSS-LANGUAGE]).

use std::{collections::HashMap, hash::BuildHasher};

use blake3::Hasher;

use crate::{
    ast::NormalizedNode,
    fingerprint::Fingerprint,
    lsh::{minhash_signature, Signature, SIGNATURE_LEN},
    state::FileId,
    tokens::{
        cross_language_token_stream_for_fingerprint, kgrams, token_stream_for_fingerprint,
        token_stream_for_fingerprint_with_language, KGRAM_WIDTH,
    },
};

/// Builds a `FileId → &NormalizedNode` index to avoid O(files) linear scans
/// for every fingerprint in [`build_cross_language_signatures`].
fn build_tree_index(trees: &[NormalizedNode]) -> HashMap<FileId, &NormalizedNode> {
    trees.iter().map(|tree| (tree.file_id, tree)).collect()
}

/// Memoises `MinHash` construction by token-stream digest across one
/// corpus build ([PIPELINE-SIGNATURE-MEMO]).
///
/// `MinHash` over the k-grams is a pure function of the token stream
/// alone, so two fingerprints whose ranges resolve to identical streams
/// — the common case in a corpus whose whole pathology is repeated
/// structure — get byte-identical signatures from one construction and
/// a digest lookup, instead of paying the per-k-gram hash cascade
/// again. Sound by construction: the key pins the exact stream, not a
/// structural proxy for it. Fallback signatures never pass through
/// here — they are deliberately scoped to the fingerprint's byte range
/// ([`fallback_signature`]) and stay per-fingerprint.
#[derive(Debug, Default)]
pub struct SignatureMemo {
    /// Stream digest → the signature its k-grams minhash to.
    memoised: HashMap<[u8; 32], Signature>,
    /// Streams answered from the memo.
    hits: u64,
    /// Streams that paid for a fresh `MinHash` construction.
    misses: u64,
}

/// Most distinct streams one memo retains. Each entry holds a
/// 32-byte digest and a 1 KiB signature, so the memo's resident
/// ceiling is ~270 MiB — a bounded share of the
/// [PERF-FLUTTER-TODO-MEMORY] budget however large the corpus grows.
/// Past the cap a fresh stream is constructed and not retained; the
/// output is identical either way, the memo being transparent.
const SIGNATURE_MEMO_MAX_ENTRIES: usize = 262_144;

impl SignatureMemo {
    /// The signature for a non-empty token stream, constructed at most
    /// once per distinct retained stream.
    fn signature(&mut self, tokens: &[&'static str]) -> Signature {
        let key = stream_digest(tokens);
        if let Some(found) = self.memoised.get(&key) {
            crate::observe::bump(&mut self.hits);
            return *found;
        }
        crate::observe::bump(&mut self.misses);
        let grams = kgrams(tokens, KGRAM_WIDTH);
        let constructed = minhash_signature(&grams);
        if self.memoised.len() < SIGNATURE_MEMO_MAX_ENTRIES {
            let _previous = self.memoised.insert(key, constructed);
        }
        constructed
    }

    /// Streams answered from the memo.
    #[must_use]
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Streams that paid for a fresh construction — the distinct-stream
    /// population, which is also the memo's resident entry count.
    #[must_use]
    pub fn misses(&self) -> u64 {
        self.misses
    }
}

/// Collision-free digest of a token stream: every token length-prefixed
/// so no two distinct streams serialise to the same bytes.
fn stream_digest(tokens: &[&'static str]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    for token in tokens {
        let length = u64::try_from(token.len()).unwrap_or(u64::MAX);
        let _ = hasher.update(&length.to_le_bytes());
        let _ = hasher.update(token.as_bytes());
    }
    *hasher.finalize().as_bytes()
}

/// Language-aware signature for one fingerprint against its file's
/// normalised tree. When the language is known, import/prologue
/// boilerplate is stripped from the token stream so shared import
/// patterns stop feeding the LSH false-positive path described in
/// [PIPELINE-BOILERPLATE-FILTER] — the structural pass already applies
/// the same filter, so the two signals share the same corpus. The
/// language-aware path also resolves synthetic sibling-window byte
/// ranges (#339): a window spanning several consecutive children gets
/// its signature from the resolved token stream instead of the
/// offset-seeded fallback, so `token_jaccard` measures token evidence
/// rather than byte-offset luck.
///
/// A pure function of the tree content, the fingerprint's range and
/// hash, and the language — never of [`FileId`] — which is what
/// licenses persisting the result in the content-addressed parse
/// store ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
fn signature_for_fingerprint(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
    language: Option<&str>,
    memo: &mut SignatureMemo,
) -> Signature {
    let tokens = language.map_or_else(
        || token_stream_for_fingerprint(root, fingerprint),
        |language| token_stream_for_fingerprint_with_language(root, fingerprint, language),
    );
    tokens.map_or_else(
        || empty_signature(fingerprint, language),
        |tokens| signature_for_tokens(&tokens, fingerprint, language, memo),
    )
}

/// Builds one file's `MinHash` signatures, positionally 1:1 with
/// `fingerprints`. Called at parse/load time so the result is
/// persisted in the parse store beside the fingerprints it was built
/// from and reattached on later cache hits
/// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
#[must_use]
pub fn signatures_for_file(
    tree: &NormalizedNode,
    fingerprints: &[Fingerprint],
    language: Option<&str>,
    memo: &mut SignatureMemo,
) -> Vec<Signature> {
    fingerprints
        .iter()
        .map(|fingerprint| signature_for_fingerprint(tree, fingerprint, language, memo))
        .collect()
}

/// Builds aliases-only signatures for explicit cross-language audits.
#[must_use]
pub fn build_cross_language_signatures<S: BuildHasher>(
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Vec<Signature> {
    let tree_index = build_tree_index(trees);
    let mut memo = SignatureMemo::default();
    fingerprints
        .iter()
        .map(|fingerprint| {
            let language = file_languages.get(&fingerprint.file_id).copied();
            cross_language_signature(fingerprint, &tree_index, language, &mut memo)
        })
        .collect()
}

/// Builds one cross-language signature, falling back to fingerprint scope.
fn cross_language_signature(
    fingerprint: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    language: Option<&str>,
    memo: &mut SignatureMemo,
) -> Signature {
    let Some(language) = language else {
        return empty_signature(fingerprint, None);
    };
    let Some(root) = tree_index.get(&fingerprint.file_id).copied() else {
        return empty_signature(fingerprint, Some(language));
    };
    let tokens = cross_language_token_stream_for_fingerprint(root, fingerprint, language);
    tokens.map_or_else(
        || empty_signature(fingerprint, Some(language)),
        |tokens| signature_for_tokens(&tokens, fingerprint, Some(language), memo),
    )
}

/// Produces a signature from a prepared token stream using the configured
/// k-gram width.
fn signature_for_tokens(
    tokens: &[&'static str],
    fingerprint: &Fingerprint,
    language: Option<&str>,
    memo: &mut SignatureMemo,
) -> Signature {
    if tokens.len() < KGRAM_WIDTH {
        return empty_signature(fingerprint, language);
    }
    memo.signature(tokens)
}

/// Empty-token signatures are scoped to the exact fingerprint instead of a
/// shared legacy default so unrelated empty token streams do not LSH-cluster
/// through compatibility behavior.
fn empty_signature(fingerprint: &Fingerprint, language: Option<&str>) -> Signature {
    let _ = language;
    fallback_signature(fingerprint)
}

/// Fingerprint-scoped signature used when no k-grams are available. This
/// avoids treating unrelated empty token sets as perfect LSH matches.
/// Uses blake3 XOF to derive all 128 slot values from a single hash call.
/// The byte offsets are widened to `u64` before hashing so the input is
/// always eight little-endian bytes per offset — `usize::to_le_bytes()`
/// is four bytes on a 32-bit build, and these values persist in the
/// parse store, where an architecture-dependent signature would defeat
/// content addressing ([PIPELINE-INCREMENTAL-INTEGRITY]).
fn fallback_signature(fingerprint: &Fingerprint) -> Signature {
    let start = u64::try_from(fingerprint.byte_range.start).unwrap_or(u64::MAX);
    let end = u64::try_from(fingerprint.byte_range.end).unwrap_or(u64::MAX);
    let mut hasher = Hasher::new();
    let _ = hasher.update(&fingerprint.hash);
    let _ = hasher.update(&start.to_le_bytes());
    let _ = hasher.update(&end.to_le_bytes());
    let mut expanded = [0u8; SIGNATURE_LEN * 8];
    hasher.finalize_xof().fill(&mut expanded);
    let mut signature = [0_u64; SIGNATURE_LEN];
    for (slot, chunk) in signature.iter_mut().zip(expanded.chunks_exact(8)) {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(chunk);
        *slot = u64::from_le_bytes(arr);
    }
    signature
}

#[cfg(test)]
mod tests;
