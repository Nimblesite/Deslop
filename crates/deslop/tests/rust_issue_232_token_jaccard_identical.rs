//! E2E regression for GH #232.
//!
//! `token_jaccard` was reported as `0.00` for clusters in the `identical`
//! bucket whose occurrences are byte-for-byte identical, while
//! `nearly_identical` clusters correctly reported `1.00`. The signal was
//! zeroed on any synthetic **sibling-window** fingerprint: its byte range
//! matches no single AST node, so the non-language signature path
//! (`token_stream_for_fingerprint` → `locate`) failed to resolve it and
//! fell back to a byte-offset-seeded signature that differs between two
//! files even when the code is identical, driving the estimated Jaccard
//! to ~0.
//!
//! The fixture's `render_header` + `render_footer` block is byte-identical
//! across `alpha.rs` and `beta.rs`; the surrounding lead/tail functions
//! differ structurally so the reported clone is exactly that window.
//!
//! Acceptance: a byte-equivalent (`identical`-bucket) clone MUST report
//! `token_jaccard ≈ 1.0`. The Jaccard of two identical token multisets is
//! 1.0 by definition, so any `identical` cluster reporting `< 0.9` is the
//! GH #232 regression.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::Value;

use crate::common::deslop_cmd;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(scan_root, &output)?
        .args(["--min-nodes", "30", "--embeddings", "off"])
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
fn identical_bucket_byte_clones_report_token_jaccard_near_one() -> Result<()> {
    let scan_root = fixture("rust-issue-232-token-jaccard");
    let report = run_report(&scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let identical: Vec<&Value> = clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "identical")
        .collect();
    assert!(
        !identical.is_empty(),
        "fixture must produce the byte-identical clone in the `identical` \
         bucket so the token_jaccard signal is exercised: {report:#}"
    );

    let zeroed: Vec<String> = identical
        .iter()
        .filter(|cluster| signal(cluster, "token_jaccard") < 0.9)
        .map(|cluster| {
            format!(
                "cluster {id} bucket=identical signals={{structural={s:.2}, \
                 token_jaccard={t:.2}, embedding_cos={e:.2}}} \
                 canonical_node_count={nodes}",
                id = cluster.get("id").and_then(Value::as_str).unwrap_or("?"),
                s = signal(cluster, "structural"),
                t = signal(cluster, "token_jaccard"),
                e = signal(cluster, "embedding_cos"),
                nodes = cluster
                    .get("canonical_node_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            )
        })
        .collect();
    assert!(
        zeroed.is_empty(),
        "issue #232: byte-identical (`identical`-bucket) clones must report \
         token_jaccard ≈ 1.0, not the 0.0 sibling-window fallback. The \
         Jaccard of two identical token multisets is 1.0 by definition. \
         Offending clusters: {zeroed:#?}"
    );
    Ok(())
}
