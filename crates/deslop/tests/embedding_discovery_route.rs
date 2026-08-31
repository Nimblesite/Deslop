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

use crate::common::{embeddings::run_mock_embedding_report, signals::has_verbatim_pair, *};

/// Scans `csharp-small` with the deterministic mock embedder wired in at
/// the given `min_nodes`, so the same corpus can be reached through
/// different discovery routes.
fn run_with_embeddings(server: &MockOllama, min_nodes: &str) -> Result<Value> {
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let output = workspace.path().join("report");
    run_mock_embedding_report(workspace.path(), &output, min_nodes, server.endpoint())
}

// A pair the structural pass already holds must still be reported with
// embeddings on — the discovery route must not decide visibility.
// [PIPELINE-CLUSTER-CLOSURE]: the measured cosine is pair-scoped now, so
// the acceptance is pinned by the byte-proven fact instead.
#[test]
fn a_structurally_discovered_pair_still_renders_byte_proven() -> Result<()> {
    let server = MockOllama::spawn()?;
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let output = workspace.path().join("report");
    let report = run_mock_embedding_report(workspace.path(), &output, "8", server.endpoint())?;
    assert!(
        cluster_count(&report) > 0,
        "the fixture must produce at least one visible cluster: {report:#}"
    );
    for cluster in clusters(&report) {
        assert!(
            has_verbatim_pair(workspace.path(), cluster)?,
            "the byte-identical bodies must be byte-proven with embeddings on: \
             {cluster:#}"
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

// Embedding evidence must survive content-hash deduplication. Two
// byte-identical bodies share one provider request by design; the result
// has to fan back out to every fingerprint in the group, or the most
// perfect duplicates are exactly the ones rendered as unmeasured.
#[test]
fn byte_identical_bodies_each_remain_byte_proven() -> Result<()> {
    let server = MockOllama::spawn()?;
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let output = workspace.path().join("report");
    let report = run_mock_embedding_report(workspace.path(), &output, "8", server.endpoint())?;
    let byte_proven = clusters(&report)
        .iter()
        .filter(|cluster| has_verbatim_pair(workspace.path(), cluster).unwrap_or(false))
        .count();
    assert_eq!(
        byte_proven,
        cluster_count(&report),
        "every visible cluster must be byte-proven from the source: {report:#}"
    );
    Ok(())
}
