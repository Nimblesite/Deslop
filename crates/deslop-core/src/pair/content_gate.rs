//! Pair-content admission guard before transitive closure ([FUSED-CONTENT-GATE]).

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    ast::NormalizedNode,
    buckets::{CONTENT_PROMOTE_FLOOR, CONTENT_SUPPORT_FLOOR},
    content::measure_pair_content_indexed,
    fingerprint::Fingerprint,
    state::FileId,
};

use super::{CandidatePair, EMBEDDING_SUPPORT_FLOOR};

/// Structural overlap at which normalised shape saturates the content guard.
const SHAPE_IDENTICAL_FLOOR: f64 = 0.99;
/// Token overlap at which `MinHash` is echoing saturated normalised shape.
const SATURATING_TOKEN_FLOOR: f64 = 0.95;

/// Removes candidate edges that fail an applicable pair-content guard.
///
/// This runs after rescue measurement, so `shared_subtree_overlap` is the
/// measured `S` for rescue-eligible non-Merkle pairs, and before closure, so
/// a rejected relation can never weld two component members together.
pub(crate) fn apply_pair_content_gate<S, L>(
    pairs: &mut Vec<CandidatePair>,
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) where
    S: BuildHasher,
    L: BuildHasher,
{
    let tree_index: HashMap<FileId, &NormalizedNode> =
        trees.iter().map(|tree| (tree.file_id, tree)).collect();
    pairs.retain(|pair| {
        pair_passes_content_gate(pair, fingerprints, &tree_index, sources, languages)
    });
}

/// Applies the content guard to one candidate edge.
fn pair_passes_content_gate<S: BuildHasher, L: BuildHasher>(
    pair: &CandidatePair,
    fingerprints: &[Fingerprint],
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) -> bool {
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return false;
    };
    if !content_is_required(pair, left, right) {
        return true;
    }
    let evidence = measure_pair_content_indexed(left, right, tree_index, sources, languages);
    evidence.measured && evidence.support() >= content_floor(left, right)
}

/// Whether saturated normalised evidence lacks an independent semantic route.
fn content_is_required(pair: &CandidatePair, left: &Fingerprint, right: &Fingerprint) -> bool {
    pair.score.embedding_cos < EMBEDDING_SUPPORT_FLOOR
        && (left.hash == right.hash
            || pair.score.structural >= SHAPE_IDENTICAL_FLOOR
            || pair.shared_subtree_overlap >= SHAPE_IDENTICAL_FLOOR
            || pair.score.token_jaccard >= SATURATING_TOKEN_FLOOR)
}

/// Scope-specific content floor for this exact pair.
fn content_floor(left: &Fingerprint, right: &Fingerprint) -> f64 {
    if left.file_id == right.file_id {
        CONTENT_PROMOTE_FLOOR
    } else {
        CONTENT_SUPPORT_FLOOR
    }
}
