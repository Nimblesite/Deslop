//! The deterministic mock-embedder entry point shared by every
//! embedding suite ([FUSION-EMBED-PROVIDER]).
//!
//! Imported explicitly with `use crate::common::embeddings::*;`, for the
//! same reason as `signals`: a glob re-export would be an unused import
//! in every binary that never wires an embedder.

use std::path::Path;

use serde_json::Value;

use super::{deslop_cmd, load_json, Result};

/// Runs a scan with the deterministic mock embedder wired in and
/// returns the parsed report ([FUSION-EMBED-PROVIDER]).
///
/// Every embedding suite reaches the pipeline through the same four
/// flags — the provider, model and `--embeddings required` mode are
/// fixed by the mock, and only the scan root, output prefix, `min_nodes`
/// and endpoint vary. Restating that argument list per suite is how a
/// flag rename would silently reach only some of them.
pub(crate) fn run_mock_embedding_report(
    scan_root: &Path,
    output: &Path,
    min_nodes: &str,
    endpoint: &str,
) -> Result<Value> {
    let mut command = deslop_cmd(scan_root, output)?;
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
            endpoint,
        ])
        .assert()
        .success();
    load_json(&output.with_extension("json"))
}
