//! Discovery route is telemetry, never evidence
//! ([FUSION-CLUSTER-SIGNALS], GH #351).
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

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use anyhow::Result;
use mock_ollama::MockOllama;
use serde_json::Value;

use crate::common::{embeddings::run_mock_embedding_report, *};

/// Scans `csharp-small` with the deterministic mock embedder wired in at
/// the given `min_nodes`, so the same corpus can be reached through
/// different discovery routes.
fn run_with_embeddings(server: &MockOllama, min_nodes: &str) -> Result<Value> {
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let output = workspace.path().join("report");
    run_mock_embedding_report(workspace.path(), &output, min_nodes, server.endpoint())
}

/// Highest cosine rendered on any visible cluster.
fn peak_embedding_cos(report: &Value) -> f64 {
    clusters(report)
        .iter()
        .map(|cluster| signal(cluster, "embedding_cos"))
        .fold(0.0_f64, f64::max)
}

// A pair the structural pass already holds must still render the cosine
// the embedding pass measured for it. Rendering 0.0 for a pair the
// provider scored is the report asserting a measurement that never
// happened.
#[test]
fn a_structurally_discovered_pair_still_renders_its_measured_cosine() -> Result<()> {
    let server = MockOllama::spawn()?;
    let report = run_with_embeddings(&server, "8")?;
    assert!(
        cluster_count(&report) > 0,
        "the fixture must produce at least one visible cluster: {report:#}"
    );
    let structural_clusters: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| signal(cluster, "structural") > 0.99)
        .collect();
    assert!(
        !structural_clusters.is_empty(),
        "the fixture must produce a structurally-proven cluster, or this test \
         cannot observe the discard: {report:#}"
    );
    for cluster in structural_clusters {
        assert!(
            signal(cluster, "embedding_cos") > 0.0,
            "cluster {id} was proven structurally and measured by the provider, \
             yet renders embedding_cos = 0 — the pass that found it first \
             decided what the user sees: {report:#}",
            id = cluster_id(cluster),
        );
    }
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
            peak_embedding_cos(report) > 0.0,
            "{label}: every visible cluster renders embedding_cos = 0, so the \
             measured evidence was dropped on this route: {report:#}"
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

// Embedding evidence must survive content-hash deduplication. Two
// byte-identical bodies share one provider request by design; the result
// has to fan back out to every fingerprint in the group, or the most
// perfect duplicates are exactly the ones rendered as unmeasured.
#[test]
fn byte_identical_bodies_each_receive_the_shared_vector() -> Result<()> {
    let server = MockOllama::spawn()?;
    let report = run_with_embeddings(&server, "8")?;
    let measured = clusters(&report)
        .iter()
        .filter(|cluster| signal(cluster, "embedding_cos") > 0.0)
        .count();
    assert_eq!(
        measured,
        cluster_count(&report),
        "every visible cluster was submitted to the provider, so every one must \
         carry a measured cosine: {report:#}"
    );
    Ok(())
}
