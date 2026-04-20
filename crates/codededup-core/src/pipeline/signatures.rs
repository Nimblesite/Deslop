//! MinHash signature construction shared by [`super::run`] and
//! [`super::session`]. Feeds the token-LSH pass in [`crate::lsh`].

use crate::{
    ast::NormalizedNode,
    fingerprint::Fingerprint,
    lsh::{minhash_signature, Signature},
    tokens::{kgrams, token_stream_for_fingerprint, KGRAM_WIDTH},
};

/// Computes a MinHash signature per fingerprint. Each signature is
/// generated from k-grams of the normalised token stream of the
/// fingerprint's subtree — token Jaccard then acts as the Type-3
/// recall signal per [DECISION-TYPE3-TWO-PASS].
#[must_use]
pub fn build_signatures(fingerprints: &[Fingerprint], trees: &[NormalizedNode]) -> Vec<Signature> {
    let mut signatures: Vec<Signature> = Vec::with_capacity(fingerprints.len());
    for fingerprint in fingerprints {
        let signature = tree_for_file(trees, fingerprint)
            .and_then(|root| token_stream_for_fingerprint(root, fingerprint))
            .map_or_else(default_signature, |tokens| signature_for_tokens(&tokens));
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
fn signature_for_tokens(tokens: &[&'static str]) -> Signature {
    let grams = kgrams(tokens, KGRAM_WIDTH);
    let gram_slices: Vec<&[&'static str]> = grams.into_iter().collect();
    minhash_signature(&gram_slices)
}

/// Default signature used when no k-grams are available (subtree too
/// small to produce any). Every slot saturates at `u64::MAX`.
fn default_signature() -> Signature {
    [u64::MAX; crate::lsh::SIGNATURE_LEN]
}
