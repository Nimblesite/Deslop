//! [PAIR-COMPARE-CLI] Admission evidence for exactly two occurrences.
//!
//! Evidence is pair-scoped and recomputed on demand — it is never stored
//! on a cluster and never carried in a rendered report ([PIPELINE-FUSED],
//! gh #484). The LSP (`pair/compare`) and MCP (`compare_pair`) surfaces
//! could ask for it; the CLI could not, so the corpus gate — which drives
//! the CLI black-box — had no way to assert that a curated duplicate was
//! actually admitted with content evidence rather than merely reported
//! (gh #488).

use std::fs;

use anyhow::Result;
use serde_json::Value;

use crate::common::{
    clone_corpus::{dup_source, MIN_NODES},
    deslop_cmd, run_report,
    scan_dir::temp_scan_dir,
};

/// The two carriers of one duplicated function.
const CARRIERS: [&str; 2] = ["alpha.rs", "beta.rs"];

/// Every field an engine-authored pair verdict must carry.
const EVIDENCE_FIELDS: [&str; 8] = [
    "structural",
    "token_jaccard",
    "agreement",
    "rename_consistency",
    "fused_score",
    "content_required",
    "content_ok",
    "admitted",
];

/// The CLI answers for exactly the two endpoints it is given, echoing both
/// and returning the engine's own admission verdict.
#[test]
fn the_cli_reports_admission_evidence_for_two_named_occurrences() -> Result<()> {
    let (_tmp, scan_root) = temp_scan_dir("tree")?;
    for carrier in CARRIERS {
        fs::write(scan_root.join(carrier), dup_source(carrier))?;
    }
    // Endpoints name occurrences the engine already fingerprinted, so the
    // caller reads them out of a report — exactly the flow the corpus gate
    // uses, and the reason a cluster id is not valid input.
    let report = run_report(&scan_root, MIN_NODES)?;
    let occurrences = report
        .get("clusters")
        .and_then(Value::as_array)
        .and_then(|clusters| clusters.first())
        .and_then(|cluster| cluster.get("occurrences"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("the scan must report a cluster to compare: {report}"))?;
    assert_eq!(
        occurrences.len(),
        CARRIERS.len(),
        "both carriers must be occurrences of one cluster, or there is no pair to compare"
    );
    let endpoint = |index: usize| -> Result<String> {
        let occurrence = occurrences
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("cluster has no occurrence {index}"))?;
        let field = |name: &str| {
            occurrence
                .get(name)
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("occurrence has no `{name}`"))
        };
        let path = occurrence
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("occurrence has no `path`"))?;
        Ok(format!(
            "{path}:{}:{}",
            field("start_byte")?,
            field("end_byte")?
        ))
    };

    let output = scan_root.join("report");
    let mut command = deslop_cmd(&scan_root, &output)?;
    let assertion = command
        .args([
            "--min-nodes",
            &MIN_NODES.to_string(),
            "--embeddings",
            "off",
            "--compare",
            &endpoint(0)?,
            "--compare",
            &endpoint(1)?,
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assertion.get_output().stdout.clone())?;
    let comparison: Value = serde_json::from_str(stdout.trim()).map_err(|error| {
        anyhow::anyhow!(
            "`--compare` must print one JSON pair verdict on stdout: {error}. Got: {stdout}"
        )
    })?;

    for endpoint in ["left", "right"] {
        assert!(
            comparison
                .get(endpoint)
                .and_then(|side| side.get("path"))
                .is_some(),
            "the verdict must echo the `{endpoint}` endpoint it was asked about, so a caller \
             can never mistake which pair was measured: {comparison}"
        );
    }
    let evidence = comparison
        .get("evidence")
        .ok_or_else(|| anyhow::anyhow!("the verdict must carry `evidence`: {comparison}"))?;
    for field in EVIDENCE_FIELDS {
        assert!(
            evidence.get(field).is_some(),
            "pair evidence must carry `{field}` — the corpus gate reads exactly these to tell \
             an admitted duplicate from one merely reported: {evidence}"
        );
    }
    assert_eq!(
        evidence.get("admitted").and_then(Value::as_bool),
        Some(true),
        "two byte-identical carriers of the same function are an admitted pair; anything else \
         means the engine did not admit its own duplicate: {evidence}"
    );
    assert!(
        comparison
            .get("evidence")
            .and_then(|e| e.get("structural"))
            .and_then(Value::as_f64)
            > Some(0.0),
        "an admitted pair must carry measured structural evidence, not a default: {evidence}"
    );
    Ok(())
}
