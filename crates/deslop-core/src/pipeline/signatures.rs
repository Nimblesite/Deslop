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
//!
//! ## Bottom-up fold ([PERF-FLUTTER-TODO-CORPUS])
//!
//! The historical construction resolved every fingerprint's byte range
//! from the file root and re-walked the resolved subtree to emit its
//! token stream — `O(fingerprints × tree)` per file, which measured
//! 602 of the Flutter corpus stage's 630 seconds. The fold below walks
//! each file once, carrying a composable [`TokenState`] up the tree:
//! the `MinHash` of a sequence is the element-wise minimum over its
//! k-grams, so a parent's signature is `min` over its children's plus
//! the few k-grams straddling their boundaries — recomputable from
//! each child's signature and its first/last `k-1` tokens alone. The
//! result is byte-identical to the top-down construction (pinned by
//! `fold_signatures_match_the_top_down_construction`), at `O(nodes)`
//! per file.

use std::collections::HashMap;

use crate::{
    ast::NormalizedNode,
    boilerplate::is_boilerplate,
    fingerprint::Fingerprint,
    lsh::{minhash_signature, Signature, SIGNATURE_LEN},
    sibling::MAX_WINDOW_WIDTH,
    state::FileId,
    tokens::{cross_language_token_stream_for_fingerprint, kgrams, KGRAM_WIDTH},
};

use fold::{join_states, TokenState};

/// The composable per-subtree fold state ([PIPELINE-SIGNATURE-FOLD]).
mod fold;

/// Builds a `FileId → &NormalizedNode` index to avoid O(files) linear scans
/// for every fingerprint in [`build_cross_language_signatures`].
fn build_tree_index(trees: &[NormalizedNode]) -> HashMap<FileId, &NormalizedNode> {
    trees.iter().map(|tree| (tree.file_id, tree)).collect()
}

/// One in-progress frame of the iterative signature fold: the node being
/// closed and its finished children's states, in order.
struct FoldFrame<'tree> {
    /// The node being closed.
    node: &'tree NormalizedNode,
    /// Index of the next child to fold into `children`.
    next_child: usize,
    /// Finished child states, in source order.
    children: Vec<TokenState>,
}

impl<'tree> FoldFrame<'tree> {
    /// Opens a frame over `node` with no children closed yet.
    const fn new(node: &'tree NormalizedNode) -> Self {
        Self {
            node,
            next_child: 0,
            children: Vec::new(),
        }
    }
}

/// Output positions of every fingerprint whose byte range is `(start, end)`.
/// A range can index several fingerprints (an exact node and a window that
/// cover the same bytes resolve to the same token stream), so the value is
/// a list.
type Positions = HashMap<(usize, usize), Vec<usize>>;

/// Builds the range → output-positions index for `fingerprints`.
fn positions_for(fingerprints: &[Fingerprint]) -> Positions {
    let mut positions: Positions = HashMap::with_capacity(fingerprints.len());
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        let key = (fingerprint.byte_range.start, fingerprint.byte_range.end);
        positions.entry(key).or_default().push(index);
    }
    positions
}

/// Which output positions the exact-node pass has already filled. The
/// top-down resolver answers a range that is both an exact node's and a
/// sibling window's — a tight wrapper covering exactly the window's
/// children — with the **node** (it checks node ranges before windows on
/// the way down), so the exact emission must win and the window emission
/// must leave those positions alone.
#[derive(Debug, Default)]
struct Filled(Vec<bool>);

impl Filled {
    /// Marks `index` filled.
    fn mark(&mut self, index: usize) {
        if let Some(slot) = self.0.get_mut(index) {
            *slot = true;
        }
    }

    /// True when `index` was marked.
    fn is_marked(&self, index: usize) -> bool {
        self.0.get(index).copied().unwrap_or(false)
    }
}

/// True when the token-stream fold must contribute nothing for `node`:
/// import/prologue carriers and import-only subtrees are skipped by the
/// language-aware top-down walk ([`crate::tokens`]), so the fold skips
/// them too — their bytes never enter any signature.
fn token_skipped(node: &NormalizedNode, language: Option<&str>) -> bool {
    is_boilerplate(language, node)
}

/// Closes the top fold frame: folds the node's own state over its
/// children, emits signatures for member ranges, and passes the state up.
fn close_fold_frame(
    stack: &mut Vec<FoldFrame<'_>>,
    language: Option<&str>,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    let Some(frame) = stack.pop() else {
        return;
    };
    let skipped = token_skipped(frame.node, language);
    let mut state = if skipped {
        TokenState::empty()
    } else {
        TokenState::singleton(frame.node.kind)
    };
    if !skipped {
        for child in &frame.children {
            state = join_states(&state, child);
        }
    }
    emit_exact_member(&frame, stack, &state, positions, filled, out);
    if language.is_some() {
        emit_window_members(&frame, positions, filled, out);
    }
    if let Some(parent) = stack.last_mut() {
        parent.children.push(state);
    }
}

/// Emits the signature for the node's own range when a fingerprint covers
/// it. Deferred to the parent when the parent owns the identical range:
/// the top-down resolver returns the shallowest matching node, so a
/// wrapper chain must answer from its outermost member.
fn emit_exact_member(
    frame: &FoldFrame<'_>,
    stack: &[FoldFrame<'_>],
    state: &TokenState,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    let Some(parent) = stack.last() else {
        emit_member(state, frame.node.byte_range, positions, filled, out);
        return;
    };
    if parent.node.byte_range == frame.node.byte_range {
        return;
    }
    emit_member(state, frame.node.byte_range, positions, filled, out);
}

/// Emits signatures for the sibling-window ranges of `frame`'s children
/// that fingerprints cover. Windows of width 2..=[`MAX_WINDOW_WIDTH`] are
/// enumerated exactly as [`crate::sibling`] enumerates the fingerprints
/// themselves; membership in `positions` is the fingerprint gate, so no
/// window that was never fingerprinted costs a fold. Only called on the
/// language-aware path: the language-agnostic resolver (`locate`) answers
/// exact nodes only, so window fingerprints there keep their fallback.
fn emit_window_members(
    frame: &FoldFrame<'_>,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    let child_count = frame.node.children.len();
    for width in 2..=MAX_WINDOW_WIDTH {
        for start in 0..child_count {
            let end = start.saturating_add(width);
            if end > child_count {
                break;
            }
            let Some(first) = frame.node.children.get(start) else {
                break;
            };
            let Some(last) = frame.node.children.get(end.saturating_sub(1)) else {
                break;
            };
            let range = (first.byte_range.start, last.byte_range.end);
            // A well-formed window spans bytes forward. Positionally
            // unordered children (impossible in a real parse) produce
            // inverted ranges the top-down resolver can never resolve —
            // those fingerprints keep their fallback, and the fold must
            // agree.
            if first.byte_range.start >= last.byte_range.end {
                continue;
            }
            if !positions.contains_key(&range) {
                continue;
            }
            fold_window_state(&frame.children, start, end, range, positions, filled, out);
        }
    }
}

/// Folds the member states of one window and emits its signature.
fn fold_window_state(
    children: &[TokenState],
    start: usize,
    end: usize,
    range: (usize, usize),
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    let mut state = TokenState::empty();
    for member in children.get(start..end).unwrap_or(&[]) {
        state = join_states(&state, member);
    }
    emit_member_range(&state, range, positions, filled, out);
}

/// Emits `state`'s signature for every fingerprint position covering
/// `byte_range` when the stream is long enough, leaving the
/// fingerprint-scoped fallback otherwise (short streams carry no k-grams).
fn emit_member(
    state: &TokenState,
    byte_range: crate::ast::ByteRange,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    emit_member_range(
        state,
        (byte_range.start, byte_range.end),
        positions,
        filled,
        out,
    );
}

/// Range-keyed emission half of [`emit_member`].
fn emit_member_range(
    state: &TokenState,
    range: (usize, usize),
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    if state.count < KGRAM_WIDTH {
        return;
    }
    if let Some(indexes) = positions.get(&range) {
        for &index in indexes {
            if filled.is_marked(index) {
                continue;
            }
            filled.mark(index);
            if let Some(slot) = out.get_mut(index) {
                *slot = state.signature;
            }
        }
    }
}

/// Builds one file's `MinHash` signatures, positionally 1:1 with
/// `fingerprints`. Called at parse/load time so the result is persisted
/// in the parse store beside the fingerprints it was built from and
/// reattached on later cache hits
/// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
///
/// Every fingerprint starts at its fingerprint-scoped
/// [`fallback_signature`] — the correct signature for a stream too short
/// to hold a k-gram, and the only signature an unresolvable range can
/// have — and the fold overwrites exactly the positions whose ranges
/// resolve to a token stream with k-grams in it. The output therefore
/// matches the historical per-fingerprint construction for every input,
/// including hand-built fingerprint lists the corpus never produces.
#[must_use]
pub fn signatures_for_file(
    tree: &NormalizedNode,
    fingerprints: &[Fingerprint],
    language: Option<&str>,
) -> Vec<Signature> {
    let mut out: Vec<Signature> = fingerprints.iter().map(fallback_signature).collect();
    let mut filled = Filled(vec![false; fingerprints.len()]);
    let positions = positions_for(fingerprints);
    let mut stack = vec![FoldFrame::new(tree)];
    while let Some(frame) = stack.last_mut() {
        if let Some(child) = frame.node.children.get(frame.next_child) {
            frame.next_child = frame.next_child.saturating_add(1);
            stack.push(FoldFrame::new(child));
            continue;
        }
        close_fold_frame(&mut stack, language, &positions, &mut filled, &mut out);
    }
    out
}

/// The historical top-down construction of one fingerprint's signature,
/// kept as the reference the fold must reproduce byte-for-byte
/// (`fold_signatures_match_the_top_down_construction`).
#[cfg(test)]
fn top_down_signature(
    root: &NormalizedNode,
    fingerprint: &Fingerprint,
    language: Option<&str>,
) -> Signature {
    let tokens = language.map_or_else(
        || crate::tokens::token_stream_for_fingerprint(root, fingerprint),
        |language| {
            crate::tokens::token_stream_for_fingerprint_with_language(root, fingerprint, language)
        },
    );
    tokens.map_or_else(
        || fallback_signature(fingerprint),
        |tokens| signature_for_tokens(&tokens, fingerprint),
    )
}

/// Builds aliases-only signatures for explicit cross-language audits.
#[must_use]
pub fn build_cross_language_signatures<S: std::hash::BuildHasher>(
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
        return fallback_signature(fingerprint);
    };
    let Some(root) = tree_index.get(&fingerprint.file_id).copied() else {
        return fallback_signature(fingerprint);
    };
    let tokens = cross_language_token_stream_for_fingerprint(root, fingerprint, language);
    tokens.map_or_else(
        || fallback_signature(fingerprint),
        |tokens| signature_for_tokens(&tokens, fingerprint),
    )
}

/// Produces a signature from a prepared token stream using the configured
/// k-gram width.
fn signature_for_tokens(tokens: &[&'static str], fingerprint: &Fingerprint) -> Signature {
    if tokens.len() < KGRAM_WIDTH {
        return fallback_signature(fingerprint);
    }
    minhash_signature(&kgrams(tokens, KGRAM_WIDTH))
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
    let mut hasher = blake3::Hasher::new();
    let _ = hasher.update(&fingerprint.hash);
    let _ = hasher.update(&start.to_le_bytes());
    let _ = hasher.update(&end.to_le_bytes());
    let mut expanded = [0_u8; SIGNATURE_LEN * 8];
    hasher.finalize_xof().fill(&mut expanded);
    decode_slots(&expanded)
}

/// Decodes an XOF byte stream into signature slots, little-endian.
fn decode_slots(expanded: &[u8]) -> Signature {
    let mut signature = [0_u64; SIGNATURE_LEN];
    for (slot, chunk) in signature.iter_mut().zip(expanded.chunks_exact(8)) {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(chunk);
        *slot = u64::from_le_bytes(bytes);
    }
    signature
}

#[cfg(test)]
mod tests;
