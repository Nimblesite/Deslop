//! The [FUSION-CONTENT-GATE] routing tail and the measured-tier
//! vocabulary shared by the renderer and cross-cluster subsumption.
//!
//! [`route_shape_identical`] is the one copy of the shape-identical
//! demotion/promotion decision; `report_render::report_bucket_kind`
//! and [PIPELINE-CLUSTER-SUBSUME]'s survivor election both call it,
//! so the elected view and the rendered label cannot drift apart.

use crate::{
    content::ContentEvidence, fingerprint::Fingerprint, report::ReportSignals, state::FileId,
};

use super::{
    classify_signals, has_saturating_shape_evidence, is_token_carried_nearmiss, ClusterKind,
    CONTENT_PROMOTE_FLOOR, CONTENT_SUPPORT_FLOOR, LITERAL_TABLE_MIN_FRACTION,
};

/// [FUSION-CONTENT-GATE] routing tail: for shape-identical clusters the
/// measured content evidence decides in both directions. The
/// deterministic signals cannot: `structural` and `token_jaccard` are
/// two views of one normalised representation, so once the shape
/// saturates, the token axis echoes it at 1.0 for every same-shape
/// family — the honest #339 sibling-window signatures made that echo
/// universal, and the #197 REST settings family (content 0.72–0.80)
/// would ride it straight into the act-now tier. So content decides:
/// support at or above [`CONTENT_PROMOTE_FLOOR`] proves the clone
/// (pooled raw bytes that mostly agree, or a literal-anchored
/// consistent rename — [`ContentEvidence::support`]); anything below,
/// with no semantic backing, routes to the demoted tier —
/// [`ClusterKind::LooselySimilar`] for the cross-file scaffolding
/// spread, [`ClusterKind::StructuralOnly`] otherwise.
///
/// The anchor-free near-miss ([CLONE-BUCKETS-ROUTING] row 4) is decided
/// *before* that, by [`route_anchor_free`], and on different evidence —
/// it has no shape match for this gate's populations to measure against.
pub(crate) fn route_shape_identical(
    kind: ClusterKind,
    signals: ReportSignals,
    content: ContentEvidence,
    members: &[Fingerprint],
) -> ClusterKind {
    if !matches!(
        kind,
        ClusterKind::NearlyIdentical | ClusterKind::StructuralOnly
    ) {
        return kind;
    }
    if let Some(demoted) = route_anchor_free(signals, content, members) {
        return demoted;
    }
    if !has_saturating_shape_evidence(signals) {
        return kind;
    }
    // Semantic backing is content-independent evidence; the embedding
    // pass measured behaviour, not shape, so the gate keeps its verdict.
    // Backing means the cosine *vouches* for the cluster
    // ([`EMBEDDING_SUPPORT_FLOOR`]) — never merely that one was
    // measured. This bar used to be `STRUCTURAL_ONLY_MAX_SUPPORT`, which
    // is the ceiling *below* which a signal counts as absent; read as a
    // floor it let a cosine of 0.05 — a model reporting no relationship
    // — overrule the measured content evidence, so the gate's verdict
    // turned on whether the embedding pass ran rather than on the code.
    // `csharp-type3` published the identical two occurrences as
    // `structural_only` at cosine 0.00 and `nearly_identical` at 0.61
    // (gh #356, `deslop::embedding_route_invariance`).
    if signals.embedding_cos >= crate::pair::EMBEDDING_SUPPORT_FLOOR {
        return kind;
    }
    // Promotion rescues real clones from the demoted tier: pooled raw
    // bytes that agree, or a maximal Type-2 rename whose literals and
    // identifier mapping prove the copy. The bar depends on spread —
    // a cross-file cluster promotes at the Type-3 overlap cutoff
    // ([`CONTENT_SUPPORT_FLOOR`]), while a single-file cluster must
    // share nearly every position ([`CONTENT_PROMOTE_FLOOR`]): the
    // #197 in-class sibling families live in one file and measure
    // 0.72–0.80 and are API surface, not extract-worthy duplication.
    let promote_floor = if spans_multiple_files(members) {
        CONTENT_SUPPORT_FLOOR
    } else {
        CONTENT_PROMOTE_FLOOR
    };
    if content.support() >= promote_floor {
        return ClusterKind::NearlyIdentical;
    }
    // Literal-dominated families ([CLONE-NOISE-LITERAL-TABLE])
    // stay in the surfaced `structural_only` tier instead of the hidden
    // scaffolding one: the data-category policy ([RANK-CATEGORY]) owns
    // their visibility, and a policy knob cannot govern a cluster the
    // renderer already made disappear.
    if is_cross_file_scaffolding(members) && content.literal_fraction < LITERAL_TABLE_MIN_FRACTION {
        return ClusterKind::LooselySimilar;
    }
    ClusterKind::StructuralOnly
}

/// Returns true when a cluster's occurrences reach at least two files —
/// the spread that separates a duplicated copy from an in-file sibling
/// family, both in [`route_shape_identical`]'s promotion bar and in
/// [PIPELINE-CLUSTER-SUBSUME]'s verbatim overturn.
pub(crate) fn spans_multiple_files(members: &[Fingerprint]) -> bool {
    members
        .first()
        .is_some_and(|first| members.iter().any(|member| member.file_id != first.file_id))
}

/// Demotion for [CLONE-BUCKETS-ROUTING] **row 4** — the anchor-free
/// near-miss — or `None` to leave the routing alone. A shared-subtree
/// overlap below [`crate::pair::SHARED_SUBTREE_MIN_OVERLAP`] means no
/// meaningful shape matched, so a normalised-token estimate is the
/// cluster's only evidence, and two shapes of cluster carry that estimate
/// without earning an act-now verdict:
///
/// - **A cross-file spread** (3+ members over 3+ files) is the #134
///   scaffolding pattern arriving through the token door instead of the
///   structural one. Six distinct Flutter widgets read `structural=0.00,
///   token_jaccard=0.93` over whole-file spans whose `build` bodies share
///   nothing, because the framework-mandated declaration is most of each
///   file (#331). **A genuine clone family of that width is demoted to a
///   hint too** — the same trade [`is_cross_file_scaffolding`] already
///   makes for shape-identical spreads, for the same reason, and it is a
///   trade rather than a free win. Two narrower discriminators were
///   measured and rejected: the content gate (see
///   [`has_saturating_shape_evidence`]) and
///   [`ContentEvidence::substance_varies`], both of which demote
///   `csharp-type3` — a genuine renamed Type-3 pair — because neither can
///   evaluate a rename across misaligned shapes. Narrowing this rule
///   needs a discriminator that survives that fixture.
/// - **An unmeasured cluster**, where the content pass could not compare
///   two members at all. The anchored routes may take one on trust
///   because their Merkle equality is itself proof; row 4 has no such
///   signal, so unmeasured there means *nothing is known*. The #108
///   JSON-schema pair (`structural=0.00, token_jaccard=0.96`) would
///   otherwise be routed act-now on no evidence whatsoever.
///
/// The destination is [`ClusterKind::LooselySimilar`] — a hint the
/// renderer hides — and never [`ClusterKind::StructuralOnly`], which
/// would claim a shape match `structural = 0.00` says does not exist.
///
/// A *pair* that the content pass did measure is left alone even when its
/// agreement is low: that is the renamed Type-3 clone
/// ([`has_saturating_shape_evidence`] documents the 0.19 measurement),
/// which this gate's populations are structurally unable to vouch for.
fn route_anchor_free(
    signals: ReportSignals,
    content: ContentEvidence,
    members: &[Fingerprint],
) -> Option<ClusterKind> {
    let unearned = !content.measured || is_cross_file_scaffolding(members);
    (is_anchor_free_token_cluster(signals) && unearned).then_some(ClusterKind::LooselySimilar)
}

/// The row-4 population this guard governs: token-carried clusters
/// whose measured shared-subtree overlap stays below the
/// [FUSION-SHARED-SUBTREE] corroboration floor. A cluster at or above
/// that floor carries Merkle-subtree proof of its own — the same kind
/// of evidence the anchored routes take on trust — so the "nothing but
/// a token estimate" rationale no longer describes it, exactly as it
/// never described a Merkle-anchored cluster.
fn is_anchor_free_token_cluster(signals: ReportSignals) -> bool {
    is_token_carried_nearmiss(signals)
        && signals.structural < crate::pair::SHARED_SUBTREE_MIN_OVERLAP
}

/// Returns true when a structural-only cluster spans enough distinct
/// files to mirror the cross-test-file scaffolding pattern from issue
/// #134. Caller has already established the saturating shape-evidence
/// signal via [`has_saturating_shape_evidence`]. The 3-member, 3-file
/// floors preserve small two-occurrence pairs; smaller spreads route
/// to [`ClusterKind::StructuralOnly`] instead.
fn is_cross_file_scaffolding(members: &[Fingerprint]) -> bool {
    if members.len() < 3 {
        return false;
    }
    let mut files: Vec<FileId> = members.iter().map(|member| member.file_id).collect();
    files.sort_unstable();
    files.dedup();
    files.len() >= 3
}

/// The bucket a cluster's **measured** evidence earns on its own —
/// [`classify_signals`] plus the [FUSION-CONTENT-GATE] routing tail,
/// with no byte-equivalence proof available. Cross-cluster subsumption
/// ([PIPELINE-CLUSTER-SUBSUME], #367/#408) judges the surviving view
/// with this before the report renders, so the choice sees the same
/// demotions the reader will see; `report_render::report_bucket_kind`
/// is the rendered counterpart that additionally proves
/// [CLONE-BUCKETS-IDENTICAL] byte equivalence.
///
/// Without that proof an exact shape-and-token match is a *candidate*
/// Type-1/2, not a proven one — the routing tail decides it on measured
/// content, exactly as the renderer downgrades an unproven
/// [`ClusterKind::Identical`] before routing it.
#[must_use]
pub(crate) fn measured_kind(
    signals: ReportSignals,
    content: ContentEvidence,
    members: &[Fingerprint],
) -> ClusterKind {
    let kind = match classify_signals(signals) {
        ClusterKind::Identical => ClusterKind::NearlyIdentical,
        other => other,
    };
    route_shape_identical(kind, signals, content, members)
}

/// True for the demoted tier — the buckets the `[ranking]` policy
/// demotes ([RANK-STRUCTURAL-ONLY]) or the renderer hides outright.
/// [PIPELINE-CLUSTER-SUBSUME] uses the tier as its content-credibility
/// test: a demoted view never deletes a credible view of the same
/// region, because the deletion would replace a reported duplicate with
/// one the reader is told not to act on — or never shown at all.
#[must_use]
pub(crate) const fn is_demoted_tier(kind: ClusterKind) -> bool {
    matches!(
        kind,
        ClusterKind::StructuralOnly | ClusterKind::LooselySimilar
    )
}
