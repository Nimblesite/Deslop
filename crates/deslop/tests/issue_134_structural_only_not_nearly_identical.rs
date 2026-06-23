//! Issue #134: a cluster whose only positive signal is the structural
//! fingerprint (`structural >= 0.99`) but whose `token_jaccard` and
//! `embedding_cos` are essentially zero must NOT be labeled
//! `nearly_identical`. Without supporting token or semantic evidence,
//! agents have no way to tell a real Type-2 clone from a structural
//! skeleton match (test scaffolding, generated boilerplate). Bucketing
//! these as `nearly_identical` makes top-offenders fill with
//! low-actionability results, which is the exact regression #134
//! reproduces.
//!
//! Acceptance: no cluster in the rendered report carries
//! `bucket=nearly_identical` together with `structural >= 0.99`,
//! `token_jaccard < 0.05`, and `embedding_cos < 0.05`.

use std::{fs, path::Path};

use anyhow::Result;

mod common;
use crate::common::*;

fn run_report(tmp: &Path, scan_root: &Path) -> Result<serde_json::Value> {
    let mut cmd = deslop_cmd(scan_root, &tmp.join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "30", "--embeddings", "off"])
        .assert()
        .success();
    let mut json_path = tmp.join("report");
    let _replaced = json_path.set_extension("json");
    let body = fs::read_to_string(&json_path)?;
    Ok(serde_json::from_str(&body)?)
}

fn signal(cluster: &serde_json::Value, key: &str) -> f64 {
    cluster
        .get("signals")
        .and_then(|signals| signals.get(key))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or_default()
}

fn cluster_bucket(cluster: &serde_json::Value) -> &str {
    cluster
        .get("bucket")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?")
}

fn cluster_id(cluster: &serde_json::Value) -> &str {
    cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?")
}

#[test]
fn issue_134_structural_only_clusters_are_not_nearly_identical() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-issue-134-structural-only");
    let report = run_report(tmp.path(), &scan_root)?;
    // [METRICS-REPO] `clusters_total` counts only the clusters the report
    // renders. This fixture's sole family is a hidden structural-only
    // cluster, so it contributes zero to the metric while still being
    // detected (`clusters_hidden`) — a structural-only shape match cannot
    // inflate the percentage.
    let visible_contributing = report
        .pointer("/metrics/clusters_total")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        visible_contributing, 0,
        "hidden structural-only cluster must not count as a visible \
         contributing cluster: {report}"
    );
    let clusters_hidden = report
        .get("clusters_hidden")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    assert!(
        clusters_hidden >= 1,
        "fixture must produce at least one hidden structural-only cluster so \
         the bucketing rule is actually exercised: {report}"
    );
    let duplication_percent = report
        .pointer("/metrics/duplication_percent")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(-1.0);
    assert!(
        (0.0..=0.0001).contains(&duplication_percent),
        "structural-only shape matches must not influence duplication_percent: \
         got {duplication_percent}"
    );
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "nearly_identical")
        .filter(|cluster| {
            signal(cluster, "structural") >= 0.99
                && signal(cluster, "token_jaccard") < 0.05
                && signal(cluster, "embedding_cos") < 0.05
        })
        .map(|cluster| {
            format!(
                "cluster {} signals={{structural={:.2}, token_jaccard={:.2}, \
                 embedding_cos={:.2}}}",
                cluster_id(cluster),
                signal(cluster, "structural"),
                signal(cluster, "token_jaccard"),
                signal(cluster, "embedding_cos"),
            )
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "issue #134: structural-only clusters (token_jaccard < 0.05 and \
         embedding_cos < 0.05) must not be labeled `nearly_identical`. \
         Offending clusters: {offenders:#?}"
    );
    Ok(())
}
