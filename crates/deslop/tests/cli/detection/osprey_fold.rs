//! [CLONE-KIND-FOLD] An edited relative cannot hide renamed copies of the whole function.

use super::*;

const FIXTURE: &str = "osprey-fold-subset";
const DEFAULT_MIN_NODES: &str = "30";
const EDITED_FILE: &str = "inventory.osp";
const RENAMED_FILE: &str = "ledger.osp";
const SIMILAR_KIND: &str = "loosely_similar";
const NEARLY_IDENTICAL_KIND: &str = "nearly_identical";
const CLONE_CLUSTERS: usize = 2;
const SIMILAR_RANK: u64 = 1;
const NEARLY_IDENTICAL_RANK: u64 = 2;
/// `unshelvedFrom`: the same walk, but it appends instead of rendering.
const EDITED_FUNCTION: (u64, u64) = (1, 13);
/// `renderOpen`, `renderOpenRefunds`, `renderOpenCredits`, signature to closing brace.
const RENAMED_FUNCTIONS: [(u64, u64); 3] = [(1, 13), (15, 27), (29, 41)];

// The three ledger walkers differ only in names: a Type-2 family. The
// inventory walker changed its statements, so the four together are
// Similar code — and the three must still be reported as Nearly identical
// over the whole function, not hidden in the weaker group or cut down to a
// fragment of their bodies.
#[test]
fn renamed_osprey_functions_stay_nearly_identical_beside_an_edited_relative() -> Result<()> {
    let json = run_min_nodes(FIXTURE, DEFAULT_MIN_NODES)?;
    let clusters = report_clusters(&json)?;
    let similar = kind_cluster(&clusters, SIMILAR_KIND)?;
    let renamed = kind_cluster(&clusters, NEARLY_IDENTICAL_KIND)?;
    assert_eq!(
        clusters.len(),
        CLONE_CLUSTERS,
        "one Similar family and one Nearly identical family: {json}"
    );
    assert_eq!(rows_of(&similar, EDITED_FILE), vec![EDITED_FUNCTION]);
    assert_eq!(rows_of(&similar, RENAMED_FILE), RENAMED_FUNCTIONS.to_vec());
    assert_eq!(field(&similar, "rank").as_u64(), Some(SIMILAR_RANK));
    assert!(
        rows_of(&renamed, EDITED_FILE).is_empty(),
        "the edited walker is no near-copy: {renamed:#}"
    );
    assert_eq!(
        rows_of(&renamed, RENAMED_FILE),
        RENAMED_FUNCTIONS.to_vec(),
        "the renamed walkers are near-copies from signature to closing brace: {json}"
    );
    assert_eq!(
        field(&renamed, "rank").as_u64(),
        Some(NEARLY_IDENTICAL_RANK)
    );
    Ok(())
}

/// The one cluster of `kind`.
fn kind_cluster(clusters: &[Value], kind: &str) -> Result<Value> {
    let mut matching = clusters
        .iter()
        .filter(|cluster| field(cluster, "kind").as_str() == Some(kind));
    let found = matching
        .next()
        .ok_or_else(|| anyhow::anyhow!("no {kind} cluster in {clusters:#?}"))?;
    anyhow::ensure!(matching.next().is_none(), "more than one {kind} cluster");
    Ok(found.clone())
}

/// `(start_line, end_line)` of each occurrence of `cluster` in `file`, in line order.
fn rows_of(cluster: &Value, file: &str) -> Vec<(u64, u64)> {
    let mut rows: Vec<(u64, u64)> = field(cluster, "occurrences")
        .as_array()
        .into_iter()
        .flatten()
        .filter(|occurrence| field(occurrence, "path").as_str() == Some(file))
        .map(|occurrence| {
            let row = |key| field(occurrence, key).as_u64().unwrap_or_default();
            (row("start_line"), row("end_line"))
        })
        .collect();
    rows.sort_unstable();
    rows
}
