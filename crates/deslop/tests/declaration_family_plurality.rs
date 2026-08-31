//! [FUSED-CONTENT-GATE] A non-bijective same-shape pair must fail
//! pair-content admission before closure.
//!
//! The filter's whole justification is that a *family* is plural: a
//! window covering one declaration is a unit of logic, however much it
//! resembles its neighbour. `covers_sibling_declarations` is that proof.
//!
//! `csharp-nonbijective-pair` contains two same-shape methods whose raw
//! identifier mappings are not bijective. Shape alone cannot admit them:
//! it is not evidence that the authored methods are duplicates.

use anyhow::Result;

use crate::common::{verdict::duplicated_loc_for_path, *};

const SINGLE_ANALYSED_FILE: u64 = 1;
const NO_CLUSTERS: usize = 0;
const NO_DUPLICATED_LINES: u64 = 0;
const NO_DUPLICATION_PERCENT: f64 = 0.0;

#[test]
fn a_non_bijective_same_shape_pair_is_rejected_before_closure() -> Result<()> {
    let report = run_report(&fixture("csharp-nonbijective-pair"), 20)?;

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(SINGLE_ANALYSED_FILE)
    );
    assert_eq!(
        cluster_count(&report),
        NO_CLUSTERS,
        "shape alone must not publish a clone"
    );
    assert_eq!(
        clusters_hidden(&report),
        NO_DUPLICATED_LINES,
        "rejection happens before suppression"
    );
    assert_eq!(
        duplicated_loc_for_path(&report, "InvoiceTotals.cs")?,
        NO_DUPLICATED_LINES
    );
    assert_eq!(
        metric_field(&report, "duplication_percent").as_f64(),
        Some(NO_DUPLICATION_PERCENT)
    );
    Ok(())
}
