//! End-to-end regression coverage for issue #44: unrelated C# xUnit
//! test classes get bucketed as `Nearly identical code` because the
//! third disjunct of [`buckets::classify_signals`] paints LSH-only
//! pairs (`structural <= 0.01 && token_jaccard >= 0.90`) as Type-3
//! near-misses. C#'s grammar saturates the kind-gram alphabet on
//! xUnit scaffolding (`using_directive`, `attribute_list`,
//! `method_declaration`, `await_expression`, `__ident__`,
//! `__literal__`, …), so two completely unrelated test classes reach
//! kind-gram Jaccard ≈ 1.0 with zero structural overlap.
//!
//! Acceptance from the issue: a fixture containing 3+ unrelated C#
//! xUnit test classes must produce **zero** `nearly_identical`
//! cross-class clusters. Type-3 should require some structural
//! anchor; LSH-only matches with `structural ≈ 0` belong in
//! `loosely_similar`, not `nearly_identical`.

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result};

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

fn cluster_paths(cluster: &serde_json::Value) -> BTreeSet<String> {
    cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .map(|occurrences| {
            occurrences
                .iter()
                .filter_map(|occurrence| {
                    occurrence
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn occurrence_slices(cluster: &serde_json::Value, scan_root: &Path) -> Result<Vec<Vec<u8>>> {
    let Some(occurrences) = cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    occurrences
        .iter()
        .map(|occurrence| occurrence_slice(occurrence, scan_root))
        .collect()
}

fn occurrence_slice(occurrence: &serde_json::Value, scan_root: &Path) -> Result<Vec<u8>> {
    let path = occurrence_text(occurrence, "path")?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    let source = fs::read(scan_root.join(path))?;
    Ok(source.get(start..end).context("occurrence range")?.to_vec())
}

fn occurrence_text<'a>(occurrence: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    occurrence
        .get(key)
        .and_then(serde_json::Value::as_str)
        .with_context(|| format!("missing occurrence {key}"))
}

fn occurrence_byte(occurrence: &serde_json::Value, key: &str) -> Result<usize> {
    let value = occurrence
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .with_context(|| format!("missing occurrence {key}"))?;
    usize::try_from(value).with_context(|| format!("occurrence {key} too large"))
}

fn non_identical_source_slices(slices: &[Vec<u8>]) -> bool {
    slices
        .split_first()
        .is_some_and(|(first, rest)| rest.iter().any(|slice| slice != first))
}

fn identical_clusters_with_different_source(
    report: &serde_json::Value,
    scan_root: &Path,
) -> Result<Vec<String>> {
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut offenders = Vec::new();
    for cluster in clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "identical")
    {
        let slices = occurrence_slices(cluster, scan_root)?;
        if non_identical_source_slices(&slices) {
            offenders.push(format!(
                "cluster {} spans {:?}",
                cluster_id(cluster),
                cluster_paths(cluster)
            ));
        }
    }
    Ok(offenders)
}

// Issue #44 acceptance: unrelated C# xUnit test classes must not
// merge into a single "Nearly identical code" cluster. Three
// completely unrelated test files share only generic xUnit
// scaffolding kinds; LSH-only kind-gram Jaccard saturation must not
// route the resulting pair into the [CLONE-BUCKETS] `NearlyIdentical`
// bucket. LooselySimilar is suppressed entirely in the ranked output
// per issue #58, so neither bucket must appear for boilerplate-only
// matches.
#[test]
fn unrelated_csharp_xunit_classes_are_never_nearly_identical() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-unrelated-xunit-tests");
    let report = run_report(tmp.path(), &scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "nearly_identical")
        .filter(|cluster| cluster_paths(cluster).len() >= 2)
        .map(|cluster| {
            let id = cluster_id(cluster);
            let paths: Vec<String> = cluster_paths(cluster).into_iter().collect();
            format!("cluster {id} spans {paths:?}")
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "unrelated C# xUnit test classes must not form a 'Nearly identical' \
         cross-class cluster (issue #44). Offending clusters: {offenders:#?}"
    );
    Ok(())
}

// [CLONE-BUCKETS] Issue #64: assertion blocks with different literal values
// currently normalise to the same C# AST shape and get labelled as `Identical
// code`. A user-facing identical bucket must only contain byte-identical slices.
#[test]
fn csharp_assertion_blocks_with_different_literals_are_not_identical() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-unrelated-xunit-tests");
    let report = run_report(tmp.path(), &scan_root)?;
    let offenders = identical_clusters_with_different_source(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "C# assertion blocks with different literals must not be labelled \
         identical (issue #64). Offending clusters: {offenders:#?}"
    );
    Ok(())
}
