//! Visibility decisions for the rendered report.
//!
//! Every rule here removes a cluster the pipeline built, so each one is
//! a deliberate precision trade with a defect behind it — and a
//! duplicate that silently disappears is the hardest defect class to
//! notice. Split from [`super`] so the rules, and the trace that
//! records which one fired, sit together rather than inside the
//! rendering walk.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    buckets::{classify, is_shape_corroborated_nearmiss, is_token_carried_nearmiss, ClusterKind},
    clone_category::CloneCategory,
    cluster::Cluster,
    cluster_filters::{
        is_embedding_role_mismatch, is_noise_pattern, is_single_file_declaration_family, ParseCache,
    },
    state::FileId,
};

use super::{ReportCluster, ReportInputs, ReportSignals};

/// Records why a cluster left the visible report. A duplicate that
/// silently disappears is the hardest defect class to notice, so the
/// routing decision — signals and measured content evidence included —
/// is traceable without re-running the pipeline.
pub(super) fn log_hidden_cluster(
    cluster: &ReportCluster,
    content: crate::content::ContentEvidence,
    as_data: bool,
    as_structural_only: bool,
) {
    tracing::debug!(
        cluster = cluster.id.as_str(),
        bucket = cluster.bucket.as_str(),
        category = cluster.category.as_str(),
        occurrences = cluster.occurrences.len(),
        structural = cluster.signals.structural,
        token_jaccard = cluster.signals.token_jaccard,
        embedding_cos = cluster.signals.embedding_cos,
        content_agreement = content.agreement,
        content_rename_consistency = content.rename_consistency,
        content_substance_varies = content.substance_varies,
        dropped_as_data = as_data,
        dropped_as_structural_only = as_structural_only,
        "cluster hidden from report",
    );
}

/// Decides whether a cluster must be dropped from the ranked report.
///
/// The cheap test runs first: a cluster whose every occurrence sits in a
/// report-hidden path (e.g. all members in generated `*.g.dart` /
/// `*.freezed.dart` files) is dropped regardless of the expensive
/// re-parse checks below, so those are skipped. Without this a large
/// generated file is re-walked once per cluster only to be hidden anyway,
/// dominating analysis time on codegen-heavy Dart/Flutter repos
/// ([CLONE-NOISE-REPARSE-CACHE]). The remaining rules:
/// - `#58`: `LooselySimilar` clusters carry only token overlap, no
///   structural/semantic anchor — token-only boilerplate, not duplication.
/// - `#120/#122`: low-structure embedding mega-clusters are report-dominating
///   `Type-4` false positives.
/// - `#69/#70/#71/#72`: re-parsed noise patterns (polymorphic interface
///   impls, test-data variation, REST shape, scaffolding).
/// - `#119`: embedding-dominant `same_behavior` pairs of incompatible roles.
pub(super) fn cluster_is_hidden<S: BuildHasher>(
    cluster: &Cluster,
    report_cluster: &ReportCluster,
    inputs: &ReportInputs<'_, S>,
    parse_cache: &ParseCache,
    category: CloneCategory,
) -> bool {
    let occurrences_all_hidden = !report_cluster.occurrences.is_empty()
        && report_cluster.occurrences.iter().all(|occ| occ.hidden);
    if occurrences_all_hidden {
        return true;
    }
    let kind = classify(report_cluster);
    // [DECISION-CROSS-LANGUAGE] Cross-language clusters stay hidden unless the
    // opt-in is enabled — off by default, no heuristics or type-system bridges.
    let token_only_or_mega = (kind == ClusterKind::LooselySimilar
        || is_low_structure_embedding_mega_cluster(report_cluster))
        && !(inputs.exclusion.allows_cross_language_comparison()
            && spans_multiple_languages(&cluster.members, inputs.file_languages));
    let noise = is_noise_pattern(
        &cluster.members,
        inputs.sources,
        inputs.file_languages,
        parse_cache,
    );
    // [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] The gate exists because
    // *embedding* evidence can pair role-incompatible code — a reader
    // against a writer that the model scores alike. It was keyed on the
    // `same_behavior` bucket because that was the only route embedding
    // evidence could carry a cluster through. It is no longer: a
    // shared-subtree near-miss may now be corroborated by the embedding
    // axis instead of the token axis ([FUSION-SHARED-SUBTREE]), so the
    // same evidence reaches an act-now bucket by a second door and must
    // meet the same gate. Keyed on the bucket alone, the Python
    // role-mismatch pair walked straight through it.
    let gate_signals: ReportSignals = cluster.signals.into();
    let embedding_carried_nearmiss = kind == ClusterKind::NearlyIdentical
        && gate_signals.structural < 0.99
        && gate_signals.token_jaccard < crate::pair::SHARED_SUBTREE_MIN_JACCARD
        && gate_signals.embedding_cos >= crate::pair::EMBEDDING_SUPPORT_FLOOR;
    let role_mismatch = (kind == ClusterKind::SameBehavior || embedding_carried_nearmiss)
        && is_embedding_role_mismatch(
            &cluster.members,
            inputs.sources,
            inputs.file_languages,
            parse_cache,
        );
    // [RANK-STRUCTURAL-ONLY] A single-file `structural_only` family of
    // sibling declarations (REST CRUD / settings / builder methods) is the
    // same evidence-free noise as the cross-file scaffolding in
    // [CLONE-NOISE-SCAFFOLDING]. The content check confines this to members
    // that provably differ in substance, so worth-extracting clones — a
    // consistent rename over preserved literals — stay visible (demoted,
    // not hidden).
    //
    // The anchor-free near-miss ([CLONE-BUCKETS-ROUTING] row 4, judged on
    // the *raw* signals the routing itself used) must consult the same
    // proof. The #197 settings family is one family whichever door it
    // arrives through: the offset-invariant #339 sibling-window
    // signatures inverted its triple from `structural=1.00,
    // token_jaccard=0.00` to `structural=0.00, token_jaccard=0.91`, and
    // gating the proof on the `structural_only` label alone let the very
    // wrappers it was written to convict ride row 4 into the act-now
    // tier as the top offender. The proof, not the bucket, is the
    // discriminator: a genuine in-file Type-3 copy whose body binds
    // locals, loops or branches fails `forwarding_body` and stays
    // visible, so recall pays nothing.
    // The near-miss legs are gated on the routed bucket, not read off
    // the raw signals alone. This proof is about shape/token-only
    // evidence, so a cluster the router sent to `same_behavior` — where
    // the semantic axis is the strongest evidence ([CLONE-BUCKETS-ROUTING]
    // row 2) — must not be convicted by it. Ungated, an honest
    // `structural` made that routine: a `while` loop and a `for` loop
    // over one accumulator chain measure 0.81 shape and 0.68 tokens, so
    // the row-4b predicate fired on a Type-4 cluster and the single-file
    // family gate hid it outright.
    let signals: ReportSignals = cluster.signals.into();
    let family_evidence_kind = kind == ClusterKind::StructuralOnly
        || (kind == ClusterKind::NearlyIdentical
            && (is_token_carried_nearmiss(signals) || is_shape_corroborated_nearmiss(signals)));
    let single_file_declaration_family = family_evidence_kind
        && is_single_file_declaration_family(
            cluster,
            category,
            inputs.sources,
            inputs.file_languages,
            parse_cache,
        );
    token_only_or_mega || noise || role_mismatch || single_file_declaration_family
}

/// Returns true for embedding-dominant mega-clusters that are too broad
/// to be actionable. Keeps small Type-4 pairs available while suppressing
/// the real-world "all pytest modules are related" closure failure.
fn is_low_structure_embedding_mega_cluster(cluster: &ReportCluster) -> bool {
    cluster.signals.structural < 0.10
        && cluster.signals.embedding_cos >= crate::pair::EMBEDDING_SUPPORT_FLOOR
        && cluster.size > 10
        && cluster.canonical_node_count > 500
}

/// Returns true when a cluster contains more than one parser language id.
fn spans_multiple_languages<S: BuildHasher>(
    members: &[crate::fingerprint::Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    let mut languages = members
        .iter()
        .filter_map(|member| file_languages.get(&member.file_id).copied());
    let Some(first) = languages.next() else {
        return false;
    };
    languages.any(|language| language != first)
}

/// Bucket totals emitted for GH#45 classification observability.
#[derive(Default)]
pub(super) struct BucketDistribution {
    /// Visible identical-code clusters.
    identical: usize,
    /// Visible nearly-identical clusters.
    nearly_identical: usize,
    /// Visible structural-only clusters.
    structural_only: usize,
    /// Visible loosely-similar clusters.
    loosely_similar: usize,
    /// Visible same-behavior clusters.
    same_behavior: usize,
}

impl BucketDistribution {
    /// Counts visible report clusters by canonical bucket.
    pub(super) fn from_clusters(clusters: &[ReportCluster]) -> Self {
        let mut distribution = Self::default();
        for cluster in clusters {
            distribution.add(classify(cluster));
        }
        distribution
    }

    /// Increments one bucket.
    fn add(&mut self, kind: ClusterKind) {
        match kind {
            ClusterKind::Identical => self.identical = self.identical.saturating_add(1),
            ClusterKind::NearlyIdentical => {
                self.nearly_identical = self.nearly_identical.saturating_add(1);
            }
            ClusterKind::StructuralOnly => {
                self.structural_only = self.structural_only.saturating_add(1);
            }
            ClusterKind::LooselySimilar => {
                self.loosely_similar = self.loosely_similar.saturating_add(1);
            }
            ClusterKind::SameBehavior => self.same_behavior = self.same_behavior.saturating_add(1),
        }
    }

    /// Emits the structured classification distribution.
    ///
    /// The target is pinned to the *stage* rather than this module: the
    /// [issue #45] observability contract names one target per pipeline
    /// stage (`deslop_core::pair`, `::cluster`, `::report`), and this
    /// event is the report stage's. Splitting `report.rs` for the
    /// file-length rule moved the call site into `report::hidden` and
    /// silently renamed the target with it — a file-organisation change
    /// must not move an observable contract
    /// (`issue_45_observability::issue_45_pipeline_emits_stage_observability_events`).
    pub(super) fn log(self, visible: usize, hidden: usize) {
        tracing::info!(
            target: "deslop_core::report",
            visible,
            hidden,
            identical = self.identical,
            nearly_identical = self.nearly_identical,
            structural_only = self.structural_only,
            loosely_similar = self.loosely_similar,
            same_behavior = self.same_behavior,
            "bucket distribution",
        );
    }
}
