//! Regression tests for BUG #61 and BUG #62: literal-keyed dict entries
//! falsely reported as identical clones.
//!
//! [PIPELINE-FINGERPRINT-MERKLE] normalises every integer or string literal
//! to `__literal__`, so every `int: str` entry in a dict becomes the same
//! normalised subtree. The sibling-window fingerprinter then emits hashes
//! over consecutive windows of these identical subtrees. Because every
//! window of the same width hashes identically, the clusterer groups them
//! as clones — even though they are distinct entries inside a single dict.
//!
//! BUG #61: uniform sibling windows (all children have the same subtree
//! hash) must not be fingerprinted — any pair of same-width windows over
//! a uniform sequence is trivially identical and not a real clone.
//!
//! BUG #62: a secondary guard — two-member clusters where both occurrences
//! sit in the same file with non-overlapping but adjacent byte ranges
//! inside the same parent node must be suppressed.

use std::fs;

use anyhow::Result;

mod common;
use crate::common::*;

fn run_cli_on_fixture(fixture_name: &str) -> Result<serde_json::Value> {
    let tmp = tempfile::tempdir()?;
    let report_base = tmp.path().join("report");
    let mut cmd = deslop_cmd(&fixture(fixture_name), &report_base)?;
    let _assertion = cmd.args(["--min-nodes", "4"]).assert().success();
    let json_path = {
        let mut p = report_base.clone();
        let mut name = p
            .file_name()
            .map(std::ffi::OsStr::to_os_string)
            .unwrap_or_default();
        name.push(".json");
        p.set_file_name(name);
        p
    };
    let body = fs::read_to_string(&json_path)?;
    Ok(serde_json::from_str(&body)?)
}

fn clusters(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .map(|v| v.iter().collect())
        .unwrap_or_default()
}

fn intra_file_clusters(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    clusters(report)
        .into_iter()
        .filter(|cluster| {
            let occurrences = cluster
                .pointer("/occurrences")
                .and_then(serde_json::Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let paths: std::collections::BTreeSet<&str> = occurrences
                .iter()
                .filter_map(|o| o.get("path").and_then(serde_json::Value::as_str))
                .collect();
            paths.len() == 1
        })
        .collect()
}

// [PIPELINE-FINGERPRINT-MERKLE] BUG #61 — A single Python file containing
// a dict with many literal int→str entries must produce zero clone clusters.
// Before the fix, uniform sibling windows (all `__literal__: __literal__`
// pairs) hash identically and create spurious cross-window matches.
#[test]
fn single_file_literal_dict_produces_no_clone_clusters() -> Result<()> {
    let report = run_cli_on_fixture("python-dict-false-positive")?;
    let found_clusters = clusters(&report);
    assert!(
        found_clusters.is_empty(),
        "literal-keyed dict entries must not produce clone clusters — \
         every entry is distinct even though normalization collapses \
         int/str literals to __literal__. Got {} clusters: {:#?}",
        found_clusters.len(),
        found_clusters
            .iter()
            .map(|c| serde_json::to_string_pretty(c).unwrap_or_default())
            .collect::<Vec<_>>()
    );
    Ok(())
}

// [PIPELINE-FINGERPRINT-MERKLE] BUG #62 — When a two-member cluster has
// both occurrences in the same file with adjacent (non-overlapping but
// contiguous) byte ranges, it is a false positive and must be suppressed.
// This is a secondary guard independent of the #61 fix.
#[test]
fn intra_file_adjacent_occurrences_not_reported_as_clones() -> Result<()> {
    let report = run_cli_on_fixture("python-dict-false-positive")?;
    let bad_clusters = intra_file_clusters(&report);
    assert!(
        bad_clusters.is_empty(),
        "clusters with both occurrences in the same file must be suppressed — \
         adjacent dict entries are not real duplicates. Got {} intra-file clusters: {:#?}",
        bad_clusters.len(),
        bad_clusters
            .iter()
            .map(|c| serde_json::to_string_pretty(c).unwrap_or_default())
            .collect::<Vec<_>>()
    );
    Ok(())
}
