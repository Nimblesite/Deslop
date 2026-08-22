//! Signature-layer unit tests: offset invariance of sibling-window
//! signatures (#339), fingerprint-scoped fallbacks (#86), the
//! architecture-independence pin for persisted fallback slots
//! ([PIPELINE-INCREMENTAL-INTEGRITY]), and the stream-digest memo
//! ([PIPELINE-SIGNATURE-MEMO]).

use std::path::PathBuf;

use super::*;
use crate::{ast::ByteRange, fingerprint::Fingerprint, lang::LanguageParser, state::FileRegistry};

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

    // One memo per file, deliberately: the equality assertion below
    // must compare two independent constructions. A shared memo would
    // hand the second file the first file's signature back and prove
    // nothing about offset invariance.
    let mut short_memo = SignatureMemo::default();
    let mut long_memo = SignatureMemo::default();
    let short_signatures = signatures_for_file(
        &short_tree,
        std::slice::from_ref(&short_window),
        Some("fsharp"),
        &mut short_memo,
    );
    let long_signatures = signatures_for_file(
        &long_tree,
        std::slice::from_ref(&long_window),
        Some("fsharp"),
        &mut long_memo,
    );

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

// [PIPELINE-SIGNATURE-MEMO] The capture test for the corpus-build
// signature cost: on the Flutter material corpus, 90% of 285,510
// fingerprints are sibling windows whose token streams repeat across
// files, and rebuilding the `MinHash` for every repeat put 58 of the
// corpus stage's 62 seconds into signature construction. One distinct
// stream must cost one construction; every further occurrence must be
// answered from the memo with a byte-identical signature.
#[test]
fn a_repeated_token_stream_costs_one_minhash_construction() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let first = registry.register(PathBuf::from("repeat_a.fs"));
    let second = registry.register(PathBuf::from("repeat_b.fs"));

    let first_source = format!(
        "module RepeatFirst

{SHARED_WINDOW}"
    );
    let second_source = format!(
        "module RepeatSecondRenamed

{SHARED_WINDOW}"
    );
    let first_tree = fsharp_tree(&first_source, first)?;
    let second_tree = fsharp_tree(&second_source, second)?;
    let first_window = window_fingerprint(&first_source, &first_tree, first)?;
    let second_window = window_fingerprint(&second_source, &second_tree, second)?;

    let mut shared_memo = SignatureMemo::default();
    let first_signatures = signatures_for_file(
        &first_tree,
        std::slice::from_ref(&first_window),
        Some("fsharp"),
        &mut shared_memo,
    );
    let second_signatures = signatures_for_file(
        &second_tree,
        std::slice::from_ref(&second_window),
        Some("fsharp"),
        &mut shared_memo,
    );

    assert_eq!(
        (shared_memo.misses(), shared_memo.hits()),
        (1, 1),
        "one distinct stream across two files must cost exactly one          MinHash construction and one memo answer — a second          construction means the memo key failed to collapse identical          streams, and the 90%-sibling-window corpus pays the whole          signature stage again"
    );

    let mut unmemoised = SignatureMemo::default();
    let independent = signatures_for_file(
        &second_tree,
        std::slice::from_ref(&second_window),
        Some("fsharp"),
        &mut unmemoised,
    );
    assert_eq!(
        (first_signatures.as_slice(), second_signatures.as_slice()),
        (independent.as_slice(), independent.as_slice()),
        "the memo answer must be byte-identical to an independent          construction of the same stream — anything else silently          rewrites token_jaccard for every repeated window"
    );
    assert_eq!(
        (unmemoised.misses(), unmemoised.hits()),
        (1, 0),
        "fixture: the independent construction must itself be a fresh miss"
    );
    Ok(())
}

// [PIPELINE-SIGNATURE-MEMO] The fallback path is scoped to the
// fingerprint's byte range on purpose (#86), so it must never enter
// the stream memo: two unrelated too-short streams answering each
// other from the memo would LSH-cluster through shared emptiness.
#[test]
fn too_short_streams_never_touch_the_memo() {
    let mut memo = SignatureMemo::default();
    let short_stream = fingerprint(3, 5, 9);
    let produced = signature_for_tokens(&["a"; 4], &short_stream, Some("rust"), &mut memo);
    assert_eq!(
        produced,
        fallback_signature(&short_stream),
        "a stream shorter than KGRAM_WIDTH must fall back to the          fingerprint-scoped signature"
    );
    assert_eq!(
        (memo.hits(), memo.misses()),
        (0, 0),
        "the fallback must never be memoised: its whole point is that          unrelated empty streams do not share a signature"
    );
}
