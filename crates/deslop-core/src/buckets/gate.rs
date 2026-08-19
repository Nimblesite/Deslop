//! The [FUSION-CONTENT-GATE] correction: measured content evidence
//! decides what a saturated shape match is worth.
//!
//! `structural` and `token_jaccard` are two views of one normalised
//! representation, so once the shape saturates they echo each other
//! and say nothing about what the code *said*. The floors, the
//! support quantity, and the fused-confidence correction live here;
//! the routing tail that applies them per bucket lives in
//! [`super::routing`].

use crate::{content::ContentEvidence, report::ReportSignals};

use super::{is_lsh_only_nearmiss, ClusterKind};

/// Content agreement at which a *cross-file* shape-identical cluster
/// holds an act-now `nearly_identical` verdict ([FUSION-CONTENT-GATE]).
/// Shape saturation makes the token axis an echo of the structural one
/// — the honest #339 sibling-window signatures made that echo universal
/// — so measured content is the only discriminating evidence left. The
/// 0.7 operating point matches the [TECH-TOKEN-SOURCERERCC] Type-3
/// overlap cutoff: a genuine renamed copy keeps most collapsed-leaf
/// positions byte-equal and clears it comfortably.
pub const CONTENT_SUPPORT_FLOOR: f64 = 0.7;

/// Content agreement required for a *single-file* shape-identical
/// cluster to hold the act-now verdict ([FUSION-CONTENT-GATE]). In-class
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
/// saturate by construction ([FUSION-CONTENT-GATE]): an exact Merkle
/// match, or a near-total kind-stream Jaccard — the token LSH pass
/// hashes the same normalised representation the structural pass does,
/// so a `token_jaccard` at the [`super::classify_signals`] near-identical line
/// is shape evidence too, not content evidence ('s surviving
/// mixed cluster read `structural=0.62, token_jaccard=0.98`).
///
/// The anchor-free row-4 route ([`is_lsh_only_nearmiss`]) is
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
    signals.structural >= 0.99 || signals.token_jaccard >= SATURATING_TOKEN_FLOOR
}

/// Content support carried by the two independent measured
/// populations: either may vouch for a shape-identical cluster — pooled
/// byte agreement or a corroborated consistent rename —
/// and [FUSION-CONTENT-GATE] routes on the stronger, never on their
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
/// The anchor-free row-4 near-miss is excluded for the reason
/// [`has_saturating_shape_evidence`] documents at length: its members do
/// not align position for position, so *both* content populations are
/// structurally unable to vouch for a genuine renamed Type-3 clone
/// (`csharp-type3` measures agreement 0.19, rename consistency 0.00).
/// Convicting it here would manufacture the exact false negative
/// `report_render::route_anchor_free` exists to avoid.
#[must_use]
pub fn lacks_content_support(signals: ReportSignals) -> bool {
    has_saturating_shape_evidence(signals)
        && !is_lsh_only_nearmiss(signals)
        && content_support(signals.agreement, signals.rename_consistency) < CONTENT_SUPPORT_FLOOR
}

/// Token overlap at or above which the token layer is echoing shape
/// rather than reporting content ([FUSION-CONTENT-GATE]). Named because
/// the assertion surface has to distinguish the two routes into
/// `structural_only` — evidence-free below
/// [`super::STRUCTURAL_ONLY_MAX_SUPPORT`], content-gated at or above this — and
/// a test carrying its own copy of the number drifts from the router.
pub const SATURATING_TOKEN_FLOOR: f64 = 0.95;

/// Confidence discount applied to rename-consistency evidence when the
/// gate fuses it ([FUSION-CONTENT-GATE]). A literal-anchored bijective
/// rename is proven duplication, but its identifier positions matched
/// through a mapping rather than byte equality — strictly weaker
/// evidence than a verbatim copy. The discount keeps a proven Type-2
/// rename above the [FUSED-THRESHOLD] act-now line while reserving
/// saturation (`fused == 1.0`) for byte-proven duplication, so the
/// rendered score still orders copy-paste above rename.
pub const RENAME_CONSISTENCY_DISCOUNT: f64 = 0.9;

/// Corrects the rendered fused confidence for shape-identical clusters
/// ([FUSION-CONTENT-GATE]). `structural` and `token_jaccard`
/// are two views of one normalised representation, so summing them says
/// nothing beyond "the shapes matched" — every shape match used to
/// render `fused = 1.0`, which made the agent-facing act-now threshold
/// unreachable from below. The honest confidence for a shape match is
/// its structural certainty scaled by measured content evidence — pooled
/// byte agreement or discounted rename consistency, whichever is the
/// stronger proof — or the semantic signal when that beats both.
/// Byte-equivalence-proven [`ClusterKind::Identical`] clusters keep
/// their saturated confidence, and clusters discovered without an exact
/// shape match (LSH / embedding paths) keep the existing fusion.
#[must_use]
pub fn content_gated_signals(
    signals: ReportSignals,
    content: ContentEvidence,
    kind: ClusterKind,
) -> ReportSignals {
    // #344: the measured content evidence is stamped on **every** path,
    // including the two that leave the confidence untouched. A reader
    // that can see `fused` but not the evidence behind it cannot tell a
    // corroborated rename from an anchor-poor scaffolding family — the
    // two render the same triple ([FUSION-CONTENT-GATE]). Returning the
    // input unchanged here would leave those fields at the zeroes
    // `From<PairScore>` seeded, which reads as "measured, and found
    // nothing" rather than "measured, and found this".
    let signals = with_content_evidence(signals, content);
    if kind == ClusterKind::Identical || !has_saturating_shape_evidence(signals) {
        return signals;
    }
    let content_confidence = content
        .agreement
        .max(RENAME_CONSISTENCY_DISCOUNT * content.rename_consistency);
    let fused = signals
        .embedding_cos
        .max(signals.structural.max(signals.token_jaccard) * content_confidence)
        .clamp(0.0, 1.0);
    // A shape-identical cluster routed `NearlyIdentical` shares one
    // Merkle hash, so the members' normalised kind streams are equal by
    // construction and the true token Jaccard is 1.0 — the same
    // argument the byte-equivalence upgrade applies to `Identical`. A
    // lower rendered value is a fingerprint-scoped fallback-signature
    // artifact, not evidence, so it is corrected here. The
    // `structural` guard scopes the correction to clusters the Merkle
    // argument actually covers — a mixed LSH-glued cluster keeps its
    // estimated value. `StructuralOnly` keeps its unscored signal:
    // absent token support is that bucket's defining signature
    // ([RANK-STRUCTURAL-ONLY]).
    let token_jaccard = if kind == ClusterKind::NearlyIdentical && signals.structural >= 0.99 {
        1.0
    } else {
        signals.token_jaccard
    };
    ReportSignals {
        token_jaccard,
        fused,
        ..signals
    }
}

/// Stamps the measured content evidence onto a rendered signal triple
/// without touching the confidence ([FUSION-CONTENT-GATE], #344).
fn with_content_evidence(signals: ReportSignals, content: ContentEvidence) -> ReportSignals {
    ReportSignals {
        agreement: content.agreement,
        rename_consistency: content.rename_consistency,
        literal_fraction: content.literal_fraction,
        ..signals
    }
}
