//! Embedding role guard applied before transitive closure
//! ([CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]).

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    cluster_filters::{is_embedding_role_mismatch, ParseCache},
    fingerprint::Fingerprint,
    state::FileId,
};

use super::{CandidatePair, EMBEDDING_SUPPORT_FLOOR};

/// Removes candidate edges carried by embedding evidence alone whose
/// endpoints play incompatible roles, so a topic-similar class/function
/// pair can never weld a component
/// ([CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]).
pub(crate) fn apply_embedding_role_guard<L: BuildHasher>(
    pairs: &mut Vec<CandidatePair>,
    fingerprints: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str, L>,
    cache: &ParseCache,
) {
    pairs.retain(|pair| {
        let (Some(left), Some(right)) =
            (fingerprints.get(pair.left), fingerprints.get(pair.right))
        else {
            return false;
        };
        !(embedding_needs_role_guard(pair)
            && is_embedding_role_mismatch(left, right, sources, languages, cache))
    });
}

/// Whether this pair needs embedding evidence rather than structural or
/// token evidence to clear its pair-specific admission floor.
fn embedding_needs_role_guard(pair: &CandidatePair) -> bool {
    pair.score.embedding_cos >= EMBEDDING_SUPPORT_FLOOR
        && pair.score.structural < pair.fused_min_score
        && pair.score.token_jaccard < pair.fused_min_score
}
