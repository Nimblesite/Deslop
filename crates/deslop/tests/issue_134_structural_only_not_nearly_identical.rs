//! Issue #134: a same-shape family whose members differ in *substance*
//! must NOT be admitted. The fixture is three
//! handlers sharing one 96-node skeleton whose renamed identifiers map
//! consistently but whose loop strides (`+ 1` / `+ 2` / `+ 3`) diverge
//! at the aligned literal position: [FUSED-CONTENT-GATE] measures zero
//! literal preservation, so no content evidence vouches for the family
//! and pair admission rejects it before closure. The divergent literal is
//! what separates this
//! family from a genuine Type-2 clone — an identical-logic rename with
//! its literals preserved is the *reportable* side of the same line
//! (`fused_golden_bands.rs`, `type2_rename_anchor_floor.rs`,
//! [TECH-PMATCH-BAKER]).
//!
//! Acceptance: raw-content divergence produces no cluster or duplicate
//! metric.

use anyhow::Result;

use crate::common::{verdict::duplicated_loc_for_path, *};

const MIN_NODES: u32 = 30;
const LEFT_FILE: &str = "Alpha.cs";
const RIGHT_FILE: &str = "Beta.cs";
const THIRD_FILE: &str = "Gamma.cs";
const FILE_COUNT: u64 = 3;
const NO_CLUSTERS: usize = 0;
const NO_HIDDEN_COMPONENTS: u64 = 0;
const NO_DUPLICATED_LINES: u64 = 0;
const NO_DUPLICATION_PERCENT: f64 = 0.0;

#[test]
fn issue_134_shape_only_pair_is_rejected_before_closure() -> Result<()> {
    let scan_root = fixture("csharp-issue-134-structural-only");
    let report = run_report(&scan_root, MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILE_COUNT),
        "every fixture file must be analysed"
    );
    assert_eq!(
        cluster_count(&report),
        NO_CLUSTERS,
        "a shape-only family must not form a cluster: {report:#}"
    );
    assert_eq!(
        clusters_hidden(&report),
        NO_HIDDEN_COMPONENTS,
        "the pair fails before suppression: {report:#}"
    );
    for file in [LEFT_FILE, RIGHT_FILE, THIRD_FILE] {
        assert_eq!(
            duplicated_loc_for_path(&report, file)?,
            NO_DUPLICATED_LINES,
            "{file} must receive no duplicate lines from a rejected pair"
        );
    }
    assert_eq!(
        metric_field(&report, "duplication_percent").as_f64(),
        Some(NO_DUPLICATION_PERCENT)
    );
    Ok(())
}
