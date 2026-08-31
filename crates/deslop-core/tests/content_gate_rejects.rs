//! [FUSED-CONTENT-GATE] Pipeline regression coverage: a content-rejected pair never forms a closure.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use deslop_core::{
    error::CoreError,
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    EmbeddingMode, Report,
};

const DART_FORWARDING_FIXTURE: &str = "../deslop/tests/fixtures/dart-forwarding-business-pair";
const MIN_NODES: u32 = 12;
const EXPECTED_VISIBLE_CLUSTERS: usize = 0;
const EXPECTED_HIDDEN_CLUSTERS: usize = 0;
const EXPECTED_SCHEMA_FILES_ANALYSED: usize = 1;
const SCHEMA_FILE: &str = "schemas.py";
const SCHEMA_SOURCE: &str = "def schema_report_get():\n    return {\"type\": \"object\", \"properties\": {\"path\": {\"type\": \"string\"}}, \"required\": [\"path\"]}\n\ndef schema_top_offenders():\n    return {\"type\": \"object\", \"properties\": {\"limit\": {\"type\": \"integer\"}}, \"required\": [\"limit\"]}\n";

#[test]
fn content_gate_rejects_dart_forwarding_business_pair() -> Result<(), CoreError> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(DART_FORWARDING_FIXTURE);
    let report = run_without_embeddings(root)?;
    assert_eq!(
        report.clusters.len(),
        EXPECTED_VISIBLE_CLUSTERS,
        "the content-rejected pair must not form a visible closure"
    );
    assert_eq!(
        report.clusters_hidden, EXPECTED_HIDDEN_CLUSTERS,
        "the pair must be rejected before closure, not hidden after rendering"
    );
    Ok(())
}

/// [FUSED-CONTENT-GATE] The schema pair shares normalised shape but not
/// authored content, so the candidate edge is rejected before closure.
#[test]
fn schema_token_only_pair_is_rejected_before_closure() -> Result<()> {
    let root = tempfile::tempdir().context("schema fixture directory")?;
    fs::write(root.path().join(SCHEMA_FILE), SCHEMA_SOURCE).context("write schema fixture")?;
    let report = run_without_embeddings(root.path().to_path_buf())?;
    assert_eq!(
        report.files_analysed, EXPECTED_SCHEMA_FILES_ANALYSED,
        "the schema fixture must analyse its authored source file"
    );
    assert_eq!(
        report.clusters.len(),
        EXPECTED_VISIBLE_CLUSTERS,
        "distinct schema functions must not form a visible closure"
    );
    assert!(
        report.clusters.is_empty(),
        "distinct schema functions must not form a reported closure: {:?}",
        report.clusters
    );
    assert_eq!(
        report.clusters_hidden, EXPECTED_HIDDEN_CLUSTERS,
        "the pair must be rejected before closure, not hidden after rendering"
    );
    Ok(())
}

fn run_without_embeddings(root: PathBuf) -> Result<Report, CoreError> {
    run(&PipelineConfig {
        root,
        min_nodes: MIN_NODES,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Off,
            provider: None,
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
}
