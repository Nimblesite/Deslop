//! [RANK-STRUCTURAL-ONLY] The declaration-family filter must prove
//! plurality on **every** suppression path.
//!
//! The filter's whole justification is that a *family* is plural: a
//! window covering one declaration is a unit of logic, however much it
//! resembles its neighbour. `covers_sibling_declarations` is that proof.
//!
//! A short-circuit that suppresses on content evidence alone — before
//! any member range is shown to span sibling declarations — hides
//! single-method and statement-window clones that are not a declaration
//! family at all. Content evidence answers "do these differ in
//! substance"; it cannot answer "is this scaffolding", because the two
//! liftable methods below differ in substance too. Only the AST can say
//! what a member *is*.
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

#[test]
fn a_non_bijective_single_method_pair_is_not_a_declaration_family() -> Result<()> {
    let scan_root = fixture("csharp-nonbijective-pair");
    let report = run_report(&scan_root, 20)?;

    let cluster = expect_sole_cluster(
        &report,
        "two liftable single-method duplicates in one file must publish exactly one \
         cluster. Suppressing them as a sibling-declaration family is a false \
         negative: neither window covers more than one declaration, so the family \
         predicate was never proven.",
    )?;
    assert_single_file_cluster(cluster, 2, "InvoiceTotals.cs");
    let _texts = assert_cluster_mentions(
        &scan_root,
        cluster,
        &["SummariseDomestic", "SummariseExport"],
    )?;
    assert_duplicated_loc_at_least(&report, 10);

    assert!(
        signal(cluster, "structural") >= 0.99,
        "the two bodies share a shape, so the structural signal is saturated: {cluster:#}"
    );
    Ok(())
}
