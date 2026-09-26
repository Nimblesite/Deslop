//! Exact conditions under which aligned-core evidence can affect a pair.

use super::{CoreCopy, Measurements, PairAxes, ResolvedPair};
use crate::{
    content::{ContentPair, PairScope},
    overlap::judge_core,
    pair::{SHARED_SUBTREE_MIN_JACCARD, SHARED_SUBTREE_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_OVERLAP},
    pipeline::PipelineSession,
    report::PairClassification,
};

/// [FUSED-SHARED-SUBTREE-CORE] Reads the shared code through the content gate.
pub(super) fn verdict(
    session: &PipelineSession,
    pair: &ResolvedPair<'_>,
    axes: &mut PairAxes<'_>,
    content: &ContentPair,
) -> CoreCopy {
    let core = axes
        .overlap
        .aligned_core(pair.left.fingerprint, pair.right.fingerprint);
    let scope = PairScope {
        same_file: !pair.cross_file(),
        interior: false,
        core: true,
    };
    let floor = usize::try_from(session.min_nodes).unwrap_or(usize::MAX);
    let measured = judge_core(&core, floor, || {
        content.core(&core, &session.sources, scope)
    });
    CoreCopy::from(measured.copy, measured.evidence.contradiction)
}

/// [FUSED-SHARED-SUBTREE-CORE-NEED] Whether any admission or category
/// branch can still read an aligned-core verdict for this measured pair.
pub(super) fn required(
    measured: Measurements,
    smaller_nodes: usize,
    admitted_without_core: bool,
    category_without_core: Option<PairClassification>,
) -> bool {
    rescue_can_read_core(measured, smaller_nodes)
        || if admitted_without_core {
            category_without_core.is_none()
        } else {
            matches!(
                category_without_core,
                Some(PairClassification::StructuralOnly)
            )
        }
}

/// Rescue is the only admission path that reads aligned-core evidence.
fn rescue_can_read_core(measured: Measurements, smaller_nodes: usize) -> bool {
    measured.rescue_scope
        && measured.score.structural >= SHARED_SUBTREE_MIN_OVERLAP
        && measured.score.token_jaccard >= SHARED_SUBTREE_MIN_JACCARD
        && smaller_nodes >= SHARED_SUBTREE_MIN_NODE_COUNT
}

#[cfg(test)]
mod tests {
    use super::super::{admission::classify_admitted, CoreCopy, Measurements, SourceIdentity};
    use super::required;
    use crate::{
        config::RoutingTuning,
        content::{ContentContradiction, ContentEvidence},
        pair::{
            PairScore, EMBEDDING_SUPPORT_FLOOR, SHARED_SUBTREE_MIN_JACCARD,
            SHARED_SUBTREE_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_OVERLAP,
        },
        report::{PairClassification, PairTextIdentity},
    };

    const LOW_STRUCTURAL: f64 = 0.2;
    const LOW_TOKEN: f64 = 0.4;
    const SUPPORTED_CONTENT: f64 = 0.8;
    const WEAK_CONTENT: f64 = 0.1;
    const NEGLIGIBLE_CONTENT: f64 = 0.0;
    const FEW_NODES: usize = 1;

    fn measured(content_support: f64) -> Measurements {
        Measurements {
            score: PairScore {
                structural: LOW_STRUCTURAL,
                token_jaccard: LOW_TOKEN,
                embedding_cos: 0.0,
            },
            content: ContentEvidence {
                agreement: content_support,
                rename_consistency: 0.0,
                consistent_rename: false,
                literal_fraction: 0.0,
                measured: true,
                contradiction: ContentContradiction::None,
            },
            core: CoreCopy::Absent,
            rescue_scope: false,
            merkle_equal: false,
            text: SourceIdentity {
                raw: PairTextIdentity::Different,
                identical: false,
            },
        }
    }

    fn admitted_need(measured: Measurements, smaller_nodes: usize) -> bool {
        let category = classify_admitted(measured, RoutingTuning::default());
        required(measured, smaller_nodes, true, category)
    }

    // [FUSED-SHARED-SUBTREE-CORE-NEED] Content already determines a
    // general clone category and no rescue or shape-only route remains.
    #[test]
    fn supported_unrescued_pair_does_not_need_an_aligned_core() {
        let verdict = admitted_need(measured(SUPPORTED_CONTENT), FEW_NODES);
        assert!(
            !verdict,
            "whole-content support already decides the category"
        );
    }

    // Every branch that can still consume the core must retain it.
    #[test]
    fn rescue_shape_only_and_weak_content_still_need_a_core() {
        let mut rescue = measured(SUPPORTED_CONTENT);
        rescue.rescue_scope = true;
        rescue.score.structural = SHARED_SUBTREE_MIN_OVERLAP;
        rescue.score.token_jaccard = SHARED_SUBTREE_MIN_JACCARD;
        assert!(admitted_need(rescue, SHARED_SUBTREE_MIN_NODE_COUNT));
        let mut shape_only = measured(NEGLIGIBLE_CONTENT);
        shape_only.score.structural = RoutingTuning::default().nearly_identical_min_shape;
        assert!(required(
            shape_only,
            FEW_NODES,
            false,
            Some(PairClassification::StructuralOnly),
        ));
        assert!(admitted_need(measured(WEAK_CONTENT), FEW_NODES));
    }

    // [FUSED-SHARED-SUBTREE-CORE-NEED] Identical, renamed, near and
    // embedding-supported pairs already have categories before core work.
    #[test]
    fn earlier_categories_need_no_core_when_rescue_is_impossible() {
        let routing = RoutingTuning::default();
        let mut identical = measured(WEAK_CONTENT);
        identical.text = SourceIdentity {
            raw: PairTextIdentity::ByteIdentical,
            identical: true,
        };
        assert_eq!(
            classify_admitted(identical, routing),
            Some(PairClassification::Identical)
        );
        assert!(!admitted_need(identical, FEW_NODES));
        let mut renamed = measured(WEAK_CONTENT);
        renamed.content.consistent_rename = true;
        assert_eq!(
            classify_admitted(renamed, routing),
            Some(PairClassification::NearlyIdentical)
        );
        assert!(!admitted_need(renamed, FEW_NODES));
        let mut near = measured(SUPPORTED_CONTENT);
        near.score.structural = routing.nearly_identical_min_shape;
        assert_eq!(
            classify_admitted(near, routing),
            Some(PairClassification::NearlyIdentical)
        );
        assert!(!admitted_need(near, FEW_NODES));
        let mut embedded = measured(WEAK_CONTENT);
        embedded.score.embedding_cos = EMBEDDING_SUPPORT_FLOOR;
        assert_eq!(
            classify_admitted(embedded, routing),
            Some(PairClassification::SameBehavior)
        );
        assert!(!admitted_need(embedded, FEW_NODES));
    }

    // [FUSED-SHARED-SUBTREE-CORE-NEED] A rejected weak pair cannot gain
    // admission without rescue and is too dissimilar for structural-only.
    #[test]
    fn already_rejected_pair_needs_no_core_without_a_rescue_route() {
        let verdict = required(measured(WEAK_CONTENT), FEW_NODES, false, None);
        assert!(
            !verdict,
            "non-rescued rejection has no core-dependent outcome"
        );
    }
}
