//! [PIPELINE-DETERMINISM] A cluster id names exactly one finding.
//!
//! The id is what a reader, the editor state, and the MCP
//! `cluster-by-id` lookup all hold on to. `[PIPELINE-RANK-WORST-FIRST]`
//! also breaks ranking ties with it, "so the order is total and
//! reproducible" — which it is not while two clusters can share one.
//!
//! This is not hypothetical. `cluster_id_source` in
//! `crates/deslop-core/src/cluster.rs` returns the *smallest member
//! digest* and nothing else, so any two clusters whose members share a
//! normalised subtree carry the same id no matter which files they are
//! in. The #107 fixture holds three unrelated pytest modules, each
//! repeating `data["model_config"]` on two lines; all three publish as
//! separate clusters and all three are stamped `bc8ca6ce6565ba6d`.
//!
//! What that costs, in order of how quickly a user meets it:
//!
//! * `cluster-by-id` and the editor's "go to cluster" resolve to
//!   whichever of them is found first, so two thirds of the findings are
//!   unreachable through the surfaces built to reach them;
//! * the ranking tie-break stops being a total order, so two runs may
//!   sort the colliding clusters differently and the report is no longer
//!   reproducible;
//! * a reader counting distinct ids under-counts the findings.
//!
//! Tracked as gh #430. The id has to stay content-derived so it survives
//! a file move, and it has to distinguish two findings that merely share
//! a shape; mixing the members' workspace-relative paths into the digest
//! does both, and moves every committed id when it lands.

use std::collections::BTreeMap;

use crate::common::*;

/// The #107 fixture: three unrelated pytest modules that repeat one
/// subscript chain, plus the authored control clone.
const COLLIDING_FIXTURE: &str = "python-issue-107-chained-dict-assert";

/// `min-nodes` the #107 pin scans at — low enough to admit the
/// subscript-chain subtrees the collision is built from.
const COLLIDING_MIN_NODES: u32 = 4;

/// Every published cluster id, with the occurrence file lists that
/// claimed it.
fn ids_to_files(report: &serde_json::Value) -> BTreeMap<String, Vec<Vec<String>>> {
    let mut seen: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    for cluster in clusters(report) {
        seen.entry(cluster_id(cluster).to_owned())
            .or_default()
            .push(occurrence_files(cluster));
    }
    seen
}

#[test]
fn no_two_published_clusters_share_one_id() -> Result<()> {
    let report = run_report(&fixture(COLLIDING_FIXTURE), COLLIDING_MIN_NODES)?;
    let by_id = ids_to_files(&report);
    let collisions: Vec<(&String, &Vec<Vec<String>>)> = by_id
        .iter()
        .filter(|(_, claimants)| claimants.len() > 1)
        .collect();
    assert!(
        collisions.is_empty(),
        "a cluster id names one finding: it is what `cluster-by-id` \
         resolves, what the editor stores, and what breaks the ranking \
         tie so the order is total ([PIPELINE-DETERMINISM]). These ids \
         name several findings at once, so all but one of each group is \
         unreachable: {collisions:#?}\nfull report: {report:#}"
    );
    assert!(
        !by_id.is_empty(),
        "the fixture must publish something, or the assertion above \
         holds vacuously: {report:#}"
    );
    Ok(())
}

/// Two copied helpers and two other copied helpers, all of one normalised
/// shape, in one file: `checkHeader`/`checkHeaderAgain` are one copy and
/// `checkStamp`/`checkStampAgain` another, and the two copies differ in
/// their selector and in the assertion they make.
const SAME_FILE_FAMILIES_FIXTURE: &str = "js-cluster-id-same-file-families";

/// `min-nodes` that admits each helper body as one occurrence.
const SAME_FILE_FAMILIES_MIN_NODES: u32 = 8;

/// The one file every occurrence of the fixture lives in.
const SAME_FILE: &str = "checks.js";

/// The two copies the fixture holds, by the lines of their two occurrences.
const SAME_FILE_COPIES: [[(u64, u64); 2]; 2] = [[(1, 4), (6, 9)], [(11, 14), (16, 19)]];

// [PIPELINE-DETERMINISM] The path mix that separates same-shape findings
// across files cannot separate two of them inside one file: the digest
// is the smallest member's shape plus the sorted member paths, and two
// copies of one shape in one file feed it the same shape and the same
// path. `main` stamps the two `ledger` copies of this fixture — and the
// two single-line families inside them — with one id apiece. Tracked as
// gh #531.
#[test]
fn two_same_shape_copies_in_one_file_get_two_ids() -> Result<()> {
    let report = run_report(
        &fixture(SAME_FILE_FAMILIES_FIXTURE),
        SAME_FILE_FAMILIES_MIN_NODES,
    )?;
    let by_id = ids_to_files(&report);
    for copy in SAME_FILE_COPIES {
        assert!(
            clusters(&report)
                .iter()
                .any(|cluster| covers_exactly(cluster, &copy)),
            "each copy is its own finding at its own extent: {copy:?} in {SAME_FILE}: {report:#}"
        );
    }
    let collisions: Vec<(&String, &Vec<Vec<String>>)> = by_id
        .iter()
        .filter(|(_, claimants)| claimants.len() > 1)
        .collect();
    assert!(
        collisions.is_empty(),
        "two copies of one shape in one file are two findings, and `cluster-by-id` \
         must reach both ([PIPELINE-DETERMINISM]); these ids name more than one: \
         {collisions:#?}\nfull report: {report:#}"
    );
    Ok(())
}

/// Whether `cluster` publishes exactly the two line spans of `copy`, both
/// in [`SAME_FILE`].
fn covers_exactly(cluster: &serde_json::Value, copy: &[(u64, u64); 2]) -> bool {
    let mut spans = cluster_line_spans(cluster);
    spans.sort_unstable();
    spans == copy.to_vec()
        && occurrence_files(cluster)
            .iter()
            .all(|path| path.ends_with(SAME_FILE))
}
