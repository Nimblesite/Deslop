//! Signature-layer unit tests: the bottom-up fold's byte-for-byte parity
//! with the historical top-down construction
//! ([PERF-FLUTTER-TODO-CORPUS]), offset invariance of sibling-window
//! signatures (#339), fingerprint-scoped fallbacks (#86), and the
//! architecture-independence pin for persisted fallback slots
//! ([PIPELINE-INCREMENTAL-INTEGRITY]).

use std::path::PathBuf;

use super::*;
use crate::{
    ast::ByteRange,
    fingerprint::{collect_non_boilerplate_fingerprints, Fingerprint},
    lang::{
        shared::FILE_KIND, LanguageParser,
    },
    sibling::collect_non_boilerplate_sibling_fingerprints,
    state::{FileId, FileRegistry},
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

/// One synthetic normalised node: the shape the fold walks, without a
/// parser in the way. Ranges are hand-placed so windows, wrappers and
/// boilerplate subtrees are all constructible exactly.
fn node(
    file_id: FileId,
    kind: &'static str,
    start: usize,
    end: usize,
    children: Vec<NormalizedNode>,
) -> NormalizedNode {
    NormalizedNode {
        kind,
        children,
        byte_range: ByteRange { start, end },
        file_id,
    }
}

/// A python file-shaped root over `children`.
fn file_root(file_id: FileId, children: Vec<NormalizedNode>) -> NormalizedNode {
    let end = children
        .last()
        .map_or(0, |last| last.byte_range.end);
    node(file_id, FILE_KIND, 0, end, children)
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

/// The fingerprint population the corpus build produces for `tree`:
/// exact-node fingerprints followed by sibling-window fingerprints.
fn corpus_fingerprints(tree: &NormalizedNode, language: &str, min_nodes: usize) -> Vec<Fingerprint> {
    let mut fingerprints = collect_non_boilerplate_fingerprints(tree, min_nodes, language);
    fingerprints.extend(collect_non_boilerplate_sibling_fingerprints(
        tree, min_nodes, language,
    ));
    fingerprints
}

/// [PERF-FLUTTER-TODO-CORPUS] The fold must reproduce the historical
/// top-down construction byte-for-byte over the full fingerprint
/// population — exact nodes and sibling windows alike — because the
/// signatures persist in the parse store and feed every rendered
/// `token_jaccard`. Any divergence is a silent accuracy change dressed
/// as a performance fix.
///
/// The fixture is a synthetic Python tree holding: an import-only
/// prologue (exercises the token-skip predicate), a deep expression
/// chain (junction grams whose left side exceeds `KGRAM_WIDTH`), a run
/// of eight statements (windows of every width the sibling pass
/// emits), and a wrapper sharing its only child's byte range (the
/// shallowest-owner deferral).
#[test]
fn fold_signatures_match_the_top_down_construction() {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("synthetic.py"));
    let import = node(
        file_id,
        "import_statement",
        0,
        20,
        vec![node(file_id, "__ident__", 7, 12, vec![])],
    );
    // Eight sibling statements, each a function_call with a deep argument
    // chain so sequences pass KGRAM_WIDTH and junction grams read from
    // both sides.
    let mut statements = Vec::new();
    // Statement `i` owns [i*40, i*40+40); its call chain lives inside at
    // [+5, +35). Each binary wrap EXPANDS outward with depth so every
    // child stays contained — the containment the range resolver and
    // the window enumerator both assume.
    let mut offset = 40;
    for _statement in 0..8 {
        let mut argument = node(file_id, "argument_list", offset + 10, offset + 30, vec![]);
        for depth in 0..6 {
            let start = offset + 10 - depth;
            let end = offset + 30 + depth;
            argument = node(file_id, "binary_expression", start, end, vec![argument]);
        }
        statements.push(node(
            file_id,
            "expression_statement",
            offset,
            offset.saturating_add(40),
            vec![node(
                file_id,
                "call",
                offset + 5,
                offset.saturating_add(35),
                vec![
                    node(file_id, "__ident__", offset + 5, offset + 9, vec![]),
                    argument,
                ],
            )],
        ));
        offset = offset.saturating_add(40);
    }
    // A wrapper that re-describes its only child's range: both emit
    // fingerprints today, and the resolver answers from the wrapper.
    let inner = node(file_id, "block", offset, offset + 30, vec![]);
    let wrapper = node(file_id, "function_body", offset, offset + 30, vec![inner]);
    let tree = file_root(
        file_id,
        vec![import, wrapper]
            .into_iter()
            .chain(statements)
            .collect(),
    );

    let fingerprints = corpus_fingerprints(&tree, "python", 3);
    assert!(
        fingerprints.len() > 30,
        "fixture must produce a real population, got {}",
        fingerprints.len()
    );
    let folded = signatures_for_file(&tree, &fingerprints, Some("python"));
    assert_eq!(folded.len(), fingerprints.len());
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        let reference = top_down_signature(&tree, fingerprint, Some("python"));
        assert_eq!(
            folded.get(index),
            Some(&reference),
            "fold diverged from the top-down construction at fingerprint {index} \
             (range {:?}, node_count {})",
            fingerprint.byte_range,
            fingerprint.node_count
        );
    }

    // The language-agnostic path must agree too.
    let folded_plain = signatures_for_file(&tree, &fingerprints, None);
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        let reference = top_down_signature(&tree, fingerprint, None);
        assert_eq!(
            folded_plain.get(index),
            Some(&reference),
            "fold diverged on the language-agnostic path at fingerprint {index}"
        );
    }
}

/// [PERF-FLUTTER-TODO-CORPUS] Parity over a real parse: the F# fixture
/// through the actual parser, exact and sibling fingerprints together.
#[test]
fn fold_signatures_match_top_down_over_a_real_parse() -> Result<(), String> {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("parity.fs"));
    let source = format!("module Parity\n\n{SHARED_WINDOW}");
    let tree = fsharp_tree(&source, file_id)?;
    let fingerprints = corpus_fingerprints(&tree, "fsharp", 3);
    assert!(
        fingerprints.len() > 10,
        "fixture must produce fingerprints, got {}",
        fingerprints.len()
    );
    let folded = signatures_for_file(&tree, &fingerprints, Some("fsharp"));
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        let reference = top_down_signature(&tree, fingerprint, Some("fsharp"));
        assert_eq!(
            folded.get(index),
            Some(&reference),
            "fold diverged from the top-down construction at fingerprint {index}"
        );
    }
    Ok(())
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

    // Independent constructions per file, deliberately: the equality
    // assertion below must compare two independent folds. Nothing is
    // shared between the two calls, so equality is a property of the
    // construction, not of a memo handing one side the other's answer.
    let short_signatures = signatures_for_file(
        &short_tree,
        std::slice::from_ref(&short_window),
        Some("fsharp"),
    );
    let long_signatures = signatures_for_file(
        &long_tree,
        std::slice::from_ref(&long_window),
        Some("fsharp"),
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

// [PERF-FLUTTER-TODO-CORPUS] The structural guarantee the memo used to
// buy: two files holding byte-identical windows produce byte-identical
// signatures, and both match an independent top-down construction of the
// same stream. The fold gives this by construction — equal token
// sequences have equal k-grams — so the pin now asserts it directly
// instead of counting constructions.
#[test]
fn repeated_token_streams_produce_byte_identical_signatures() -> Result<(), String> {
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

    let first_signatures = signatures_for_file(
        &first_tree,
        std::slice::from_ref(&first_window),
        Some("fsharp"),
    );
    let second_signatures = signatures_for_file(
        &second_tree,
        std::slice::from_ref(&second_window),
        Some("fsharp"),
    );
    assert_eq!(
        first_signatures, second_signatures,
        "two copies of one window must produce the same signature — anything else \
         silently rewrites token_jaccard for every repeated window"
    );
    assert_eq!(
        first_signatures.as_slice(),
        [top_down_signature(&first_tree, &first_window, Some("fsharp"))],
        "the fold's signature must be byte-identical to an independent top-down \
         construction of the same stream"
    );
    Ok(())
}

// #86 / [PIPELINE-SIGNATURE-FALLBACK] A stream too short to hold a
// k-gram falls back to the fingerprint-scoped signature, so unrelated
// short streams never share one. Two such fingerprints over one tree
// must therefore differ — and must both be fallback signatures.
#[test]
fn too_short_streams_stay_fingerprint_scoped() {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("short.py"));
    let tree = file_root(
        file_id,
        vec![
            node(
                file_id,
                "expression_statement",
                0,
                10,
                vec![node(file_id, "__ident__", 0, 9, vec![])],
            ),
            node(
                file_id,
                "expression_statement",
                10,
                20,
                vec![node(file_id, "__literal__", 10, 19, vec![])],
            ),
        ],
    );
    let short_a = fingerprint(1, 0, 10);
    let short_b = fingerprint(2, 10, 20);
    let expected_a = fallback_signature(&short_a);
    let expected_b = fallback_signature(&short_b);
    let produced = signatures_for_file(&tree, &[short_a, short_b], Some("python"));
    assert_eq!(
        produced.as_slice(),
        [expected_a, expected_b],
        "streams shorter than KGRAM_WIDTH must fall back to the \
         fingerprint-scoped signature"
    );
    assert_ne!(
        produced.first(),
        produced.get(1),
        "issue #86: unrelated empty token streams must not share a signature"
    );
}

// #86: unresolvable ranges keep the fingerprint-scoped fallback too.
#[test]
fn issue_86_unresolvable_ranges_are_fingerprint_scoped() {
    let first = fingerprint(1, 0, 0);
    let second = fingerprint(2, 0, 0);
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("empty.rs"));
    let tree = file_root(file_id, vec![]);

    let expected_first = fallback_signature(&first);
    let produced = signatures_for_file(&tree, &[first.clone(), second.clone()], Some("rust"));
    let unknown_language = signatures_for_file(&tree, &[first, second], None);

    assert_ne!(
        produced.first(),
        produced.get(1),
        "issue #86: unrelated empty Rust token streams must not share a legacy signature"
    );
    assert_ne!(
        unknown_language.first(),
        unknown_language.get(1),
        "issue #86: unrelated empty unknown-language streams must not share a legacy \
         signature either"
    );
    assert_eq!(
        produced.first(),
        Some(&expected_first),
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



