//! E2E regression for GH #147: `xs.iter().map(|x| x.field.as_str()).collect()`
//! is a pure Rust language idiom that clusters across unrelated element
//! types. Extracting it would require a trait on 10+ unrelated structs.
//! The cluster must not surface as actionable duplication.
//! Spec: [CLONE-NOISE-RUST-ITER-COLLECT], [PIPELINE-FINGERPRINT-MERKLE-TYPE-REFERENCE].

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// The node floor the iterator-chain idiom is judged at.
const ITER_COLLECT_MIN_NODES: u32 = 4;

const CONTROL_FIXTURE: &str = "rust-small";
const CONTROL_FILES: &[&str] = &["alpha.rs", "beta.rs"];
const CONTROL_KIND: &str = "nearly_identical";
const CONTROL_CLUSTER_COUNT: usize = 1;
const CONTROL_OCCURRENCE_COUNT: u64 = 2;
const CONTROL_RANK: u64 = 1;
const CONTROL_NODES: u64 = 39;
const CONTROL_START: u64 = 1;
const CONTROL_END: u64 = 10;

fn cluster_occurrence_paths(cluster: &Value) -> Vec<String> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(|occurrence| {
                    occurrence
                        .get("path")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
}

#[test]
fn rust_iter_map_collect_idiom_does_not_cluster_across_unrelated_types() -> Result<()> {
    let report = run_report(&fixture("rust-issue-147-iter-collect-idiom"), ITER_COLLECT_MIN_NODES)?;
    let cross_file_clusters: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| {
            let paths = cluster_occurrence_paths(cluster);
            let distinct: std::collections::BTreeSet<&String> = paths.iter().collect();
            distinct.len() >= 2
        })
        .collect();
    assert!(
        cross_file_clusters.is_empty(),
        "the `.iter().map(|x| x.field.method()).collect()` idiom must not \
         cluster across unrelated element types: {report:#}"
    );
    assert_real_rust_clone()?;
    Ok(())
}

fn assert_real_rust_clone() -> Result<()> {
    let report = run_report(&fixture(CONTROL_FIXTURE), ITER_COLLECT_MIN_NODES)?;
    let found = clusters(&report);
    assert_eq!(found.len(), CONTROL_CLUSTER_COUNT, "{report:#}");
    let cluster = found
        .first()
        .ok_or_else(|| anyhow::anyhow!("missing control clone"))?;
    assert_control_identity(cluster);
    assert_control_occurrences(cluster);
    Ok(())
}

fn assert_control_identity(cluster: &Value) {
    assert_eq!(cluster["kind"].as_str(), Some(CONTROL_KIND));
    assert_eq!(cluster["rank"].as_u64(), Some(CONTROL_RANK));
    assert_eq!(
        cluster["canonical_node_count"].as_u64(),
        Some(CONTROL_NODES)
    );
    assert_eq!(
        cluster["occurrence_count"].as_u64(),
        Some(CONTROL_OCCURRENCE_COUNT)
    );
    assert_eq!(cluster_occurrence_paths(cluster), CONTROL_FILES);
}

fn assert_control_occurrences(cluster: &Value) {
    for occurrence in occurrences(cluster) {
        assert_eq!(occurrence["start_line"].as_u64(), Some(CONTROL_START));
        assert_eq!(occurrence["end_line"].as_u64(), Some(CONTROL_END));
        assert_eq!(occurrence["hidden"].as_bool(), Some(false));
    }
}
