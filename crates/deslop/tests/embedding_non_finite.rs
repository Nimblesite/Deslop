//! Black-box regression for malformed embedding vectors.
//!
//! A provider can return a valid JSON number that overflows `f32`. Such a
//! vector is rejected evidence: it must never enter the cache, the ANN
//! index, the candidate graph, or any rendered surface.
//!
//! The mass-only wire carries no cluster-level signal triple at all
//! ([FUSED-SCOPE]: every similarity symbol is a function of a pair, none
//! of a cluster; [RANK-MASS-SUM]: mass alone ranks). So "the invalid
//! vector never reached the report" is proven here by the *absence* of
//! every pair-evidence field rather than by a zeroed `embedding_cos` — a
//! cluster that cannot name an embedding cannot leak one, and the
//! absence holds even if a routing table were reintroduced.
//!
//! Rejecting every vector must not blind the detector either: the
//! structural pipeline is deterministic and independent of the provider,
//! so the fixture's clone still publishes with positive mass.

use crate::mock_ollama::{MockBehavior, MockOllama};
use anyhow::Result;

use crate::common::{
    clusters, embeddings::mock_embedding_run, field, signals::assert_no_pair_surface_on_cluster,
};

/// Source files the `csharp-small` fixture contributes to the scan.
const EXPECTED_FILES_ANALYSED: u64 = 2;

/// Subtrees allowed into the cache or ANN index when every vector overflows.
const EXPECTED_INDEXED_SUBTREES: u64 = 0;

/// Clusters the fixture's byte-identical clone must still publish.
const MIN_PUBLISHED_CLUSTERS: usize = 1;

/// Occurrences a published duplicate cluster holds at minimum.
const MIN_OCCURRENCES: usize = 2;

/// Embedding-derived names the mass-only wire forbids on a cluster. The
/// shared pair-surface check covers `signals`; these are the specific
/// leaks this fixture manufactures, asserted by name so a regression
/// that re-adds one is reported as itself.
const FORBIDDEN_EMBEDDING_FIELDS: [&str; 4] =
    ["embedding_cos", "embedding", "fused", "fused_score"];

/// Every overflowing vector is accounted as failed, and none of them
/// reaches the cache or the ANN index ([FUSED-EMBED-PROVIDER]).
fn assert_every_vector_was_rejected(report: &serde_json::Value) {
    let provenance = field(report, "embedding_provenance");
    let attempted = field(provenance, "attempted_subtrees")
        .as_u64()
        .unwrap_or_default();
    assert!(
        attempted > 0,
        "fixture never exercised the provider: {report:#}"
    );
    assert_eq!(
        field(provenance, "indexed_subtrees").as_u64(),
        Some(EXPECTED_INDEXED_SUBTREES),
        "an invalid vector reached the cache or ANN index: {provenance:#}"
    );
    assert_eq!(
        field(provenance, "failed_subtrees").as_u64(),
        Some(attempted),
        "every non-finite occurrence must be counted as failed: {provenance:#}"
    );
}

/// No rejected vector may surface as cluster evidence. The mass-only
/// wire has no place to put one, and this asserts that place stays gone.
fn assert_no_embedding_evidence_escaped(cluster: &serde_json::Value) {
    assert_no_pair_surface_on_cluster(cluster, "overflowing embedding provider");
    for forbidden in FORBIDDEN_EMBEDDING_FIELDS {
        assert!(
            cluster.get(forbidden).is_none(),
            "rejected provider evidence escaped as cluster {forbidden}: {cluster:#}"
        );
    }
}

/// Occurrences the report actually publishes — `report_hide` and
/// otherwise-hidden rows do not count toward mass ([RANK-MASS-SUM]).
fn visible_occurrences(cluster: &serde_json::Value) -> usize {
    field(cluster, "occurrences").as_array().map_or(0, |rows| {
        rows.iter()
            .filter(|row| !field(row, "hidden").as_bool().unwrap_or(false))
            .count()
    })
}

/// A provider failure must not cost the structural finding: the clone is
/// still published, still ranked, and still carries positive mass
/// ([RANK-MASS-SUM]).
fn assert_structural_finding_survived(cluster: &serde_json::Value) {
    let mass = field(cluster, "mass").as_u64().unwrap_or_default();
    assert!(mass > 0, "a published cluster must carry mass: {cluster:#}");
    let visible = visible_occurrences(cluster);
    assert!(
        visible >= MIN_OCCURRENCES,
        "a duplicate needs at least two visible occurrences: {cluster:#}"
    );
    let nodes = field(cluster, "canonical_node_count")
        .as_u64()
        .unwrap_or_default();
    let expected =
        nodes.saturating_mul(u64::try_from(visible).unwrap_or_default().saturating_sub(1));
    assert_eq!(
        mass, expected,
        "[RANK-MASS-SUM] mass is canonical nodes x additional visible \
         occurrences: {nodes} x ({visible} - 1) = {expected}: {cluster:#}"
    );
    assert!(
        field(cluster, "rank_band")
            .as_str()
            .is_some_and(|band| !band.is_empty()),
        "every published cluster carries a mass-derived rank band: {cluster:#}"
    );
}

/// [FUSED-EMBED-PROVIDER] Every overflowing vector is accounted as failed,
/// while the deterministic pipeline still returns a valid finite report.
#[test]
fn overflowing_json_vectors_are_rejected_before_cache_index_and_report() -> Result<()> {
    let server = MockOllama::spawn_with(MockBehavior::OverflowingEmbeddings)?;
    let (_workspace, report) = mock_embedding_run(&server, "csharp-small", "8")?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(EXPECTED_FILES_ANALYSED),
        "{report:#}"
    );
    assert_every_vector_was_rejected(&report);
    let published = clusters(&report);
    assert!(
        published.len() >= MIN_PUBLISHED_CLUSTERS,
        "rejecting every vector must not blind the structural pipeline: {report:#}"
    );
    for cluster in published {
        assert_no_embedding_evidence_escaped(cluster);
        assert_structural_finding_survived(cluster);
    }
    Ok(())
}
