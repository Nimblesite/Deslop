//! Pair-content admission guard before transitive closure ([FUSED-CONTENT-GATE]).

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    ast::NormalizedNode,
    buckets::{CONTENT_PROMOTE_FLOOR, CONTENT_SUPPORT_FLOOR},
    cluster_filters::{is_embedding_role_mismatch, ParseCache},
    content::measure_pair_content_indexed,
    fingerprint::Fingerprint,
    state::FileId,
};

use super::{
    CandidatePair, EMBEDDING_SUPPORT_FLOOR, LSH_ONLY_MIN_JACCARD, SHARED_SUBTREE_MIN_OVERLAP,
};

/// Structural overlap at which normalised shape saturates the content guard.
const SHAPE_IDENTICAL_FLOOR: f64 = 0.99;
/// Token overlap at which `MinHash` is echoing saturated normalised shape.
const SATURATING_TOKEN_FLOOR: f64 = 0.95;
/// The content guard applies at shape saturation ([FUSED-CONTENT-GATE]
/// step 3), so the gate floor stays at `SHAPE_IDENTICAL_FLOOR`.
///
/// Removes candidate edges that fail an applicable pair-content guard.
///
/// This runs after rescue measurement, so `shared_subtree_overlap` is the
/// measured `S` for rescue-eligible non-Merkle pairs, and before closure, so
/// a rejected relation can never weld two component members together.
pub(crate) fn apply_pair_content_gate<L>(
    pairs: &mut Vec<CandidatePair>,
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str, L>,
    cache: &ParseCache,
) where
    L: BuildHasher,
{
    let tree_index: HashMap<FileId, &NormalizedNode> =
        trees.iter().map(|tree| (tree.file_id, tree)).collect();
    pairs.retain(|pair| {
        pair_passes_content_gate(pair, fingerprints, &tree_index, sources, languages, cache)
    });
}

/// Applies the content guard to one candidate edge.
fn pair_passes_content_gate<L: BuildHasher>(
    pair: &CandidatePair,
    fingerprints: &[Fingerprint],
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str, L>,
    cache: &ParseCache,
) -> bool {
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return false;
    };

    if embedding_needs_role_guard(pair)
        && is_embedding_role_mismatch(left, right, sources, languages, cache)
    {
        return false;
    }
    let required = content_is_required(pair, left, right);
    let evidence = measure_pair_content_indexed(left, right, tree_index, sources, languages);
    let verdict = !required || (evidence.measured && evidence.support() >= content_floor(pair, left, right));
    eprintln!("PROBE L={:?}:{}-{}({}) R={:?}:{}-{}({}) hash_eq={} S={:.3} J={:.3} shared={:.3} req={} A={:.3} R={:.3} floor={:.2} verdict={}",
        left.file_id, left.byte_range.start, left.byte_range.end, left.node_count, right.file_id, right.byte_range.start, right.byte_range.end, right.node_count,
        left.hash == right.hash, pair.score.structural, pair.score.token_jaccard, pair.shared_subtree_overlap, required,
        evidence.agreement, evidence.rename_consistency, content_floor(pair, left, right), verdict);
    verdict
}

/// Whether this pair needs embedding evidence rather than structural or
/// token evidence to clear its configured pair-specific admission floor.
fn embedding_needs_role_guard(pair: &CandidatePair) -> bool {
    pair.score.embedding_cos >= EMBEDDING_SUPPORT_FLOOR
        && pair.score.structural < pair.fused_min_score
        && pair.score.token_jaccard < pair.fused_min_score
}

/// Whether saturated normalised evidence lacks an independent semantic route.
fn content_is_required(pair: &CandidatePair, left: &Fingerprint, right: &Fingerprint) -> bool {
    pair.score.embedding_cos < EMBEDDING_SUPPORT_FLOOR
        && (left.hash == right.hash
            || pair.score.structural >= SHAPE_IDENTICAL_FLOOR
            || pair.shared_subtree_overlap >= SHAPE_IDENTICAL_FLOOR
            || pair.score.token_jaccard >= SATURATING_TOKEN_FLOOR
            || lsh_only_pair_needs_content(pair, left, right))
}

/// Whether an unanchored LSH-only pair needs authored-content corroboration.
///
/// Its token score has already met the strict LSH-only floor, but that is
/// still normalised evidence. Without a Merkle anchor, embedding support, or
/// shared-subtree rescue, raw content is the pair-local discriminator.
fn lsh_only_pair_needs_content(
    pair: &CandidatePair,
    left: &Fingerprint,
    right: &Fingerprint,
) -> bool {
    left.hash != right.hash
        && pair.score.embedding_cos < EMBEDDING_SUPPORT_FLOOR
        && pair.shared_subtree_overlap < SHARED_SUBTREE_MIN_OVERLAP
        && pair.score.token_jaccard >= LSH_ONLY_MIN_JACCARD
}

/// Scope-specific content floor for this exact pair.
///
/// An unanchored LSH-only pair pays the promote floor in every scope:
/// with no structural anchor, no embedding support, and no
/// shared-subtree alignment, the token echo is its whole case, and a
/// token echo must be corroborated as strongly as a same-file
/// promotion before it may weld two views into one closure
/// ([FUSED-CONTENT-GATE]). The whole-file-against-interior-window
/// pairs of the #339 corpus ride exactly this route at cross-file
/// support strength and manufacture mixed-extent clusters.
fn content_floor(pair: &CandidatePair, left: &Fingerprint, right: &Fingerprint) -> f64 {
    if left.file_id == right.file_id || lsh_only_pair_needs_content(pair, left, right) {
        CONTENT_PROMOTE_FLOOR
    } else {
        CONTENT_SUPPORT_FLOOR
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        ast::ByteRange,
        cluster_filters::ParseCache,
        fingerprint::Fingerprint,
        pair::{CandidatePair, PairScore},
        state::{FileId, FileRegistry},
    };

    use super::apply_pair_content_gate;

    const PYTHON: &str = "python";
    const RUST: &str = "rust";
    const TYPE_SOURCE: &str = "struct RequestEnvelope { id: u64 }\n";
    const FUNCTION_SOURCE: &str = "fn build_request(id: u64) -> u64 { id }\n";
    const EMBEDDING_MEGA_TYPE_SOURCE: &str = "class TestSandboxEmbodiedLiveEditHttp:\n    async def test_api_file_patch_is_observable_in_next_chat_turn(self, client):\n        assert client\n";
    const EMBEDDING_MEGA_FUNCTION_SOURCE: &str = "async def test_workspace_status(client, workspace):\n    response = await client.get('/status')\n    assert response.status_code == 200\n";
    const EMBEDDING_EVIDENCE: f64 = 0.90;
    const NODE_COUNT: usize = 40;

    #[test]
    fn embedding_role_mismatch_is_rejected_before_closure() {
        let (fingerprints, sources, languages) = role_fixture(RUST, TYPE_SOURCE, FUNCTION_SOURCE);
        let mut pairs = vec![embedding_pair()];
        assert_eq!(
            pairs.len(),
            1,
            "the fixture must begin with one candidate edge"
        );
        apply_pair_content_gate(
            &mut pairs,
            &fingerprints,
            &[],
            &sources,
            &languages,
            &ParseCache::new(),
        );
        assert!(
            pairs.is_empty(),
            "a type/function pair carried only by embedding evidence must not reach closure"
        );
    }

    #[test]
    fn embedding_same_role_pair_reaches_closure() {
        let (fingerprints, sources, languages) =
            role_fixture(RUST, FUNCTION_SOURCE, FUNCTION_SOURCE);
        let mut pairs = vec![embedding_pair()];
        assert_eq!(
            pairs.len(),
            1,
            "the fixture must begin with one candidate edge"
        );
        apply_pair_content_gate(
            &mut pairs,
            &fingerprints,
            &[],
            &sources,
            &languages,
            &ParseCache::new(),
        );
        assert_eq!(
            pairs.len(),
            1,
            "the role guard must not reject two function endpoints"
        );
    }

    /// [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] A topic-similar class/function
    /// edge may not weld an unrelated embedding-discovered component.
    #[test]
    fn embedding_mega_class_function_edge_is_rejected_before_closure() {
        let (fingerprints, sources, languages) = role_fixture(
            PYTHON,
            EMBEDDING_MEGA_TYPE_SOURCE,
            EMBEDDING_MEGA_FUNCTION_SOURCE,
        );
        let mut pairs = vec![embedding_pair()];
        assert_eq!(
            pairs.len(),
            1,
            "the fixture must begin with one candidate edge"
        );
        apply_pair_content_gate(
            &mut pairs,
            &fingerprints,
            &[],
            &sources,
            &languages,
            &ParseCache::new(),
        );
        assert!(
            pairs.is_empty(),
            "a class/function edge must be rejected before it can form an embedding component"
        );
    }

    /// Fixture corpus for the role-gate unit test: fingerprints, sources
    /// and languages, in that order.
    type RoleCorpus = (
        Vec<Fingerprint>,
        HashMap<FileId, Vec<u8>>,
        HashMap<FileId, &'static str>,
    );

    fn role_fixture(language: &'static str, left_source: &str, right_source: &str) -> RoleCorpus {
        let mut registry = FileRegistry::new();
        let left = registry.register("left.rs".into());
        let right = registry.register("right.rs".into());
        let fingerprints = vec![
            fingerprint(left, left_source.len()),
            fingerprint(right, right_source.len()),
        ];
        let sources = HashMap::from([
            (left, left_source.as_bytes().to_vec()),
            (right, right_source.as_bytes().to_vec()),
        ]);
        let languages = HashMap::from([(left, language), (right, language)]);
        (fingerprints, sources, languages)
    }

    fn fingerprint(file_id: FileId, end: usize) -> Fingerprint {
        Fingerprint {
            hash: [u8::try_from(end).unwrap_or(u8::MAX); 32],
            file_id,
            byte_range: ByteRange { start: 0, end },
            node_count: NODE_COUNT,
        }
    }

    fn embedding_pair() -> CandidatePair {
        CandidatePair {
            left: 0,
            right: 1,
            endpoint_node_counts: (NODE_COUNT, NODE_COUNT),
            lsh_only_node_floor: NODE_COUNT,
            lsh_only_min_jaccard: 0.0,
            fused_min_score: 0.85,
            shared_subtree_overlap: 0.0,
            score: PairScore {
                structural: 0.0,
                token_jaccard: 0.0,
                embedding_cos: EMBEDDING_EVIDENCE,
            },
        }
    }
}
