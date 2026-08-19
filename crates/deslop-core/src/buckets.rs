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

use crate::report::{ReportCluster, ReportSignals};

/// [FUSION-CONTENT-GATE] floors and the fused-confidence correction.
mod gate;
/// The shape-identical routing tail shared by renderer and subsumption.
mod routing;

pub use gate::{
    content_gated_signals, content_support, has_saturating_shape_evidence, lacks_content_support,
    CONTENT_PROMOTE_FLOOR, CONTENT_SUPPORT_FLOOR, LITERAL_TABLE_MIN_FRACTION,
    RENAME_CONSISTENCY_DISCOUNT, SATURATING_TOKEN_FLOOR,
};
pub(crate) use routing::{
    is_demoted_tier, measured_kind, route_shape_identical, spans_multiple_files,
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

/// Signals-only fallback for reports that do not carry `cluster.bucket`.
#[must_use]
pub fn classify_signals(signals: ReportSignals) -> ClusterKind {
    if signals.structural >= 0.99 && signals.token_jaccard >= 0.99 {
        ClusterKind::Identical
    } else if signals.embedding_cos >= crate::pair::EMBEDDING_SUPPORT_FLOOR
        && signals.structural < 0.50
    {
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
/// Type-3 near-miss ([CLONE-BUCKETS-ROUTING] row 4). **Is**
/// [`crate::pair::LSH_ONLY_MIN_JACCARD`], not a copy of its value: the
/// pair layer admits an LSH-only candidate at exactly this floor, so a
/// lower value here would hide clusters the pipeline admitted and a
/// higher one would reject them after admission. Naming it separately
/// keeps the routing row greppable while leaving one number to change.
pub const LSH_ONLY_NEARMISS_MIN_JACCARD: f64 = crate::pair::LSH_ONLY_MIN_JACCARD;
