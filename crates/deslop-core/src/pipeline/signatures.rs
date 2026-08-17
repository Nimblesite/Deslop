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
/// the same filter, so the two signals now share the same corpus. The
/// language-aware path also resolves synthetic sibling-window byte
/// ranges (#339): a window spanning several consecutive children gets
/// its signature from the resolved token stream instead of the
/// offset-seeded fallback, so `token_jaccard` measures token evidence
/// rather than byte-offset luck.
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
        let tokens = language.map_or_else(
            || token_stream_for_fingerprint(root, fingerprint),
            |language| token_stream_for_fingerprint_with_language(root, fingerprint, language),
        );
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

        let trees = vec![short_tree, long_tree];
        let languages: HashMap<FileId, &'static str> =
            [(short, "fsharp"), (long, "fsharp")].into_iter().collect();
        let fingerprints = [short_window.clone(), long_window.clone()];
        let signatures = build_signatures_with_languages(&fingerprints, &trees, &languages);

        let [short_signature, long_signature] = signatures.as_slice() else {
            return Err(format!(
                "expected one signature per fingerprint, got {}",
                signatures.len()
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
}
