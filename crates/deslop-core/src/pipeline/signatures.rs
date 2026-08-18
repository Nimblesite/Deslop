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
) -> Signature {
    let tokens = language.map_or_else(
        || token_stream_for_fingerprint(root, fingerprint),
        |language| token_stream_for_fingerprint_with_language(root, fingerprint, language),
    );
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
    fn fsharp_tree(source: &str, file_id: FileId) -> Result<NormalizedNode, String> {
        crate::lang::fsharp::FSharpParser
            .parse_and_normalize(source.as_bytes(), file_id)
            .map_err(|error| format!("the F# fixture must parse: {error}"))
    }

    /// Returns the shallowest node whose range starts at `offset`.
    fn node_starting_at(node: &NormalizedNode, offset: usize) -> Option<&NormalizedNode> {
        if node.byte_range.start == offset {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| node_starting_at(child, offset))
    }

    /// Returns true when some node in `root` owns exactly `[start, end)`.
    fn exact_node_exists(root: &NormalizedNode, start: usize, end: usize) -> bool {
        (root.byte_range.start == start && root.byte_range.end == end)
            || root
                .children
                .iter()
                .any(|child| exact_node_exists(child, start, end))
    }

    /// A fingerprint spanning the shared window inside `source`, with the
    /// range derived from parsed declaration boundaries: it starts at the
    /// `accumulate` binding's node and ends at the `combine` binding's node.
    ///
    /// Deliberately not an exact-node range: it spans two consecutive
    /// children of the module and therefore matches no single subtree. That
    /// is the sibling-window shape an exact-node resolver cannot resolve.
    /// Both files get the *same* structural hash, because two copies of one
    /// window really do share a Merkle hash — that is why they pair at all.
    fn window_fingerprint(
        source: &str,
        root: &NormalizedNode,
        file_id: FileId,
    ) -> Result<Fingerprint, String> {
        let accumulate_offset = source
            .find("let accumulate")
            .ok_or("fixture contains the accumulate binding")?;
        let combine_offset = source
            .find("let combine")
            .ok_or("fixture contains the combine binding")?;
        let start = node_starting_at(root, accumulate_offset)
            .ok_or("a parsed node starts at the accumulate binding")?
            .byte_range
            .start;
        let end = node_starting_at(root, combine_offset)
            .ok_or("a parsed node starts at the combine binding")?
            .byte_range
            .end;
        if exact_node_exists(root, start, end) {
            return Err(format!(
                "fixture: {start}..{end} must be a sibling window, not an exact node"
            ));
        }
        Ok(Fingerprint {
            hash: [7; 32],
            file_id,
            byte_range: ByteRange { start, end },
            node_count: 40,
        })
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
    fn issue_339_sibling_window_signature_is_offset_invariant() -> Result<(), String> {
        let mut registry = FileRegistry::new();
        let short = registry.register(PathBuf::from("window_a.fs"));
        let long = registry.register(PathBuf::from("window_b.fs"));

        let short_source = format!("module ParseHelpers\n\n{SHARED_WINDOW}");
        let long_source = format!("module ParseHelpersWithALongerName\n\n{SHARED_WINDOW}");
        let short_tree = fsharp_tree(&short_source, short)?;
        let long_tree = fsharp_tree(&long_source, long)?;
        let short_window = window_fingerprint(&short_source, &short_tree, short)?;
        let long_window = window_fingerprint(&long_source, &long_tree, long)?;

        assert_ne!(
            short_window.byte_range.start, long_window.byte_range.start,
            "fixture: the rename must actually shift the window's offsets"
        );
        let short_len = short_window
            .byte_range
            .end
            .checked_sub(short_window.byte_range.start);
        let long_len = long_window
            .byte_range
            .end
            .checked_sub(long_window.byte_range.start);
        assert_eq!(
            short_len, long_len,
            "fixture: and must not change its length"
        );

        let short_signatures =
            signatures_for_file(&short_tree, &[short_window.clone()], Some("fsharp"));
        let long_signatures =
            signatures_for_file(&long_tree, &[long_window.clone()], Some("fsharp"));

        let ([short_signature], [long_signature]) =
            (short_signatures.as_slice(), long_signatures.as_slice())
        else {
            return Err(format!(
                "expected one signature per fingerprint, got {} and {}",
                short_signatures.len(),
                long_signatures.len()
            ));
        };
        assert_ne!(
            *short_signature,
            fallback_signature(&short_window),
            "issue #339: the sibling-window signature must come from the resolved token \
             stream, not the offset-seeded fallback — falling back means `token_jaccard` \
             measures whether the copies landed at the same byte offset, not whether \
             their tokens agree"
        );
        assert_ne!(
            *long_signature,
            fallback_signature(&long_window),
            "issue #339: the shifted copy must not be the offset-seeded fallback either"
        );
        assert_eq!(
            short_signature, long_signature,
            "issue #339: two copies of one window must produce the same token signature \
             regardless of the byte offset the rename shifted them to"
        );
        Ok(())
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

    // [PIPELINE-INCREMENTAL-INTEGRITY] The fallback signature persists
    // in the parse store, so its slots must be a pure, fixed function
    // of the fingerprint — identical on every architecture and across
    // releases. A 32-bit build hashing 4-byte offsets, or any semantic
    // drift in the derivation, changes every slot and fails this pin.
    #[test]
    fn fallback_signature_slots_are_architecture_independent() {
        let signature = fallback_signature(&fingerprint(7, 3, 9));
        assert_eq!(
            signature.get(..4),
            Some(
                &[
                    13_181_474_024_201_563_239_u64,
                    1_576_249_985_012_619_851,
                    14_983_257_718_629_721_174,
                    4_485_375_611_891_913_186,
                ][..]
            ),
            "the fallback signature's leading slots moved — either the \
             derivation changed semantics (bump the parse store's \
             SEMANTIC_EPOCH) or the input encoding became \
             architecture-dependent"
        );
    }
}
