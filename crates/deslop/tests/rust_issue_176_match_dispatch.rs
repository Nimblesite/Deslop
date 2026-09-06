//! E2E regression for GH #176: runs of `match` arms of the shape
//! `PATH::IDENT => Ok(handler(args))` collapse to one structure under
//! Type-2 normalisation, so the sibling-window pass matches one window of
//! arms against another window of arms within the *same* dispatch `match`.
//! These are routing tables — each arm maps a distinct command to a
//! distinct handler — and are not extractable duplication. The cluster
//! must never surface in the report.
//! Spec: [CLONE-NOISE-RUST-MATCH-DISPATCH].

use std::fs;

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

fn run_report(fixture_name: &str) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(&fixture(fixture_name), &output)?
        .args(["--min-nodes", "3", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn cluster_occurrence_paths(cluster: &Value) -> std::collections::BTreeSet<String> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| {
            occurrence
                .get("path")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn rust_match_dispatch_arms_do_not_cluster_as_duplicates() -> Result<()> {
    let report = run_report("rust-issue-176-match-dispatch")?;
    let count = cluster_count(&report);
    assert_eq!(
        count, 0,
        "a `match` dispatch table routes distinct commands to distinct \
         handlers and must not surface as duplicate clusters: {report:#}"
    );
    Ok(())
}

#[test]
fn rust_verbatim_copied_match_arms_still_cluster() -> Result<()> {
    let report = run_report("rust-issue-176-verbatim-copy")?;
    let cross_file = clusters(&report)
        .iter()
        .any(|cluster| cluster_occurrence_paths(cluster).len() >= 2);
    assert!(
        cross_file,
        "a run of `match` arms copy-pasted verbatim across two files is \
         genuine duplication and must still surface as a cluster: {report:#}"
    );
    Ok(())
}
