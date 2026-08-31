//! Exhaustive admission algebra for one explicit pair.

use crate::{
    buckets::{content_support, CONTENT_PROMOTE_FLOOR, CONTENT_SUPPORT_FLOOR},
    pair::{
        CROSS_LANGUAGE_MIN_JACCARD, EMBEDDING_SUPPORT_FLOOR, FUSED_THRESHOLD, LSH_ONLY_MIN_JACCARD,
        LSH_ONLY_MIN_NODE_COUNT, MAX_ENDPOINT_NODE_RATIO, RESCUE_MIN_CONTENT_AGREEMENT,
        SHARED_SUBTREE_MIN_JACCARD, SHARED_SUBTREE_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_OVERLAP,
    },
    report::PairClassification,
};

use super::{Measurements, ResolvedPair};
use crate::pipeline::PipelineSession;

/// Structural overlap at which normalised shape saturates the content guard.
const SHAPE_IDENTICAL_FLOOR: f64 = 0.99;
/// Token overlap at which the normalised token axis echoes saturated shape.
const SATURATING_TOKEN_FLOOR: f64 = 0.95;

/// Fully evaluated pair-admission predicates.
pub(super) struct AdmissionFacts {
    /// Whether content corroboration is required.
    pub(super) content_required: bool,
    /// Whether every applicable content guard passed.
    pub(super) content_ok: bool,
    /// Final pair-edge verdict.
    pub(super) admitted: bool,
    /// First failed guard in specification order.
    failure: Option<AdmissionFailure>,
}

impl AdmissionFacts {
    /// Evaluates the exhaustive admission algebra from the pair measurements.
    pub(super) fn from(
        session: &PipelineSession,
        pair: &ResolvedPair<'_>,
        measured: Measurements,
    ) -> Self {
        let policy = PairPolicy::from(session, pair, measured);
        let content_required = content_required(measured);
        let content_ok =
            !content_required || pair_content_support(measured) >= policy.content_floor;
        let failure = policy
            .failure
            .or((!content_ok).then_some(AdmissionFailure::Content));
        Self {
            content_required,
            content_ok,
            admitted: failure.is_none(),
            failure,
        }
    }

    /// Classifies this pair only; rejected shape-only pairs remain explicit.
    pub(super) fn classification(&self, measured: Measurements) -> Option<PairClassification> {
        if measured.byte_identical {
            return Some(PairClassification::Identical);
        }
        if self.content_required && !self.content_ok {
            return Some(PairClassification::StructuralOnly);
        }
        classify_admitted(self.admitted, measured)
    }

    /// Human-readable result derived from the same predicates as `admitted`.
    pub(super) fn explanation(&self, classification: Option<PairClassification>) -> String {
        match (self.admitted, classification) {
            (true, Some(PairClassification::Identical)) => {
                "admitted: exact pair clears every admission guard".to_owned()
            }
            (true, _) => "admitted: explicit pair clears every admission guard".to_owned(),
            (false, Some(PairClassification::StructuralOnly)) => {
                "rejected: saturated normalised evidence lacks required pair content support"
                    .to_owned()
            }
            (false, _) => self.rejection_reason().to_owned(),
        }
    }

    /// First failed non-content guard in the specified admission order.
    fn rejection_reason(&self) -> &'static str {
        match self.failure {
            Some(AdmissionFailure::Language) => {
                "rejected: cross-language comparison is not enabled"
            }
            Some(AdmissionFailure::Size) => "rejected: endpoint sizes are incoherent",
            Some(AdmissionFailure::Lsh) => "rejected: pair fails the LSH-only guards",
            Some(AdmissionFailure::Score) => {
                "rejected: pair clears neither its threshold nor rescue"
            }
            Some(AdmissionFailure::Content) | None => "rejected: pair fails content corroboration",
        }
    }
}

/// First failed admission guard, in specification order.
#[derive(Clone, Copy)]
enum AdmissionFailure {
    /// Cross-language comparison is disabled.
    Language,
    /// Endpoint sizes are incoherent.
    Size,
    /// LSH-only evidence misses its floors.
    Lsh,
    /// Neither the ordinary threshold nor rescue passed.
    Score,
    /// Required pair content support is missing.
    Content,
}

/// Policy predicates shared while evaluating one pair.
struct PairPolicy {
    /// First failed non-content guard.
    failure: Option<AdmissionFailure>,
    /// Content floor for this scope.
    content_floor: f64,
}

impl PairPolicy {
    /// Computes non-content gates and the scope-specific content floor.
    fn from(session: &PipelineSession, pair: &ResolvedPair<'_>, measured: Measurements) -> Self {
        let cross_language = session.file_languages.get(&pair.left.fingerprint.file_id)
            != session.file_languages.get(&pair.right.fingerprint.file_id);
        let explicit_cross_language =
            cross_language && session.exclusion.allows_cross_language_comparison();
        let threshold = if explicit_cross_language && !measured.merkle_equal {
            CROSS_LANGUAGE_MIN_JACCARD
        } else {
            FUSED_THRESHOLD
        };
        let rescue = rescue_applies(pair, measured, threshold);
        let failure = first_policy_failure([
            (
                !cross_language || explicit_cross_language,
                AdmissionFailure::Language,
            ),
            (
                size_is_coherent(pair, measured.merkle_equal),
                AdmissionFailure::Size,
            ),
            (
                lsh_guard(pair, measured, rescue, explicit_cross_language),
                AdmissionFailure::Lsh,
            ),
            (
                measured.score.bounded_fused() >= threshold || rescue,
                AdmissionFailure::Score,
            ),
        ]);
        Self {
            failure,
            content_floor: if pair.cross_file() {
                CONTENT_SUPPORT_FLOOR
            } else {
                CONTENT_PROMOTE_FLOOR
            },
        }
    }
}

/// First failed non-content admission predicate in specification order.
fn first_policy_failure(results: [(bool, AdmissionFailure); 4]) -> Option<AdmissionFailure> {
    results
        .into_iter()
        .find_map(|(passed, failure)| (!passed).then_some(failure))
}

/// Whether saturated normalised evidence needs content corroboration.
fn content_required(measured: Measurements) -> bool {
    measured.score.embedding_cos < EMBEDDING_SUPPORT_FLOOR
        && (measured.merkle_equal
            || measured.score.structural >= SHAPE_IDENTICAL_FLOOR
            || measured.score.token_jaccard >= SATURATING_TOKEN_FLOOR)
}

/// Pair-content support, never a cluster quantity.
fn pair_content_support(measured: Measurements) -> f64 {
    content_support(measured.agreement, measured.rename_consistency)
}

/// Whether the below-threshold cross-file rescue admits this pair.
fn rescue_applies(pair: &ResolvedPair<'_>, measured: Measurements, threshold: f64) -> bool {
    pair.cross_file()
        && measured.score.bounded_fused() < threshold
        && measured.score.structural >= SHARED_SUBTREE_MIN_OVERLAP
        && measured.score.token_jaccard >= SHARED_SUBTREE_MIN_JACCARD
        && smaller_node_count(pair) >= SHARED_SUBTREE_MIN_NODE_COUNT
        && measured.agreement >= RESCUE_MIN_CONTENT_AGREEMENT
}

/// Size coherence for pairs without an exact Merkle anchor.
fn size_is_coherent(pair: &ResolvedPair<'_>, merkle_equal: bool) -> bool {
    let (smaller, larger) = node_counts(pair);
    merkle_equal || larger <= smaller.saturating_mul(MAX_ENDPOINT_NODE_RATIO)
}

/// Applies the pair-specific LSH-only floors.
fn lsh_guard(
    pair: &ResolvedPair<'_>,
    measured: Measurements,
    rescue: bool,
    explicit_cross_language: bool,
) -> bool {
    let lsh_only = !measured.merkle_equal && measured.score.embedding_cos <= 0.0 && !rescue;
    let jaccard_floor = if explicit_cross_language {
        CROSS_LANGUAGE_MIN_JACCARD
    } else {
        LSH_ONLY_MIN_JACCARD
    };
    let node_floor = if explicit_cross_language {
        smaller_node_count(pair).max(LSH_ONLY_MIN_NODE_COUNT)
    } else {
        smaller_node_count(pair)
    };
    !lsh_only
        || (measured.score.token_jaccard >= jaccard_floor && node_floor >= LSH_ONLY_MIN_NODE_COUNT)
}

/// Endpoint node counts as `(smaller, larger)`.
fn node_counts(pair: &ResolvedPair<'_>) -> (usize, usize) {
    let left = pair.left.fingerprint.node_count;
    let right = pair.right.fingerprint.node_count;
    (left.min(right), left.max(right))
}

/// Smaller endpoint node count.
fn smaller_node_count(pair: &ResolvedPair<'_>) -> usize {
    node_counts(pair).0
}

/// Presentation classification for a pair that survived admission.
fn classify_admitted(admitted: bool, measured: Measurements) -> Option<PairClassification> {
    if !admitted {
        return None;
    }
    if measured.score.embedding_cos >= EMBEDDING_SUPPORT_FLOOR
        && measured.score.structural < SHARED_SUBTREE_MIN_OVERLAP
        && measured.score.token_jaccard < SHARED_SUBTREE_MIN_JACCARD
    {
        return Some(PairClassification::SameBehavior);
    }
    if measured.score.structural >= SHARED_SUBTREE_MIN_OVERLAP {
        return Some(PairClassification::NearlyIdentical);
    }
    Some(PairClassification::LooselySimilar)
}
