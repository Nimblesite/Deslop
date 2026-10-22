//! [MCP-TOOL-FILTERS-KINDS] Isolates selection at the formerly quarantined matcher.

use super::{matches_filters, DuplicatesFilters, ReportCluster};
use serde_json::json;

const NEARLY_IDENTICAL: &str = "nearly_identical";
const IDENTICAL: &str = "identical";
const WARNING: &str = "warning";
const LANGUAGE: &str = "csharp";
const FIRST_PATH: &str = "Alpha.cs";
const SECOND_PATH: &str = "Beta.cs";
const CLUSTER_ID: &str = "54a6b248b64f425d";
const NODE_COUNT: usize = 58;
const COPY_COUNT: usize = 2;
const FIRST_RANK: usize = 1;
const START_BYTE: usize = 0;
const END_BYTE: usize = 383;
const START_LINE: usize = 1;
const END_LINE: usize = 19;

/// The canonical wire cluster, with both of its members retained.
fn cluster() -> serde_json::Result<ReportCluster> {
    let occurrence = |path: &str| {
        json!({"path": path, "start_byte": START_BYTE, "end_byte": END_BYTE,
        "start_line": START_LINE, "end_line": END_LINE, "hidden": false})
    };
    serde_json::from_value(json!({
        "id": CLUSTER_ID, "rank": FIRST_RANK, "kind": NEARLY_IDENTICAL, "severity": WARNING,
        "mass": NODE_COUNT, "canonical_node_count": NODE_COUNT, "occurrence_count": COPY_COUNT,
        "occurrences_total": COPY_COUNT, "occurrences_truncated": false,
        "occurrences": [occurrence(FIRST_PATH), occurrence(SECOND_PATH)],
    }))
}

/// Each selected kind combines with the other shared filters.
fn filters(kind: &str) -> serde_json::Result<DuplicatesFilters> {
    serde_json::from_value(json!({
        "kinds": [kind], "languages": [LANGUAGE], "severities": [WARNING],
        "min_size": NODE_COUNT, "path_contains": SECOND_PATH,
    }))
}

#[test]
fn selected_kind_and_occurrence_filters_retain_the_ranked_pair() -> serde_json::Result<()> {
    let cluster = cluster()?;
    assert!(matches_filters(&filters(NEARLY_IDENTICAL)?, &cluster));
    assert!(!matches_filters(&filters(IDENTICAL)?, &cluster));
    assert_eq!(cluster.id, CLUSTER_ID);
    assert_eq!(cluster.rank, FIRST_RANK);
    assert_eq!(cluster.occurrence_count, COPY_COUNT);
    assert_eq!(cluster.occurrences.len(), COPY_COUNT);
    Ok(())
}
