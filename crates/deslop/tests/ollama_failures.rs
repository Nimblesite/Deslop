//! End-to-end coverage for rejected Ollama subtree embeddings.
//!
//! Issue #5: provider failures must not be represented as zero vectors.

use std::{fs, path::Path};

use anyhow::{anyhow, Result};
use serde_json::Value;

mod common;
use crate::common::*;

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;
use mock_ollama::{MockBehavior, MockOllama};

#[test]
fn mock_provider_rejected_subtrees_are_reported() -> Result<()> {
    let server = MockOllama::spawn_with(MockBehavior::RejectAllEmbeds)?;
    let provenance = run_with_ollama(server.endpoint())?;
    let attempted = metric(&provenance, "attempted_subtrees");
    let failed = metric(&provenance, "failed_subtrees");
    assert!(
        attempted > 0,
        "embedding attempts must be surfaced: {provenance}"
    );
    assert!(
        failed > 0,
        "provider rejections must be counted: {provenance}"
    );
    assert!(
        failed <= attempted,
        "failed_subtrees cannot exceed attempted_subtrees: {provenance}"
    );
    assert!(
        server.max_embed_batch_len() > 1,
        "Ollama embeddings must be requested in batches; max batch was {}",
        server.max_embed_batch_len()
    );
    Ok(())
}

// [FUSION-EMBED-PROVIDER] A context-length rejection on an aggregate Ollama
// batch must bisect and retry rather than marking all subtrees as failed.
#[test]
fn ollama_context_rejection_retries_small_subtrees_individually() -> Result<()> {
    let server = MockOllama::spawn_with(MockBehavior::RejectMultiInputEmbeds)?;
    let provenance = run_with_ollama(server.endpoint())?;
    assert!(
        metric(&provenance, "attempted_subtrees") > 0,
        "embedding attempts must be surfaced: {provenance}"
    );
    assert_eq!(
        metric(&provenance, "failed_subtrees"),
        0,
        "a context error on an aggregate Ollama request must retry the small \
         snippets instead of marking the whole batch failed: {provenance}"
    );
    assert!(
        server.max_embed_batch_len() > 1,
        "fixture must reproduce an aggregate batch before retry; max batch was {}",
        server.max_embed_batch_len()
    );
    Ok(())
}

/// Seeds `csharp-small` into a temp scan root, runs deslop with required
/// Ollama embeddings against `endpoint`, asserts success, and returns the
/// report's `embedding_provenance` object.
fn run_with_ollama(endpoint: &str) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut cmd = deslop_cmd(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
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
            endpoint,
        ])
        .assert()
        .success();
    embedding_provenance(tmp.path())
}

fn seed_scan_root(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn embedding_provenance(tmp: &Path) -> Result<Value> {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    let report: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    report
        .get("embedding_provenance")
        .cloned()
        .ok_or_else(|| anyhow!("embedding_provenance missing: {report}"))
}

fn metric(provenance: &Value, field: &str) -> u64 {
    provenance
        .get(field)
        .and_then(Value::as_u64)
        .unwrap_or_default()
}
