//! E2E regression for GH #154.
//!
//! `top-offenders` was surfacing `structural_only` clusters
//! (`structural=1.00, token_jaccard=0.00, embedding_cos=0.00`) that
//! matched function signatures whose bodies were entirely unrelated.
//! The pattern is unavoidable in any codebase that uses shared-context
//! structs (`fn check_*(ctx: &mut Ctx)`), so refactoring the source to
//! silence deslop would hurt readability. The pipeline must drop these
//! evidence-free clusters from the ranked report.
//!
//! Acceptance: the rendered report MUST NOT contain any cluster with
//! `bucket="structural_only"` AND `token_jaccard < 0.1`. By construction
//! every `structural_only` cluster the renderer emits already satisfies
//! the second condition, so this assertion really demands that the
//! whole bucket be filtered from `clusters`.

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(scan_root, &output)?
        .args(["--min-nodes", "4", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn cluster_bucket(cluster: &Value) -> &str {
    cluster.get("bucket").and_then(Value::as_str).unwrap_or("?")
}

fn signal(cluster: &Value, key: &str) -> f64 {
    cluster
        .pointer(&format!("/signals/{key}"))
        .and_then(Value::as_f64)
        .unwrap_or_default()
}

#[test]
fn structural_only_signature_clusters_are_dropped_from_the_report() -> Result<()> {
    let scan_root = fixture("rust-issue-154-structural-only");
    let report = run_report(&scan_root)?;
    let total_clusters = report
        .pointer("/metrics/clusters_total")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        total_clusters >= 1,
        "fixture must produce at least one cluster (visible or hidden) so \
         the structural_only filter is actually exercised: {report:#}"
    );
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "structural_only")
        .filter(|cluster| signal(cluster, "token_jaccard") < 0.1)
        .map(|cluster| {
            format!(
                "cluster {id} signals={{structural={s:.2}, token_jaccard={t:.2}, \
                 embedding_cos={e:.2}}} size={size}",
                id = cluster.get("id").and_then(Value::as_str).unwrap_or("?"),
                s = signal(cluster, "structural"),
                t = signal(cluster, "token_jaccard"),
                e = signal(cluster, "embedding_cos"),
                size = cluster.get("size").and_then(Value::as_u64).unwrap_or(0),
            )
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "issue #154: structural_only clusters with token_jaccard < 0.1 \
         have no real evidence and must not appear in the ranked report. \
         Offending clusters: {offenders:#?}"
    );
    // [METRICS-REPO] This fixture mixes visible near-miss clones with hidden
    // structural_only families. The metric must count only the lines the
    // visible clusters cover, so the suppressed families add nothing.
    let reported = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        reported,
        visible_duplicated_loc(&report),
        "duplicated_loc must equal the visible-cluster line union — hidden \
         structural_only families must not inflate the metric: {report:#}"
    );
    let hidden = report
        .get("clusters_hidden")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        hidden >= 1,
        "the fixture's structural_only clusters must be suppressed via \
         the hidden-cluster path so they still count toward visibility \
         telemetry: clusters_hidden={hidden}, report={report:#}"
    );
    Ok(())
}
