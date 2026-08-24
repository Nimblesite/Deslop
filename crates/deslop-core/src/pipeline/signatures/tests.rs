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
    lang::{shared::FILE_KIND, LanguageParser},
    sibling::collect_non_boilerplate_sibling_fingerprints,
    state::{FileId, FileRegistry},
};

/// Corpus-scale complexity canary for the fold.
mod canary;
/// Byte-parity pins against the top-down construction.
mod fold_parity;

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
    let end = children.last().map_or(0, |last| last.byte_range.end);
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
fn corpus_fingerprints(
    tree: &NormalizedNode,
    language: &str,
    min_nodes: usize,
) -> Vec<Fingerprint> {
    let mut fingerprints = collect_non_boilerplate_fingerprints(tree, min_nodes, language);
    fingerprints.extend(collect_non_boilerplate_sibling_fingerprints(
        tree, min_nodes, language,
    ));
    fingerprints
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
