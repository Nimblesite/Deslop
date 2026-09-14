//! [RANK-STRUCTURAL-ONLY] The declaration-family filter must prove
//! plurality on **every** suppression path, and [FUSED-CONTENT-GATE]
//! must not refuse a single-method pair on shape or on rename shape.
//!
//! The filter's whole justification is that a *family* is plural: a
//! window covering one declaration is a unit of logic, however much it
//! resembles its neighbour. `covers_sibling_declarations` is that proof.
//!
//! `csharp-nonbijective-pair` is the control. One class, two methods,
//! each carrying a loop, an accumulator, a branch and arithmetic. The
//! identifier substitution is non-bijective by construction (`Amount`
//! aligns with both `Price` and `Cost`; `Quantity` with both `Units`
//! and `Count`), which is exactly the evidence a short-circuit reads as
//! proof of scaffolding. It is not: this pair is what a parameterised
//! extraction lifts, and it is the same thing `csharp-merge-drift`
//! pins for the merge planner.

use anyhow::Result;

use crate::common::{verdict::*, *};

const FIXTURE: &str = "csharp-nonbijective-pair";
const FILE: &str = "InvoiceTotals.cs";
const MIN_NODES: u32 = 20;
const SINGLE_ANALYSED_FILE: u64 = 1;
const PAIR: u64 = 2;
const NOTHING_HIDDEN: u64 = 0;
/// Each method spans thirteen lines; both are duplicated.
const DUPLICATED_LINES: u64 = 26;
const WHY: &str = "two liftable single-method duplicates in one file must publish exactly one \
     cluster. Suppressing them as a sibling-declaration family, or refusing them \
     at admission because their rename is not bijective, is a false negative: \
     neither window covers more than one declaration, and both bodies loop, \
     branch and accumulate.";

#[test]
fn a_non_bijective_single_method_pair_is_not_a_declaration_family() -> Result<()> {
    let scan_root = fixture(FIXTURE);
    let report = run_report(&scan_root, MIN_NODES)?;

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(SINGLE_ANALYSED_FILE)
    );
    let cluster = expect_sole_cluster(&report, WHY)?;
    assert_single_file_cluster(cluster, PAIR, FILE);
    assert_eq!(
        clusters_hidden(&report),
        NOTHING_HIDDEN,
        "{WHY} nothing here is scaffolding, so nothing may be hidden: {report:#}"
    );
    let _texts = assert_cluster_mentions(
        &scan_root,
        cluster,
        &["SummariseDomestic", "SummariseExport", "Math.Round"],
    )?;
    assert_eq!(duplicated_loc_for_path(&report, FILE)?, DUPLICATED_LINES);
    assert_duplicated_loc_at_least(&report, DUPLICATED_LINES);
    Ok(())
}
