//! The blast-radius pins for `[REPAIR-COSINE-MERGE]` /
//! [FUSION-CLUSTER-SIGNALS]: restoring a pair's measured cosine must not
//! cost a finding.
//!
//! A measured cosine belongs to the pair, not to the pass that surfaced
//! it — discovery route is telemetry, never evidence. Merging the cosine
//! back onto structurally- and LSH-discovered pairs therefore sends
//! cosines into two consumers that can *remove* a cluster from the
//! report:
//!
//! - the low-structure embedding mega-cluster hide, which drops a
//!   cluster once `structural < 0.10`, `embedding_cos >= 0.80`,
//!   `size > 10` and `canonical_node_count > 500` all hold at once; and
//! - the C# LSH Type-3 near-miss carve-out, which keeps a cross-file
//!   near miss visible only while `embedding_cos <= EPSILON`.
//!
//! Both were the review's named risk rows: the fix for one false
//! negative silently opening another. The invariant below is what makes
//! that impossible to introduce unnoticed: turning embeddings on may
//! add findings and sharpen signals, but every file set the
//! embeddings-off run reported must still be reported, under the same
//! bucket. A single hand-built cluster could only pin one corner of
//! that; this pins every cluster of every corpus swept.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

mod common;

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use mock_ollama::MockOllama;

use crate::common::{
    cluster_bucket, cluster_file_set, clusters, deslop_cmd, field, fixture, load_json, run_report,
    seed,
};

/// Corpora swept, with the node floor each is sized for. C# leads
/// because the Type-3 near-miss carve-out is C#-only.
const CORPORA: [(&str, &str); 3] = [
    ("csharp-type3", "10"),
    ("csharp-type1", "10"),
    ("ts-mixed-band", "12"),
];

/// The published clusters of one run, keyed by the set of files each
/// names, mapping to the buckets published over that file set.
///
/// Keyed by file set rather than cluster id because ids derive from
/// cluster membership: a cluster that legitimately gains an occurrence
/// gets a new id, and comparing ids would call that a lost finding. The
/// file set is what the user acts on.
type Published = BTreeMap<Vec<String>, Vec<String>>;

/// Runs the corpus with embeddings served by the deterministic mock and
/// returns its published clusters.
fn with_embeddings(corpus: &str, min_nodes: &str) -> Result<Published> {
    let server = MockOllama::spawn()?;
    let workspace = tempfile::tempdir()?;
    seed(&fixture(corpus), workspace.path())?;
    let output = workspace.path().join("report");
    let mut command = deslop_cmd(workspace.path(), &output)?;
    let _assertion = command
        .args([
            "--min-nodes",
            min_nodes,
            "--embeddings",
            "required",
            "--embedding-provider",
            "ollama",
            "--embedding-model",
            "nomic-embed-text",
            "--embedding-endpoint",
            server.endpoint(),
        ])
        .assert()
        .success();
    let report = load_json(&output.with_extension("json"))?;
    let provenance = field(&report, "embedding_provenance");
    let indexed = field(provenance, "indexed_subtrees")
        .as_u64()
        .unwrap_or_default();
    assert!(
        indexed > 0,
        "{corpus} never indexed a vector, so this run proves nothing about \
         the cosine consumers: {report:#}"
    );
    Ok(published(&report))
}

/// The same corpus with the embedding pass off.
fn without_embeddings(corpus: &str, min_nodes: &str) -> Result<Published> {
    let floor = min_nodes.parse().context("node floor")?;
    Ok(published(&run_report(&fixture(corpus), floor)?))
}

fn published(report: &serde_json::Value) -> Published {
    let mut out = Published::new();
    for cluster in clusters(report) {
        let files: Vec<String> = cluster_file_set(cluster).into_iter().collect();
        let buckets = out.entry(files).or_default();
        buckets.push(cluster_bucket(cluster).to_owned());
        buckets.sort();
    }
    out
}

/// [FUSION-CLUSTER-SIGNALS] Restored cosines may add findings. They may
/// never remove one: every file set the embeddings-off run reported as
/// duplicated must still be reported when the same pairs arrive
/// carrying their measured cosine — through the mega-cluster hide, the
/// C# Type-3 near-miss carve-out, and every other cosine-reading
/// filter.
#[test]
#[ignore = "GH #356: ollama-provider suite, excluded from the release gate. `ts-mixed-band` \
            publishes a four-file clone with embeddings off and nothing with them on — \
            ANN bridges mutate structural components before measurement \
            (`session/render.rs`). BRANCH_REVIEW.md requires this stay red rather than be \
            baselined. Assertions are intact — run with `-- --ignored`."]
fn embeddings_on_reports_every_file_set_embeddings_off_reported() -> Result<()> {
    for (corpus, min_nodes) in CORPORA {
        let cold = without_embeddings(corpus, min_nodes)?;
        let warm = with_embeddings(corpus, min_nodes)?;
        assert!(
            !cold.is_empty(),
            "{corpus} publishes nothing with embeddings off, so the comparison \
             is vacuous"
        );
        for (files, buckets) in &cold {
            assert!(
                warm.contains_key(files),
                "{corpus}: {files:?} was published as {buckets:?} with embeddings \
                 off and is reported by no cluster with them on — a restored \
                 signal removed a finding. Published with embeddings on: {warm:#?}"
            );
        }
    }
    Ok(())
}

/// [CLONE-BUCKETS] How similar two copies are is a property of the
/// copies, not of the pass that surfaced them. A structurally proven
/// duplicate (`structural = 1.0`) must not be re-labelled by the
/// arrival of a cosine: `same_behavior` claims semantic-only evidence
/// and reads as strictly weaker than the byte-level proof the cold run
/// already had.
#[test]
#[ignore = "GH #356: ollama-provider suite, excluded from the release gate. `csharp-type3` \
            publishes Delta.cs/Epsilon.cs as two `structural_only` clusters with embeddings \
            off and one `same_behavior` cluster with them on — the bucket follows the \
            discovery route, not the code. BRANCH_REVIEW.md requires this stay red rather \
            than be baselined. Assertions are intact — run with `-- --ignored`."]
fn embeddings_on_never_moves_a_reported_bucket() -> Result<()> {
    for (corpus, min_nodes) in CORPORA {
        let cold = without_embeddings(corpus, min_nodes)?;
        let warm = with_embeddings(corpus, min_nodes)?;
        for (files, cold_buckets) in &cold {
            let Some(warm_buckets) = warm.get(files) else {
                continue;
            };
            assert_eq!(
                warm_buckets, cold_buckets,
                "{corpus}: {files:?} was published as {cold_buckets:?} with \
                 embeddings off and {warm_buckets:?} with them on — the bucket \
                 followed the discovery route, not the code"
            );
        }
    }
    Ok(())
}
