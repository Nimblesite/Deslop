//! GH #66: endpoint mappings with different identifiers and literals
//! must not be classified as identical code.

use std::{fs, path::Path};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::signals::{assert_structural_only_contract, has_verbatim_pair};
use crate::common::*;

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(scan_root, &output)?
        .args(["--min-nodes", "30", "--embeddings", "off"])
        .assert()
        .success();
    let mut json_path = output;
    let _replaced = json_path.set_extension("json");
    Ok(serde_json::from_str(&fs::read_to_string(json_path)?)?)
}

#[test]
fn issue_66_route_mappings_with_value_differences_are_not_identical() -> Result<()> {
    let report = run_report(&fixture("csharp-issue-66-route-mapping"))?;
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("report must contain clusters: {report}"))?;
    assert!(
        !clusters.is_empty(),
        "route-mapping fixture must produce at least one cluster: {report}"
    );

    let cluster = clusters
        .iter()
        .find(|cluster| cluster_has_path(cluster, "ChatEndpoints.cs"))
        .filter(|cluster| cluster_has_path(cluster, "SiteEndpoints.cs"))
        .ok_or_else(|| anyhow!("expected cross-endpoint route mapping cluster: {report}"))?;
    let occurrences = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("target cluster must contain occurrences: {cluster}"))?;
    assert!(
        occurrences.len() >= 2,
        "target cluster must include both endpoint mappings: {cluster}"
    );
    // The bucket label and the interpretation sentence are gone from the
    // mass-only wire; what proves the mappings are NOT identical is the
    // byte truth: their occurrences differ in raw bytes (different
    // identifiers and literals), so the pair is a rename, never a copy
    // ([PIPELINE-CLUSTER-CLOSURE]).
    let scan_root = fixture("csharp-issue-66-route-mapping");
    assert!(
        !has_verbatim_pair(&scan_root, cluster)?,
        "value-different endpoint mappings must not be byte-identical: {cluster}"
    );
    assert_structural_only_contract(cluster, "issue #66 route mapping");
    Ok(())
}

fn cluster_has_path(cluster: &Value, file_name: &str) -> bool {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .is_some_and(|occurrences| {
            occurrences.iter().any(|occurrence| {
                occurrence
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with(file_name))
            })
        })
}
