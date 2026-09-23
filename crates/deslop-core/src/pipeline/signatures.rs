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

use std::collections::{HashMap, HashSet};

use fold::{join_states, TokenState};

use crate::{
    ast::NormalizedNode,
    boilerplate::is_boilerplate,
    fingerprint::Fingerprint,
    lsh::{minhash_signature, Signature, SIGNATURE_LEN},
    sibling::MAX_WINDOW_WIDTH,
    state::FileId,
    tokens::{cross_language_token_stream_for_fingerprint, kgrams, KGRAM_WIDTH},
};

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
    children: Vec<FoldedChild>,
    /// Structural mass, including the current node and finished children.
    node_count: usize,
}

/// A finished child contributes both token evidence and structural mass.
struct FoldedChild {
    /// Token signature and boundary state of this child.
    tokens: TokenState,
    /// Structural mass independent of language-aware token skipping.
    node_count: usize,
}

impl<'tree> FoldFrame<'tree> {
    /// Opens a frame over `node` with no children closed yet.
    const fn new(node: &'tree NormalizedNode) -> Self {
        Self {
            node,
            next_child: 0,
            children: Vec::new(),
            node_count: 1,
        }
    }
}

/// A byte span and structural mass identify a candidate without conflating
/// a grammar wrapper with its same-range child.
type MemberKey = (usize, usize, usize);

/// Output positions indexed by span and structural mass.
type Positions = HashMap<MemberKey, Vec<usize>>;

/// Builds the range → output-positions index for `fingerprints`.
fn positions_for(fingerprints: &[Fingerprint]) -> Positions {
    let mut positions: Positions = HashMap::with_capacity(fingerprints.len());
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        let key = (
            fingerprint.byte_range.start,
            fingerprint.byte_range.end,
            fingerprint.node_count,
        );
        positions.entry(key).or_default().push(index);
    }
    positions
}

/// Which positions are filled, and which keys have multiple same-mass
/// candidates that require exact hash-aware resolution after the fold.
#[derive(Debug, Default)]
struct Filled {
    /// Which fingerprint positions received a folded signature.
    positions: Vec<bool>,
    /// Spans with more than one same-mass candidate in the tree.
    ambiguous: HashSet<MemberKey>,
}

impl Filled {
    /// Marks `index` filled.
    fn mark(&mut self, index: usize) {
        if let Some(slot) = self.positions.get_mut(index) {
            *slot = true;
        }
    }

    /// True when `index` was marked.
    fn is_marked(&self, index: usize) -> bool {
        self.positions.get(index).copied().unwrap_or(false)
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
    let state = frame_token_state(&frame, language);
    emit_member(
        &state,
        frame.node.byte_range,
        frame.node_count,
        positions,
        filled,
        out,
    );
    emit_window_members(&frame, positions, filled, out);
    if let Some(parent) = stack.last_mut() {
        parent.node_count = parent.node_count.saturating_add(frame.node_count);
        parent.children.push(FoldedChild {
            tokens: state,
            node_count: frame.node_count,
        });
    }
}

/// Joins the node token with its children, except a skipped prologue.
fn frame_token_state(frame: &FoldFrame<'_>, language: Option<&str>) -> TokenState {
    let skipped = token_skipped(frame.node, language);
    let mut state = if skipped {
        TokenState::empty()
    } else {
        TokenState::singleton(frame.node.kind)
    };
    if !skipped {
        for child in &frame.children {
            state = join_states(&state, &child.tokens);
        }
    }
    state
}

/// Emits signatures for the sibling-window ranges of `frame`'s children
/// that fingerprints cover. Windows of width 2..=[`MAX_WINDOW_WIDTH`] are
/// enumerated exactly as [`crate::sibling`] enumerates the fingerprints
/// themselves; membership in `positions` is the fingerprint gate, so no
/// window that was never fingerprinted costs a fold.
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
            emit_window_from(frame, start, end, positions, filled, out);
        }
    }
}

/// Emits one well-formed child run when a fingerprint names its span/mass.
fn emit_window_from(
    frame: &FoldFrame<'_>,
    start: usize,
    end: usize,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    let (Some(first), Some(last)) = (
        frame.node.children.get(start),
        frame.node.children.get(end.saturating_sub(1)),
    ) else {
        return;
    };
    if first.byte_range.start >= last.byte_range.end {
        return;
    }
    let mass = window_node_count(&frame.children, start, end);
    let key = (first.byte_range.start, last.byte_range.end, mass);
    if positions.contains_key(&key) {
        fold_window_state(&frame.children, start, end, key, positions, filled, out);
    }
}

/// Structural mass of the child run being folded.
fn window_node_count(children: &[FoldedChild], start: usize, end: usize) -> usize {
    children
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .fold(0, |sum, child| sum.saturating_add(child.node_count))
}

/// Folds the member states of one window and emits its signature.
fn fold_window_state(
    children: &[FoldedChild],
    start: usize,
    end: usize,
    key: MemberKey,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    let mut state = TokenState::empty();
    for member in children.get(start..end).unwrap_or(&[]) {
        state = join_states(&state, &member.tokens);
    }
    emit_member_range(&state, key, positions, filled, out);
}

/// Emits `state`'s signature for every fingerprint position covering
/// `byte_range` when the stream is long enough, leaving the
/// fingerprint-scoped fallback otherwise (short streams carry no k-grams).
fn emit_member(
    state: &TokenState,
    byte_range: crate::ast::ByteRange,
    node_count: usize,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    emit_member_range(
        state,
        (byte_range.start, byte_range.end, node_count),
        positions,
        filled,
        out,
    );
}

/// Range-keyed emission half of [`emit_member`].
fn emit_member_range(
    state: &TokenState,
    key: MemberKey,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    if state.count < KGRAM_WIDTH {
        return;
    }
    if let Some(indexes) = positions.get(&key) {
        for &index in indexes {
            if filled.is_marked(index) {
                let _inserted = filled.ambiguous.insert(key);
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
/// have — and the fold overwrites positions whose range and structural
/// mass identify a token stream with k-grams in it. Same-span/same-mass
/// ambiguities take the hash-aware resolver to preserve exact identity.
#[must_use]
pub fn signatures_for_file(
    tree: &NormalizedNode,
    fingerprints: &[Fingerprint],
    language: Option<&str>,
) -> Vec<Signature> {
    let mut out: Vec<Signature> = fingerprints.iter().map(fallback_signature).collect();
    let mut filled = Filled {
        positions: vec![false; fingerprints.len()],
        ..Filled::default()
    };
    let positions = positions_for(fingerprints);
    fold_file(tree, language, &positions, &mut filled, &mut out);
    resolve_ambiguous_signatures(tree, fingerprints, language, &positions, &filled, &mut out);
    out
}

/// Walks one tree bottom-up and emits its folded token signatures.
fn fold_file(
    tree: &NormalizedNode,
    language: Option<&str>,
    positions: &Positions,
    filled: &mut Filled,
    out: &mut [Signature],
) {
    let mut stack = vec![FoldFrame::new(tree)];
    while let Some(frame) = stack.last_mut() {
        if let Some(child) = frame.node.children.get(frame.next_child) {
            frame.next_child = frame.next_child.saturating_add(1);
            stack.push(FoldFrame::new(child));
            continue;
        }
        close_fold_frame(&mut stack, language, positions, filled, out);
    }
}

/// Rechecks rare same-span/same-mass candidates by hash, keeping the
/// ordinary unambiguous population on the linear fold.
fn resolve_ambiguous_signatures(
    tree: &NormalizedNode,
    fingerprints: &[Fingerprint],
    language: Option<&str>,
    positions: &Positions,
    filled: &Filled,
    out: &mut [Signature],
) {
    for (key, indexes) in positions {
        if indexes.len() < 2 && !filled.ambiguous.contains(key) {
            continue;
        }
        for &index in indexes {
            if let (Some(fingerprint), Some(slot)) = (fingerprints.get(index), out.get_mut(index)) {
                *slot = top_down_signature(tree, fingerprint, language);
            }
        }
    }
}

/// Exact construction for ambiguous spans, also the fold's test reference.
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
pub(crate) fn cross_language_signature(
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

/// [PIPELINE-SIGNATURE-FALLBACK] Fingerprint-scoped signature when no k-grams exist. This
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
