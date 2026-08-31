//! [FUSED-CONTENT-GATE] and [CLONE-NOISE-VERBATIM-SUBGROUP] E2E coverage. The four byte-identical TypeScript copies are admitted and remain one mass-only cluster. Their shape-identical stranger fails pair content before closure, so it can never be rendered as a copy.

use anyhow::Result;
use serde_json::Value;

use crate::common::{
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    verbatim_subgroup::duplicated_loc_for,
    *,
};

/// The four byte-identical copies that must remain after pairwise admission.
const COPY_FILES: [&str; 4] = ["copy_0.ts", "copy_1.ts", "copy_2.ts", "copy_3.ts"];
/// The shape-identical but content-rejected endpoint.
const STRANGER_FILE: &str = "stranger.ts";
/// The one admitted copy family.
const EXPECTED_CLUSTER_COUNT: usize = 1;
/// Every copy is an occurrence of the admitted family.
const EXPECTED_COPY_OCCURRENCES: usize = 4;
/// The first and only mass-ranked cluster.
const EXPECTED_CLUSTER_RANK: u64 = 1;
/// A rejected stranger contributes no duplicated lines.
const EXPECTED_STRANGER_DUPLICATED_LOC: u64 = 0;

/// Runs the fixture with embeddings disabled, making pair content the independent admission guard.
fn run_family_report() -> Result<Value> {
    run_report_args(
        &fixture("verbatim-plus-stranger"),
        &["--min-nodes", "15", "--embeddings", "off"],
    )
}

#[test]
fn verbatim_copies_survive_and_content_rejected_stranger_never_closes() -> Result<()> {
    let scan_root = fixture("verbatim-plus-stranger");
    let report = run_family_report()?;
    assert_eq!(
        clusters(&report).len(),
        EXPECTED_CLUSTER_COUNT,
        "the four copies form one admitted family: {report:#}"
    );
    let copies = expect_cluster_spanning(&report, &COPY_FILES)?;
    assert_eq!(
        occurrences(copies).len(),
        EXPECTED_COPY_OCCURRENCES,
        "every copy is retained and the stranger is absent: {copies:#}"
    );
    assert_eq!(
        field(copies, "rank").as_u64(),
        Some(EXPECTED_CLUSTER_RANK),
        "the surviving family is the report's first mass-ranked cluster: {copies:#}"
    );
    assert_eq!(
        cluster_file_set(copies),
        COPY_FILES.iter().map(|path| (*path).to_owned()).collect(),
        "the closure contains exactly the admitted copy endpoints"
    );
    assert!(
        has_verbatim_pair(&scan_root, copies)?,
        "the retained family is proven by exact source bytes: {copies:#}"
    );
    assert_structural_only_contract(copies, "verbatim copy family");
    assert_no_pair_surface_on_cluster(copies, "verbatim copy family");
    assert!(
        clusters(&report)
            .iter()
            .all(|cluster| !cluster_file_set(cluster).contains(STRANGER_FILE)),
        "the content-rejected stranger never enters a transitive closure: {report:#}"
    );
    assert_eq!(
        duplicated_loc_for(&report, STRANGER_FILE),
        EXPECTED_STRANGER_DUPLICATED_LOC,
        "the rejected stranger contributes no duplicated lines: {report:#}"
    );
    Ok(())
}
