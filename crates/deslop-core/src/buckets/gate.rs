//! The [FUSED-CONTENT-GATE] correction: measured content evidence
//! decides what a saturated shape match is worth.
//!
//! `structural` and `token_jaccard` are two views of one normalised
//! representation, so once the shape saturates they echo each other
//! and say nothing about what the code *said*. The floors and the
//! support quantity live here; the routing tail that applies them per
//! bucket lives in [`super::routing`].
//!
//! There is no cluster-level `fused` and no content-confidence
//! multiply ([FUSED-SCOPE], [FUSED-CONTENT-GATE]). `fused` is the pair
//! admission score, decided pair by pair at admission
//! ([FUSED-STRATEGY-BOUNDED-MAX]); the report carries the elected
//! pair's measured axes and its content evidence, never a rendered
//! cluster confidence. Routing reads `support = max(agreement,
//! rename_consistency)` directly — no discount, no shape scaling.

use crate::{content::ContentEvidence, report::ReportSignals};

use super::{is_shape_corroborated_nearmiss, is_token_carried_nearmiss, ClusterKind};

/// Content agreement at which a *cross-file* shape-identical cluster
/// holds an act-now `nearly_identical` verdict ([FUSED-CONTENT-GATE]).
/// Shape saturation makes the token axis an echo of the structural one
/// — the honest #339 sibling-window signatures made that echo universal
/// — so measured content is the only discriminating evidence left. The
/// 0.7 operating point matches the [TECH-TOKEN-SOURCERERCC] Type-3
/// overlap cutoff: a genuine renamed copy keeps most collapsed-leaf
/// positions byte-equal and clears it comfortably.
pub const CONTENT_SUPPORT_FLOOR: f64 = 0.7;

/// Content agreement required for a *single-file* shape-identical
/// cluster to hold the act-now verdict ([FUSED-CONTENT-GATE]). In-class
/// sibling-method families such as the #197 REST settings surface
/// measure 0.72–0.80 (shared plumbing, differing endpoint literals) and
/// are API surface, not extract-worthy duplication — they must keep
/// their demoted verdict — while a genuine same-file near-miss window
/// shares nearly every position (≥ 0.85, the same act-now grade as
/// [FUSED-THRESHOLD]).
pub const CONTENT_PROMOTE_FLOOR: f64 = 0.85;

/// Literal fraction at which a shape-identical cluster counts as a data
/// literal ([CLONE-NOISE-LITERAL-TABLE]): the canonical member's
/// collapsed leaves are overwhelmingly literal positions — a numeric
/// array, a lookup table, generated test data — in any language. Such
/// clusters are governed by the `[ranking] data_clones` policy
/// ([RANK-CATEGORY]) instead of the scaffolding hide, so they stay
/// labelled and policy-controllable rather than silently vanishing.
pub const LITERAL_TABLE_MIN_FRACTION: f64 = 0.8;

/// True when a cluster's deterministic signals are shape echoes that
/// saturate by construction ([FUSED-CONTENT-GATE]): an exact Merkle
/// match, or a near-total kind-stream Jaccard — the token LSH pass
/// hashes the same normalised representation the structural pass does,
/// so a `token_jaccard` at the [`super::classify_signals`] near-identical line
/// is shape evidence too, not content evidence ('s surviving
/// mixed cluster read `structural=0.62, token_jaccard=0.98`).
///
/// The near-miss row-4 routes ([`is_token_carried_nearmiss`],
/// [`is_shape_corroborated_nearmiss`]) are
/// deliberately **not** included. Both of this gate's populations —
/// positional byte agreement and literal-anchored rename consistency —
/// assume the members align position for position, which is exactly what
/// an anchor-free cluster does not do: `structural ≤ 0.01` means the
/// shapes differ. Measured against a genuine Type-3 clone whose
/// identifiers are all renamed and whose bodies differ by one statement
/// (`csharp-type3`), agreement collapses to 0.19 — the literals — and
/// rename consistency to 0.0, because the extra statement destroys the
/// alignment the rename proof needs. Gating row 4 here therefore demotes
/// the renamed near-miss, the most valuable clone class there is. Row 4
/// is instead routed on cluster *spread* in
/// `report_render::route_shape_identical`.
#[must_use]
pub fn has_saturating_shape_evidence(signals: ReportSignals) -> bool {
    signals.structural >= STRUCTURAL_SATURATION_FLOOR
        || signals.token_jaccard >= SATURATING_TOKEN_FLOOR
}

/// Structural grade at or above which a view's occurrences are the same
/// normalised tree: [FUSED-SHARED-SUBTREE] grades `1 - TED/max(nodes)`,
/// so saturation means the members align node for node and the view is a
/// faithful description of one repeated shape. Named because three
/// routing sites turn on this single boundary — a view *below* it is
/// measuring occurrences that disagree in shape.
pub const STRUCTURAL_SATURATION_FLOOR: f64 = 0.99;

/// Content support carried by the two independent measured
/// populations: either may vouch for a shape-identical cluster — pooled
/// byte agreement or a corroborated consistent rename —
/// and [FUSED-CONTENT-GATE] routes on the stronger, never on their
/// mean. Defined once here because the mean is what demoted maximal
/// Type-2 renames, so the two callers that read this quantity —
/// [`ContentEvidence::support`] on the measured evidence and
/// [`lacks_content_support`] on the rendered signals — must not be free
/// to drift apart.
#[must_use]
pub fn content_support(agreement: f64, rename_consistency: f64) -> f64 {
    agreement.max(rename_consistency)
}

/// [CLONE-BUCKETS-ROUTING] route 2 into the demoted tier, read back off
/// a *rendered* signal triple: the deterministic shape evidence
/// saturates by construction ([`has_saturating_shape_evidence`]) while
/// the measured content evidence stays below [`CONTENT_SUPPORT_FLOOR`],
/// so nothing about what the code actually *said* vouches for the
/// match. A scaffolding family and a corroborated Type-2 rename render
/// the identical `structural = 1.00, token_jaccard = 1.00` triple; this
/// is the predicate that separates them.
///
/// Consumers are decision surfaces that must not act on shape alone —
/// today the refactor preconditions
/// ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 1), which would otherwise fold
/// two unrelated methods into one shared helper.
///
/// The row-4 near-miss routes are excluded for the reason
/// [`has_saturating_shape_evidence`] documents at length: their members
/// do not align position for position, so *both* content populations
/// are structurally unable to vouch for a genuine renamed Type-3 clone
/// (`csharp-type3` measures agreement 0.19, rename consistency 0.00).
/// Convicting one here would manufacture the exact false negative
/// `report_render::route_anchor_free` exists to avoid. The exclusion is
/// route membership *without a Merkle-saturated shape*: once
/// `structural >= 0.99` the members align by construction and the
/// conviction stands exactly as before ([FUSED-SHARED-SUBTREE]).
#[must_use]
pub fn lacks_content_support(signals: ReportSignals) -> bool {
    let misaligned_nearmiss = signals.structural < STRUCTURAL_SATURATION_FLOOR
        && (is_token_carried_nearmiss(signals) || is_shape_corroborated_nearmiss(signals));
    has_saturating_shape_evidence(signals)
        && !misaligned_nearmiss
        && content_support(signals.pair_agreement, signals.pair_rename_consistency) < CONTENT_SUPPORT_FLOOR
}

/// Token overlap at or above which the token layer is echoing shape
/// rather than reporting content ([FUSED-CONTENT-GATE]). Named because
/// the assertion surface has to distinguish the two routes into
/// `structural_only` — evidence-free below
/// [`super::STRUCTURAL_ONLY_MAX_SUPPORT`], content-gated at or above this — and
/// a test carrying its own copy of the number drifts from the router.
pub const SATURATING_TOKEN_FLOOR: f64 = 0.95;

/// The final render transform for a shape-identical cluster's signal
/// triple: stamps the elected pair's measured content evidence and
/// applies the token-axis correction where the members share one
/// digest. There is no cluster `fused` to compute — admission decided
/// the pair, the report names it, and routing reads `support =
/// max(agreement, rename_consistency)` from the measured evidence.
///
/// `members_share_one_digest` is the renderer's answer to "do all of
/// this cluster's members carry the same normalised-subtree hash" — the
/// only fact that licenses the token-axis correction, and one the
/// signal triple cannot supply on its own (gh #431).
#[must_use]
pub fn content_gated_signals(
    signals: ReportSignals,
    content: ContentEvidence,
    kind: ClusterKind,
    members_share_one_digest: bool,
) -> ReportSignals {
    // The measured content evidence is stamped on **every** path,
    // including the two that leave the triple untouched. A reader that
    // can see the pair's axes but not the evidence behind them cannot
    // tell a corroborated rename from an anchor-poor scaffolding family
    // — the two render the same triple ([FUSED-CONTENT-GATE]). Returning
    // the input unchanged here would leave those fields at the zeroes
    // `From<PairScore>` seeded, which reads as "measured, and found
    // nothing" rather than "measured, and found this".
    let signals = with_content_evidence(signals, content);
    if kind == ClusterKind::Identical || !has_saturating_shape_evidence(signals) {
        return stamp_shape(signals);
    }
    stamp_shape(correct_token_echo(signals, kind, members_share_one_digest))
}

/// Corrects the rendered `token_jaccard` for a `NearlyIdentical` cluster
/// whose members all carry **one** normalised-subtree digest: their kind
/// streams are equal by construction, so the true token Jaccard is 1.0 —
/// the same argument the byte-equivalence upgrade applies to `Identical`.
/// A lower rendered value there is a fingerprint-scoped fallback-signature
/// artifact, not evidence, so it is corrected here.
///
/// The guard is the digest equality the argument names, passed in by
/// the renderer, and not a `structural` reading (gh #431). Since #408
/// that axis grades shared-subtree *overlap* ([FUSED-SHARED-SUBTREE]):
/// every value below saturation means the subtrees provably differ, so
/// no digest is shared and the argument covers none of them, while
/// saturation itself is reachable by ratio — `shared == larger` —
/// without digest equality. Scoping the correction to
/// `STRUCTURAL_SATURATION_FLOOR` therefore published `token_jaccard =
/// 1.0`, and a `shape` derived from it, for the whole `[0.99, 1.0)` band
/// on no evidence at all: a near-miss *routing* tolerance is not proof
/// of identity. A mixed LSH-glued cluster keeps its estimated value.
/// `StructuralOnly` keeps its unscored signal: absent token support is
/// that bucket's defining signature ([RANK-STRUCTURAL-ONLY]).
fn correct_token_echo(
    signals: ReportSignals,
    kind: ClusterKind,
    members_share_one_digest: bool,
) -> ReportSignals {
    let token_jaccard = if kind == ClusterKind::NearlyIdentical && members_share_one_digest {
        1.0
    } else {
        signals.token_jaccard
    };
    ReportSignals {
        token_jaccard,
        ..signals
    }
}

/// Stamps the rendered shape reading onto the signals every surface
/// receives ([FUSED-CONTENT-GATE]). This gate is the last transform a
/// rendered cluster's signals pass through, so stamping here — on both
/// exit paths — is what lets every consumer read `shape` verbatim
/// instead of re-deriving `max(structural, token_jaccard)` locally.
fn stamp_shape(mut signals: ReportSignals) -> ReportSignals {
    signals.shape = signals.shape_score();
    signals
}

/// Stamps the measured content evidence onto a rendered signal triple
/// without touching the confidence ([FUSED-CONTENT-GATE], #344).
fn with_content_evidence(signals: ReportSignals, content: ContentEvidence) -> ReportSignals {
    ReportSignals {
        pair_agreement: content.agreement,
        pair_rename_consistency: content.rename_consistency,
        literal_fraction: content.literal_fraction,
        ..signals
    }
}
