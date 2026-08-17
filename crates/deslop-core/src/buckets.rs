//! Canonical clone buckets — single source of truth for every renderer.
//!
//! Implements `[CLONE-BUCKETS]`, `[CLONE-BUCKETS-DUAL-LABEL]`, and
//! `[CLONE-BUCKETS-ROUTING]` from
//! [`docs/specs/taxonomy.md`](../../../../docs/specs/taxonomy.md).
//!
//! **Two audiences, three surface classes, one bucket identity.**
//! - **Pure-visual** (HTML card, VS Code bubble / webviews / tree view)
//!   → humans only. Use [`BucketLabels::plain_title`] + [`action_sentence`].
//!   No `Type-N`, no enum names, no signal triples in prose.
//! - **Shared-text** (CLI stderr, LSP `diagnostic.message`, VS Code
//!   Problems panel, hover tooltip) → humans first, agents scrape.
//!   Use [`BucketLabels::hybrid_title`] — plain prose with bracketed
//!   `Type-N` suffix (e.g. `"Identical code [Type-1/2]"`). Per user
//!   mandate: *"Shoot for human readable, but include technical terms
//!   in brackets for the ai"*.
//! - **AI-only** (JSON `interpretation`, `action_hints`, `schema_doc`,
//!   MCP responses) → agents only. Use plain title + action sentence +
//!   [`BucketLabels::taxonomy_label`] assembled into one precise
//!   sentence. Dropping `Type-N` would break prompts in the wild.
//!
//! Every renderer calls [`bucket_labels`] rather than hard-coding
//! strings so the four parallel vocabularies we used to ship can never
//! regrow. The helper carries all three forms; the renderer picks.

use crate::{
    content::ContentEvidence,
    report::{ReportCluster, ReportSignals},
};

/// Canonical bucket identity. The enum is the one source of truth;
/// every human / agent label attaches to one of these variants via
/// [`bucket_labels`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClusterKind {
    /// Type-1 / Type-2 exact clones. Identical after normalisation
    /// (whitespace, comments, renamed identifiers).
    Identical,
    /// Type-3 near-miss: same shape with small structural or token
    /// differences that may be semantically meaningful.
    NearlyIdentical,
    /// Structural-only match ([RANK-STRUCTURAL-ONLY]):
    /// the normalized AST shape is the only positive evidence — no
    /// token overlap, no semantic support. Usually a sibling
    /// boilerplate family (REST CRUD, settings getters, builders);
    /// occasionally a genuine Type-2 rename candidate. Surfaced, but
    /// demoted in ranking by default.
    StructuralOnly,
    /// Weak LSH-only overlap that survived the sub-threshold filters.
    /// Hint, not a directive.
    LooselySimilar,
    /// Type-4 semantic match: the embedding pass noticed two
    /// syntactically distinct implementations share behaviour. Only
    /// reachable when embeddings ran.
    SameBehavior,
}

impl ClusterKind {
    /// Every variant in canonical order. Used by renderers that need
    /// to iterate over all buckets (e.g. the CLI breakdown line).
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Identical,
            Self::NearlyIdentical,
            Self::StructuralOnly,
            Self::LooselySimilar,
            Self::SameBehavior,
        ]
    }

    /// Stable wire label used in the JSON report's `cluster.bucket`
    /// field. `snake_case` so agents can pattern-match without
    /// deserialising the whole enum.
    #[must_use]
    pub const fn wire_label(self) -> &'static str {
        match self {
            Self::Identical => "identical",
            Self::NearlyIdentical => "nearly_identical",
            Self::StructuralOnly => "structural_only",
            Self::LooselySimilar => "loosely_similar",
            Self::SameBehavior => "same_behavior",
        }
    }
}

/// Triple-labelled copy for one bucket. One struct, one helper, every
/// renderer reads from it. [`BucketLabels`] is what lets HTML, CLI,
/// LSP, VS Code, and the JSON `interpretation` agree without shared
/// string constants scattered across the crate.
///
/// Renderers pick a field by surface class per [CLONE-BUCKETS-DUAL-LABEL]:
/// - Pure-visual → [`Self::plain_title`] + [`Self::action_sentence`].
/// - Shared-text → [`Self::hybrid_title`] + [`Self::action_sentence`].
/// - AI-only → compose via [`Self::agent_summary`].
#[derive(Debug, Clone, Copy)]
pub struct BucketLabels {
    /// Plain-English heading for pure-visual surfaces (HTML card, VS
    /// Code webview, live bubble decoration). Never contains `Type-N`.
    /// Example: `"Identical code"`.
    pub plain_title: &'static str,
    /// Heading for shared-text surfaces (CLI stderr, LSP
    /// `diagnostic.message`, VS Code Problems panel, hover tooltip).
    /// Plain prose prefix + bracketed taxonomy for AI scrapers.
    /// Example: `"Identical code [Type-1/2]"`.
    pub hybrid_title: &'static str,
    /// Plain-English one-liner shown under the title on every surface.
    /// Same copy regardless of class.
    pub action_sentence: &'static str,
    /// Academic taxonomy reference appended to AI-only prose and
    /// bracketed into `hybrid_title`. Example: `"Type-1 or Type-2
    /// exact clone"` (note: the bracketed form inside `hybrid_title`
    /// uses a shorter `"Type-1/2"` for readability).
    pub taxonomy_label: &'static str,
    /// CSS class suffix used by the HTML renderer (e.g. `"identical"`
    /// → `.kind-identical`). Kept in sync with the Kinetic Manuscript
    /// palette in `render/html_css.rs`.
    pub css_suffix: &'static str,
    /// `true` when this bucket is populated exclusively by the
    /// embedding pass. Drives the `(AI match)` badge per
    /// `[CLONE-BUCKETS]` rule 5.
    pub ai_match: bool,
}

impl BucketLabels {
    /// AI-only sentence combining plain title, action sentence, and
    /// academic taxonomy. Used by JSON `cluster.interpretation` and
    /// `action_hints[*].recommendation`. Deterministic — safe to
    /// include in golden-test assertions.
    #[must_use]
    pub fn agent_summary(&self) -> String {
        format!(
            "{}. {} ({})",
            self.plain_title, self.action_sentence, self.taxonomy_label
        )
    }
}

/// Canonical bucket copy. Must match `docs/specs/taxonomy.md
/// [CLONE-BUCKETS]` byte-for-byte on the plain / hybrid / action
/// columns; if the table changes, this function changes in the same
/// commit.
#[must_use]
pub const fn bucket_labels(kind: ClusterKind) -> BucketLabels {
    match kind {
        ClusterKind::Identical => BucketLabels {
            plain_title: "Identical code",
            hybrid_title: "Identical code [Type-1/2]",
            action_sentence: "Safe to extract — every copy is the same.",
            taxonomy_label: "Type-1 or Type-2 exact clone",
            css_suffix: "identical",
            ai_match: false,
        },
        ClusterKind::NearlyIdentical => BucketLabels {
            plain_title: "Nearly identical code",
            hybrid_title: "Nearly identical code [Type-3]",
            action_sentence: "Review the locations — small differences may matter.",
            taxonomy_label: "Type-3 near-miss",
            css_suffix: "nearly-identical",
            ai_match: false,
        },
        ClusterKind::StructuralOnly => BucketLabels {
            plain_title: "Same shape, different content",
            hybrid_title: "Same shape, different content [structural-only]",
            action_sentence:
                "Only the code shape matches — usually sibling boilerplate. Verify before extracting.",
            taxonomy_label: "structural-only match (unverified Type-2/3 candidate)",
            css_suffix: "structural-only",
            ai_match: false,
        },
        ClusterKind::LooselySimilar => BucketLabels {
            plain_title: "Loosely similar code",
            hybrid_title: "Loosely similar code [weak LSH]",
            action_sentence: "Loose textual overlap. Treat as a hint.",
            taxonomy_label: "weak LSH-only signal (sub-Type-3)",
            css_suffix: "loosely-similar",
            ai_match: false,
        },
        ClusterKind::SameBehavior => BucketLabels {
            plain_title: "Same behavior, different code",
            hybrid_title: "Same behavior, different code [Type-4, AI match]",
            action_sentence:
                "The AI noticed these do the same thing written two ways — read both before merging.",
            taxonomy_label: "Type-4 semantic clone (AI match)",
            css_suffix: "same-behavior",
            ai_match: true,
        },
    }
}

/// Resolves a report cluster's canonical bucket. Fresh reports carry the
/// authoritative wire label; older reports fall back to signal routing.
#[must_use]
pub fn classify(cluster: &ReportCluster) -> ClusterKind {
    kind_from_wire_label(&cluster.bucket).unwrap_or_else(|| classify_signals(cluster.signals))
}

/// Parses the stable JSON `cluster.bucket` wire label.
fn kind_from_wire_label(label: &str) -> Option<ClusterKind> {
    match label {
        "identical" => Some(ClusterKind::Identical),
        "nearly_identical" => Some(ClusterKind::NearlyIdentical),
        "structural_only" => Some(ClusterKind::StructuralOnly),
        "loosely_similar" => Some(ClusterKind::LooselySimilar),
        "same_behavior" => Some(ClusterKind::SameBehavior),
        _ => None,
    }
}

/// Maximum token / embedding support a cluster may show while still
/// counting as evidence-free for [`is_structural_only_signals`]. The
/// 0.05 ceiling matches the #197 acceptance criterion
/// (`token_jaccard=0.00`, `embedding_cos=0.00`) while tolerating
/// `MinHash` collision noise.
pub const STRUCTURAL_ONLY_MAX_SUPPORT: f64 = 0.05;

/// Single source of truth for the structural-only evidence test
/// ([RANK-STRUCTURAL-ONLY]): the structural fingerprint is
/// the only positive support. Shared by the bucket routing and the
/// ranking demotion so a cluster labelled `structural_only` is always
/// the cluster the `[ranking]` policy demotes — the label and the
/// weight can no longer diverge.
#[must_use]
pub fn is_structural_only_signals(signals: ReportSignals) -> bool {
    signals.structural >= 0.99
        && signals.token_jaccard < STRUCTURAL_ONLY_MAX_SUPPORT
        && signals.embedding_cos < STRUCTURAL_ONLY_MAX_SUPPORT
}

/// Lowest content agreement a shape-identical cluster may show while
/// still counting as a Type-2/3 clone rather than shape-only
/// scaffolding ([FUSION-CONTENT-GATE]). Genuine renamed
/// copies change a handful of collapsed-leaf positions and stay well
/// above this floor; framework-mandated declarations and data tables
/// change most positions and fall well below it. The 0.7 operating
/// point matches the [TECH-TOKEN-SOURCERERCC] Type-3 overlap cutoff.
pub const CONTENT_SUPPORT_FLOOR: f64 = 0.7;

/// Content agreement required to *promote* a shape-identical cluster
/// into the act-now `nearly_identical` bucket when the token layer lost
/// its signature to the fingerprint-scoped fallback. Between
/// [`CONTENT_SUPPORT_FLOOR`] and this bar the legacy signal routing
/// stands: real-world sibling families such as the #197 REST
/// settings surface measure 0.72–0.80 (shared plumbing, differing
/// endpoint literals) and must keep their demoted verdict, while a
/// genuine near-miss window shares nearly every position (≥ 0.85 — the
/// same act-now grade as [FUSED-THRESHOLD]).
pub const CONTENT_PROMOTE_FLOOR: f64 = 0.85;

/// Literal fraction at which a shape-identical cluster counts as a data
/// literal ([CLONE-NOISE-LITERAL-TABLE]): the canonical member's
/// collapsed leaves are overwhelmingly literal positions — a numeric
/// array, a lookup table, generated test data — in any language. Such
/// clusters are governed by the `[ranking] data_clones` policy
/// ([RANK-CATEGORY]) instead of the scaffolding hide, so they stay
/// labelled and policy-controllable rather than silently vanishing.
pub const LITERAL_TABLE_MIN_FRACTION: f64 = 0.8;

/// True when a shape-identical cluster's only real evidence is its
/// shape: neither content population vouches for it — the pooled raw
/// bytes mostly disagree, no consistent literal-anchored rename explains
/// the differences ([`ContentEvidence::support`]), and no semantic
/// signal supports it ([FUSION-CONTENT-GATE]). Such clusters join the
/// [`is_structural_only_signals`] routing so the bucket label and the
/// ranking demotion stay in lockstep.
#[must_use]
pub fn lacks_content_support(signals: ReportSignals, content: ContentEvidence) -> bool {
    has_saturating_shape_evidence(signals)
        && content.support() < CONTENT_SUPPORT_FLOOR
        && signals.embedding_cos < STRUCTURAL_ONLY_MAX_SUPPORT
}

/// True when a cluster's deterministic signals are shape echoes that
/// saturate by construction ([FUSION-CONTENT-GATE]): an exact Merkle
/// match, or a near-total kind-stream Jaccard — the token LSH pass
/// hashes the same normalised representation the structural pass does,
/// so a `token_jaccard` at the [`classify_signals`] near-identical line
/// is shape evidence too, not content evidence ('s surviving
/// mixed cluster read `structural=0.62, token_jaccard=0.98`).
///
/// The anchor-free row-4 route ([`is_lsh_only_nearmiss`]) is the same
/// argument at its own floor: it reaches `NearlyIdentical` on token
/// overlap **alone**, and that overlap is measured over the normalised
/// stream, so it must face the content gate as well. Without this
/// disjunct a framework-mandated declaration family re-enters the
/// act-now bucket through the anchor-free door — six distinct Flutter
/// widgets measure `structural=0.00, token_jaccard=0.93` on whole-file
/// spans whose `build` bodies share nothing (#331), and every
/// [`SATURATING_TOKEN_FLOOR`] test above is blind to that band.
#[must_use]
pub fn has_saturating_shape_evidence(signals: ReportSignals) -> bool {
    signals.structural >= 0.99
        || signals.token_jaccard >= SATURATING_TOKEN_FLOOR
        || is_lsh_only_nearmiss(signals)
}

/// Token overlap at or above which the token layer is echoing shape
/// rather than reporting content ([FUSION-CONTENT-GATE]). Named because
/// the assertion surface has to distinguish the two routes into
/// `structural_only` — evidence-free below
/// [`STRUCTURAL_ONLY_MAX_SUPPORT`], content-gated at or above this — and
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

/// Signals-only fallback for reports that do not carry `cluster.bucket`.
#[must_use]
pub fn classify_signals(signals: ReportSignals) -> ClusterKind {
    if signals.structural >= 0.99 && signals.token_jaccard >= 0.99 {
        ClusterKind::Identical
    } else if signals.embedding_cos >= 0.80 && signals.structural < 0.50 {
        ClusterKind::SameBehavior
    } else if is_structural_only_signals(signals) {
        ClusterKind::StructuralOnly
    } else if is_lsh_only_nearmiss(signals)
        || signals.structural >= 0.99
        || (signals.structural >= 0.20 && signals.token_jaccard >= 0.95)
    {
        // [CLONE-BUCKETS-ROUTING] rows 4 and 5 share this destination:
        // the anchor-free LSH-only near-miss ([`is_lsh_only_nearmiss`])
        // and the structurally-anchored near-miss. Kept as one arm
        // because both routes produce the identical bucket — the named
        // predicate is what keeps row 4 legible and greppable.
        ClusterKind::NearlyIdentical
    } else {
        ClusterKind::LooselySimilar
    }
}

/// [CLONE-BUCKETS-ROUTING] row 4: a cluster with no structural anchor
/// whose token overlap clears [`LSH_ONLY_NEARMISS_MIN_JACCARD`] is a
/// genuine Type-3 near-miss, in **every** language.
///
/// A cluster only reaches the renderer with this triple by surviving
/// `pair::survival_decision`, which admits a structurally-unanchored
/// pair only above the same Jaccard floor and above the endpoint
/// node-count floor — the pipeline has already ruled out low-information
/// token noise, which is why this row is a signal test and needs no
/// language, size, or spread condition. Routing it anywhere else means
/// the pipeline admitted a pair as real duplication and the renderer
/// then discarded it: previously it fell to
/// [`ClusterKind::LooselySimilar`], which the renderer hides, so a fully
/// duplicated pair reported zero duplication in every language except
/// the one a report-render carve-out special-cased (gh #390). Pinned by
/// `crates/deslop/tests/lsh_only_nearmiss_recall.rs`.
#[must_use]
pub fn is_lsh_only_nearmiss(signals: ReportSignals) -> bool {
    signals.structural <= STRUCTURAL_ABSENT_CEILING
        && signals.token_jaccard >= LSH_ONLY_NEARMISS_MIN_JACCARD
}

/// Highest `structural` a cluster may show while counting as having no
/// structural anchor ([CLONE-BUCKETS-ROUTING] row 4). Mirrors the
/// spec's `structural ≤ 0.01`.
pub const STRUCTURAL_ABSENT_CEILING: f64 = 0.01;

/// Token overlap an anchor-free cluster must clear to count as a
/// Type-3 near-miss ([CLONE-BUCKETS-ROUTING] row 4). Deliberately equal
/// to `pair::LSH_ONLY_MIN_JACCARD`: the pair layer admits an LSH-only
/// candidate at exactly this floor, so a lower value here would hide
/// clusters the pipeline admitted, and a higher one would reject them
/// after admission. The two constants are one operating point stated in
/// the two layers that enforce it.
pub const LSH_ONLY_NEARMISS_MIN_JACCARD: f64 = 0.90;
