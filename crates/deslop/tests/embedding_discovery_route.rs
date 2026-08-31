//! Discovery route is telemetry, never evidence
//! ([FUSED-PAIR-SIGNALS], GH #351).
//!
//! A measured cosine belongs to the pair, not to the pass that surfaced
//! it. `add_embedding_pair` used to discard `pair.cosine` whenever the
//! structural or LSH pass had already entered the pair, so what the user
//! saw depended on which pass reached a duplicate first: a byte-identical
//! pair found structurally rendered `embedding_cos = 0.0` —
//! indistinguishable from "measured, found unrelated" — while the same
//! pair reached by ANN alone rendered its true cosine. For cross-file LSH
//! pairs the loss also reclassified them `lsh_only`, which demands
//! `token_jaccard >= 0.90` instead of the fused gate and routed the
//! cluster to `loosely_similar`, hiding it outright.
//!
//! A second layer sat underneath: the embedding pass deduplicated
//! fingerprints by snippet content hash, so only the first fingerprint
//! with a given body received a vector. Byte-identical duplicates share
//! their body by definition, so the more perfect the duplicate, the more
//! certainly its embedding evidence was destroyed.

use crate::mock_ollama::MockOllama;
use anyhow::Result;
use serde_json::Value;

use crate::common::{
    embeddings::run_mock_embedding_report,
    signals::{
        assert_no_pair_surface_on_cluster, assert_pair_metric, assert_structural_only_contract,
        compare_pair_with_embeddings, has_verbatim_pair, occurrence_for_file,
    },
    *,
};

const MIN_NODES: u32 = 8;
const MIN_NODES_ARG: &str = "8";
const EXACT_SCORE: f64 = 1.0;
const MISSING_SCORE: f64 = 0.0;
const LEFT_FILE: &str = "Alpha.cs";
const RIGHT_FILE: &str = "Beta.cs";
const EXACT_COPY_FILE: &str = "AlphaCopy.cs";

/// Scans `csharp-small` with the deterministic mock embedder wired in at
/// the given `min_nodes`, so the same corpus can be reached through
/// different discovery routes.
fn run_with_embeddings(server: &MockOllama, min_nodes: &str) -> Result<Value> {
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let output = workspace.path().join("report");
    run_mock_embedding_report(workspace.path(), &output, min_nodes, server.endpoint())
}

// A pair the structural pass already holds must still be admitted when no
// embedding vector is available for that exact pair. The canonical
// enclosing views differ by class name, but their normalised structure and
// consistent rename evidence independently meet pair admission.
#[test]
fn a_structurally_discovered_pair_does_not_require_an_embedding() -> Result<()> {
    let server = MockOllama::spawn()?;
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let output = workspace.path().join("report");
    let report =
        run_mock_embedding_report(workspace.path(), &output, MIN_NODES_ARG, server.endpoint())?;
    let cluster = expect_cluster_spanning(&report, &[LEFT_FILE, RIGHT_FILE])?;
    assert_structural_only_contract(cluster, "structurally discovered Type-2 pair");
    assert_no_pair_surface_on_cluster(cluster, "structurally discovered Type-2 pair");
    let comparison = compare_pair_with_embeddings(
        workspace.path(),
        MIN_NODES,
        occurrence_for_file(cluster, LEFT_FILE)?,
        occurrence_for_file(cluster, RIGHT_FILE)?,
    )?;
    assert_pair_metric(
        comparison.evidence.embedding_cos,
        MISSING_SCORE,
        "the optional embedding axis is absent for this explicit pair",
    );
    assert!(
        comparison.evidence.admitted,
        "the explicit pair must remain admitted: {comparison:#?}"
    );
    Ok(())
}

// The same corpus reached at two `--min-nodes` values changes which pass
// surfaces a pair first. Neither run may lose the embedding axis, and
// neither may route a duplicate to the hidden bucket the other shows.
#[test]
fn changing_the_discovery_route_never_erases_the_embedding_axis() -> Result<()> {
    let server = MockOllama::spawn()?;
    let coarse = run_with_embeddings(&server, "12")?;
    let fine = run_with_embeddings(&server, "8")?;
    for (label, report) in [("min-nodes 12", &coarse), ("min-nodes 8", &fine)] {
        assert!(
            cluster_count(report) > 0,
            "{label}: the fixture must stay visible at both thresholds: {report:#}"
        );
        assert!(
            cluster_count(report) > 0,
            "{label}: the fixture must stay visible at both thresholds: {report:#}"
        );
    }
    assert_eq!(
        clusters_hidden(&coarse),
        clusters_hidden(&fine),
        "discovery route changed how many duplicates were hidden: coarse \
         {coarse:#} vs fine {fine:#}"
    );
    Ok(())
}

// Embedding evidence must survive content-hash deduplication. The exact
// copy shares the source body with `Alpha`, so its vector is deduplicated
// at provider dispatch but must fan back out to both explicit endpoints.
#[test]
fn byte_identical_bodies_each_retain_their_measured_embedding() -> Result<()> {
    let server = MockOllama::spawn()?;
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let _copied = std::fs::copy(
        workspace.path().join(LEFT_FILE),
        workspace.path().join(EXACT_COPY_FILE),
    )?;
    let output = workspace.path().join("report");
    let report =
        run_mock_embedding_report(workspace.path(), &output, MIN_NODES_ARG, server.endpoint())?;
    let cluster = expect_cluster_spanning(&report, &[LEFT_FILE, EXACT_COPY_FILE])?;
    assert!(
        has_verbatim_pair(workspace.path(), cluster)?,
        "the copied file must preserve an exact pair: {cluster:#}"
    );
    let comparison = compare_pair_with_embeddings(
        workspace.path(),
        MIN_NODES,
        occurrence_for_file(cluster, LEFT_FILE)?,
        occurrence_for_file(cluster, EXACT_COPY_FILE)?,
    )?;
    assert_pair_metric(
        comparison.evidence.embedding_cos,
        EXACT_SCORE,
        "identical endpoints retain their deduplicated embedding",
    );
    assert!(
        comparison.evidence.admitted,
        "the exact pair must remain admitted: {comparison:#?}"
    );
    assert_no_pair_surface_on_cluster(cluster, "deduplicated exact pair");
    Ok(())
}
