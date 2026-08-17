//! `MinHash` signature construction shared by [`super::run`] and
//! [`super::session`]. Feeds the token-LSH pass in [`crate::lsh`].

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
    let tree_index = build_tree_index(trees);
    let mut signatures: Vec<Signature> = Vec::with_capacity(fingerprints.len());
    for fingerprint in fingerprints {
        let language = file_languages.get(&fingerprint.file_id).copied();
        let Some(root) = tree_index.get(&fingerprint.file_id).copied() else {
            signatures.push(empty_signature(fingerprint, language));
            continue;
        };
        let tokens = match language {
            Some("python") => {
                token_stream_for_fingerprint_with_language(root, fingerprint, "python")
            }
            Some(language) if exact_range_contains_boilerplate(root, fingerprint, language) => {
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
    use crate::{
        ast::ByteRange, fingerprint::Fingerprint, lang::LanguageParser, state::FileRegistry,
    };

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

    /// The duplicated region, verbatim in both files.
    const SHARED_WINDOW: &str = "\
let accumulate (values: int list) (floor: int) =
    let mutable total = 0
    for value in values do
        if value > floor then
            total <- total + value * 2
    total

let combine (values: int list) (ceiling: int) =
    let mutable carried = 1
    for value in values do
        if value < ceiling then
            carried <- carried * value + 7
    carried
";

    /// Parses `source` as F# and returns its normalised root.
    fn fsharp_tree(source: &str, file_id: FileId) -> NormalizedNode {
        crate::lang::fsharp::FSharpParser
            .parse_and_normalize(source.as_bytes(), file_id)
            .expect("the F# fixture must parse")
    }

    /// A fingerprint spanning the shared window inside `source`.
    ///
    /// Deliberately not an exact-node range: it starts at the first shared
    /// binding and ends at the last, which spans several consecutive children
    /// of the module and therefore matches no single subtree. That is the
    /// sibling-window shape `token_stream_for_fingerprint` cannot resolve.
    /// Both files get the *same* structural hash, because two copies of one
    /// window really do share a Merkle hash — that is why they pair at all.
    fn window_fingerprint(source: &str, file_id: FileId) -> Fingerprint {
        let start = source.find(SHARED_WINDOW).expect("fixture contains the window");
        Fingerprint {
            hash: [7; 32],
            file_id,
            byte_range: ByteRange {
                start,
                end: start + SHARED_WINDOW.len(),
            },
            node_count: 40,
        }
    }

    // #339 ([FUSION-SIGNALS-THREE-LAYER]). Isolated at the signature layer on
    // purpose: `content_gated_signals` overwrites a shape-identical cluster's
    // rendered `token_jaccard` to 1.0, so NO end-to-end assertion on a
    // rendered signal can distinguish real token evidence from the renderer
    // supplying the value. This is the only layer where the question is
    // answerable.
    //
    // A module rename shifts every byte offset in the second file. The window
    // itself is byte-for-byte unchanged, and the normalised kind stream it
    // produces is rename-invariant by construction, so the two signatures must
    // be equal. `fallback_signature` hashes `(hash, byte_range.start,
    // byte_range.end)`, so if the fingerprint falls through to it the two
    // signatures differ completely — and `token_jaccard` is then measuring
    // whether the copies happened to land at the same offset, not whether
    // their tokens agree.
    #[test]
    fn issue_339_sibling_window_signature_is_offset_invariant() {
        let mut registry = FileRegistry::new();
        let short = registry.register(PathBuf::from("window_a.fs"));
        let long = registry.register(PathBuf::from("window_b.fs"));

        let short_source = format!("module ParseHelpers\n\n{SHARED_WINDOW}");
        let long_source = format!("module ParseHelpersWithALongerName\n\n{SHARED_WINDOW}");
        let short_window = window_fingerprint(&short_source, short);
        let long_window = window_fingerprint(&long_source, long);

        assert_ne!(
            short_window.byte_range.start, long_window.byte_range.start,
            "fixture: the rename must actually shift the window's offsets"
        );
        assert_eq!(
            short_window.byte_range.end - short_window.byte_range.start,
            long_window.byte_range.end - long_window.byte_range.start,
            "fixture: and must not change its length"
        );

        let trees = vec![
            fsharp_tree(&short_source, short),
            fsharp_tree(&long_source, long),
        ];
        let languages: HashMap<FileId, &'static str> =
            [(short, "fsharp"), (long, "fsharp")].into_iter().collect();
        let signatures = build_signatures_with_languages(
            &[short_window, long_window],
            &trees,
            &languages,
        );

        assert_eq!(signatures.len(), 2, "one signature per fingerprint");
        assert_eq!(
            signatures[0], signatures[1],
            "issue #339: two copies of one window must produce the same token \
             signature. Differing here means the signature fell back to \
             blake3(hash, byte_range) and `token_jaccard` is measuring byte-offset \
             luck rather than token evidence"
        );
        assert_ne!(
            signatures[0],
            fallback_signature(&window_fingerprint(&short_source, short)),
            "issue #339: and it must not BE the offset-seeded fallback"
        );
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
