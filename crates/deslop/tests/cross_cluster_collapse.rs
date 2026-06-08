//! End-to-end regression coverage for issue #50: nested fingerprints
//! over the same physical code produce two distinct fused clusters
//! whose occurrence byte ranges fully overlap inside the same files.
//! `collapse_overlapping_per_file` deduplicates *within* a single
//! cluster; without a cross-cluster pass, the `[Fact] + method`
//! subtree and the bare `method` subtree (one line below the
//! attribute) survive as siblings and the user sees the same
//! dozen-occurrence clone twice with different cluster ids.
//!
//! Spec: [PIPELINE-CLUSTER-EXACT] commits to one canonical cluster
//! per duplicated region.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use assert_cmd::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn report_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    path
}

fn run_report(tmp: &Path, scan_root: &Path) -> Result<serde_json::Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(tmp.join("report"))
        .assert()
        .success();
    let body = fs::read_to_string(report_path(tmp))?;
    Ok(serde_json::from_str(&body)?)
}

#[derive(Clone, Debug)]
struct Occurrence {
    path: String,
    start: u64,
    end: u64,
}

fn cluster_occurrences(cluster: &serde_json::Value) -> Vec<Occurrence> {
    cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .map(|occurrences| {
            occurrences
                .iter()
                .filter_map(|occurrence| {
                    Some(Occurrence {
                        path: occurrence.get("path")?.as_str()?.to_owned(),
                        start: occurrence.get("start_byte")?.as_u64()?,
                        end: occurrence.get("end_byte")?.as_u64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn cluster_id(cluster: &serde_json::Value) -> String {
    cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?")
        .to_owned()
}

fn ranges_overlap(left: &Occurrence, right: &Occurrence) -> bool {
    left.path == right.path && left.start < right.end && right.start < left.end
}

fn every_occurrence_overlaps_some(inner: &[Occurrence], outer: &[Occurrence]) -> bool {
    !inner.is_empty()
        && inner
            .iter()
            .all(|candidate| outer.iter().any(|other| ranges_overlap(candidate, other)))
}

fn first_subsumed_pair(report: &serde_json::Value) -> Option<String> {
    let clusters = report.get("clusters")?.as_array()?;
    let occurrence_sets: Vec<(String, Vec<Occurrence>)> = clusters
        .iter()
        .map(|cluster| (cluster_id(cluster), cluster_occurrences(cluster)))
        .collect();
    for (outer_index, (outer_id, outer)) in occurrence_sets.iter().enumerate() {
        for (inner_id, inner) in occurrence_sets.iter().skip(outer_index.saturating_add(1)) {
            if every_occurrence_overlaps_some(inner, outer)
                && every_occurrence_overlaps_some(outer, inner)
            {
                return Some(format!(
                    "clusters {outer_id} and {inner_id} cover the same physical \
                     bytes — every occurrence in one overlaps with some \
                     occurrence in the other"
                ));
            }
        }
    }
    None
}

fn clusters_for_file(report: &serde_json::Value, needle: &str) -> Vec<serde_json::Value> {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter(|cluster| {
                    cluster_occurrences(cluster)
                        .iter()
                        .any(|occurrence| occurrence.path == needle)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

// Issue #50 acceptance: a small C# file with two [Fact]-decorated
// near-identical test methods must produce exactly one cluster covering
// the test-method region. Pre-fix, the `attribute_list +
// method_declaration` subtree and the bare `method_declaration`
// subtree each form a separate fused cluster, so the user sees the
// same occurrences reported twice. The fixture is a two-method pair so
// the cluster stays visible: a three-or-more sibling-method family is a
// single-file `structural_only` pattern suppressed by #197.
#[test]
fn fact_decorated_identical_methods_produce_one_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-fact-cross-cluster");
    let report = run_report(tmp.path(), &scan_root)?;
    let candidates = clusters_for_file(&report, "CodeLookupTests.cs");
    assert!(
        !candidates.is_empty(),
        "fixture must produce at least one clone cluster covering the test \
         methods: {report:#}"
    );
    assert!(
        candidates.len() <= 3,
        "nested-fingerprint clusters must collapse: expected at most 3 clusters \
         (method body, attribute, possible sibling window) covering the test \
         methods, got {} (was 25 before the fix): ids = {:?}",
        candidates.len(),
        candidates.iter().map(cluster_id).collect::<Vec<_>>(),
    );
    Ok(())
}

// Issue #50 invariant: no two clusters may have mutually-subsuming
// occurrence sets. If every occurrence in cluster B overlaps some
// occurrence in cluster A *and* vice versa, they describe the same
// physical bytes at different AST depths and must collapse to one.
#[test]
fn no_two_clusters_cover_the_same_physical_bytes() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-fact-cross-cluster");
    let report = run_report(tmp.path(), &scan_root)?;
    assert!(
        first_subsumed_pair(&report).is_none(),
        "cross-cluster overlap collapse missing: {}",
        first_subsumed_pair(&report).unwrap_or_default()
    );
    Ok(())
}
