//! Black-box regression for malformed embedding vectors.
//!
//! A provider can return a valid JSON number that overflows `f32`. Such a
//! vector is rejected evidence: it must never enter the cache, ANN index,
//! candidate graph, or rendered signal triple.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

mod common;

use anyhow::Result;
use mock_ollama::{MockBehavior, MockOllama};

use crate::common::{clusters, deslop_cmd, field, fixture, load_json, seed};

/// [FUSION-EMBED-PROVIDER] Every overflowing vector is accounted as failed,
/// while the deterministic pipeline still returns a valid finite report.
#[test]
fn overflowing_json_vectors_are_rejected_before_cache_index_and_report() -> Result<()> {
    let server = MockOllama::spawn_with(MockBehavior::OverflowingEmbeddings)?;
    let workspace = tempfile::tempdir()?;
    seed(&fixture("csharp-small"), workspace.path())?;
    let output = workspace.path().join("report");
    let mut command = deslop_cmd(workspace.path(), &output)?;
    let _assertion = command
        .args([
            "--min-nodes",
            "8",
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
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "{report:#}"
    );
    let provenance = field(&report, "embedding_provenance");
    let attempted = field(provenance, "attempted_subtrees")
        .as_u64()
        .unwrap_or_default();
    assert!(
        attempted > 0,
        "fixture never exercised the provider: {report:#}"
    );
    assert_eq!(
        field(provenance, "indexed_subtrees").as_u64(),
        Some(0),
        "an invalid vector reached the cache or ANN index: {provenance:#}"
    );
    assert_eq!(
        field(provenance, "failed_subtrees").as_u64(),
        Some(attempted),
        "every non-finite occurrence must be counted as failed: {provenance:#}"
    );
    for cluster in clusters(&report) {
        let signals = field(cluster, "signals");
        assert_eq!(
            field(signals, "embedding_cos").as_f64(),
            Some(0.0),
            "invalid provider evidence escaped into a cluster: {cluster:#}"
        );
        let fused = field(signals, "fused").as_f64().unwrap_or(f64::NAN);
        assert!(
            fused.is_finite(),
            "non-finite fused signal escaped: {cluster:#}"
        );
        assert!(
            (0.0..=1.0).contains(&fused),
            "fused escaped [0,1]: {cluster:#}"
        );
    }
    Ok(())
}
