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
    boilerplate::is_import_boilerplate_only_subtree,
    fingerprint::Fingerprint,
    lsh::{minhash_signature, Signature, SIGNATURE_LEN},
    state::FileId,
    tokens::{
        cross_language_token_stream_for_fingerprint, kgrams, token_stream_for_fingerprint,
        token_stream_for_fingerprint_with_language, KGRAM_WIDTH,
    },
};

/// Builds a `FileId → &NormalizedNode` index to avoid O(files) linear scans
/// for every fingerprint in [`build_signatures_with_languages`].
fn build_tree_index(trees: &[NormalizedNode]) -> HashMap<FileId, &NormalizedNode> {
    trees.iter().map(|tree| (tree.file_id, tree)).collect()
}

/// Language-aware signature for one fingerprint against its file's
/// normalised tree. When the language is known, import/prologue
/// boilerplate is stripped from the token stream so shared import
/// patterns stop feeding the LSH false-positive path described in
/// [PIPELINE-BOILERPLATE-FILTER] — the structural pass already applies
/// the same filter, so the two signals share the same corpus.
///
/// A pure function of the tree content, the fingerprint's range and
/// hash, and the language — never of [`FileId`] — which is what
/// licenses persisting the result in the content-addressed parse
/// store ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
fn signature_for_fingerprint(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
    language: Option<&str>,
) -> Signature {
    let tokens = match language {
        Some("python") => token_stream_for_fingerprint_with_language(root, fingerprint, "python"),
        Some(language) if exact_range_contains_boilerplate(root, fingerprint, language) => {
            token_stream_for_fingerprint_with_language(root, fingerprint, language)
        }
        _ => token_stream_for_fingerprint(root, fingerprint),
    };
    tokens.map_or_else(
        || empty_signature(fingerprint, language),
        |tokens| signature_for_tokens(&tokens, fingerprint, language),
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
) -> Vec<Signature> {
    fingerprints
        .iter()
        .map(|fingerprint| signature_for_fingerprint(tree, fingerprint, language))
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
    fingerprints
        .iter()
        .map(|fingerprint| {
            let language = file_languages.get(&fingerprint.file_id).copied();
            cross_language_signature(fingerprint, &tree_index, language)
        })
        .collect()
}

/// Builds one cross-language signature, falling back to fingerprint scope.
fn cross_language_signature(
    fingerprint: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    language: Option<&str>,
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
        |tokens| signature_for_tokens(&tokens, fingerprint, Some(language)),
    )
}

/// Returns true when an exact fingerprint range contains prologue syntax.
fn exact_range_contains_boilerplate(
    node: &NormalizedNode,
    fingerprint: &Fingerprint,
    language: &str,
) -> bool {
    if node.byte_range.start == fingerprint.byte_range.start
        && node.byte_range.end == fingerprint.byte_range.end
    {
        return subtree_contains_boilerplate(node, language);
    }
    if node.byte_range.start > fingerprint.byte_range.start
        || node.byte_range.end < fingerprint.byte_range.end
    {
        return false;
    }
    node.children
        .iter()
        .any(|child| exact_range_contains_boilerplate(child, fingerprint, language))
}

/// Returns true when `node` or a descendant is import/prologue boilerplate.
fn subtree_contains_boilerplate(node: &NormalizedNode, language: &str) -> bool {
    is_import_boilerplate_only_subtree(language, node)
        || node
            .children
            .iter()
            .any(|child| subtree_contains_boilerplate(child, language))
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
fn fallback_signature(fingerprint: &Fingerprint) -> Signature {
    let mut hasher = Hasher::new();
    let _ = hasher.update(&fingerprint.hash);
    let _ = hasher.update(&fingerprint.byte_range.start.to_le_bytes());
    let _ = hasher.update(&fingerprint.byte_range.end.to_le_bytes());
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
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{ast::ByteRange, fingerprint::Fingerprint, state::FileRegistry};

    fn fingerprint(seed: u8, start: usize, end: usize) -> Fingerprint {
        let mut registry = FileRegistry::new();
        let file_id = registry.register(PathBuf::from(format!("fixture_{seed}.rs")));
        Fingerprint {
            hash: [seed; 32],
            file_id,
            byte_range: ByteRange { start, end },
            node_count: 1,
        }
    }

    #[test]
    fn issue_86_empty_non_python_signatures_are_fingerprint_scoped() {
        let first = fingerprint(1, 0, 0);
        let second = fingerprint(2, 0, 0);

        let first_rust = empty_signature(&first, Some("rust"));
        let second_rust = empty_signature(&second, Some("rust"));
        let first_unknown = empty_signature(&first, None);
        let second_unknown = empty_signature(&second, None);

        assert_ne!(
            first_rust, second_rust,
            "issue #86: unrelated empty Rust token streams must not share a legacy signature"
        );
        assert_ne!(
            first_unknown, second_unknown,
            "issue #86: unrelated empty unknown-language streams must not share a legacy signature"
        );
        assert_eq!(
            first_rust,
            empty_signature(&first, Some("rust")),
            "fingerprint-scoped fallback must stay deterministic for the same fingerprint"
        );
    }
}
