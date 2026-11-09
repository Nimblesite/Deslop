//! `duplicates`-page shape helpers shared by the CLI suites: the page
//! body accessors, the occurrence budget accounting, and the
//! [PIPELINE-CLUSTER-CLOSURE] key-omission assertions.

use super::{value_array, value_get};
use anyhow::Result;
use serde_json::{json, Value};

use super::mcp_wire::{BUCKET_FIELD, CLUSTERS_POINTER, OCCURRENCES_FIELD, TOTAL_CLUSTERS_POINTER};

/// Per-cluster keys the mass-only wire never carries on a full cluster
/// ([PIPELINE-CLUSTER-CLOSURE]).
pub const RETIRED_CLUSTER_KEYS: [&str; 5] = [
    BUCKET_FIELD,
    "interpretation",
    "weight",
    "signals",
    "verdict",
];
/// The same closure for a `detail=summary` row, which additionally
/// never carries the retired pair `score`.
pub const RETIRED_CLUSTER_SUMMARY_KEYS: [&str; 5] =
    [BUCKET_FIELD, "score", "weight", "signals", "verdict"];
/// Why a retired key is a defect wherever it appears.
pub const RETIRED_KEY_REASON: &str = "mass-only wire must not carry a per-cluster label";
/// Why a deep-dive key is a defect on a summary row.
pub const DEEP_DIVE_KEY_REASON: &str =
    "ClusterSummary must leave this to the cluster-by-id deep dive";
/// Keys a `detail=summary` row drops because they live behind the
/// `cluster-by-id` deep dive rather than on every page row.
pub const SUMMARY_DEEP_DIVE_KEYS: [&str; 2] = ["members", OCCURRENCES_FIELD];

/// Asserts a report page is empty: zero total clusters and an empty
/// `clusters[]` array.
pub fn assert_empty_page(page: &Value) -> Result<()> {
    assert_eq!(value_get(page, TOTAL_CLUSTERS_POINTER)?, json!(0));
    assert!(value_get(page, CLUSTERS_POINTER)?
        .as_array()
        .is_some_and(Vec::is_empty));
    Ok(())
}

/// The `clusters[]` array of a duplicates page. [MCP-TOOLS] every page
/// carries this key, so its absence is a wire-contract failure.
pub fn page_clusters(payload: &Value) -> Result<Vec<Value>> {
    value_array(payload, CLUSTERS_POINTER)
}

/// Total occurrences shipped across `clusters` — the quantity
/// [MCP-OCCURRENCE-BUDGET] caps.
pub fn shipped_occurrences(clusters: &[Value]) -> u64 {
    clusters
        .iter()
        .map(|cluster| {
            cluster
                .get(OCCURRENCES_FIELD)
                .and_then(Value::as_array)
                .map_or(0, |occurrences| occurrences.len() as u64)
        })
        .sum()
}

/// Asserts no cluster in `clusters` carries any key in `omitted`,
/// reporting `reason` when one does.
/// [PIPELINE-CLUSTER-CLOSURE]: a published cluster owns identity,
/// extent, membership, mass and mass-derived rank — nothing that could
/// mislabel it on any surface.
pub fn assert_clusters_omit(clusters: &[Value], omitted: &[&str], reason: &str) {
    for key in omitted {
        assert!(
            clusters.iter().all(|cluster| cluster.get(key).is_none()),
            "{reason}: carried {key}: {clusters:#?}"
        );
    }
}
