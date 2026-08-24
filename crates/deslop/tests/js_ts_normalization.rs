//! Normalisation-invariance E2E tests for JavaScript and TypeScript
//! ([PIPELINE-NORMALIZE-AST], [CLONE-TYPE-TAXONOMY]).
//!
//! Type-2 detection depends on normalisation erasing everything that a
//! rename can change: quote style, comment form, literal values, and
//! identifier spellings. These tests prove that two files differing only in
//! those dimensions collapse to one clone, while a genuine structural change
//! defeats the collapse — so normalisation neither under- nor over-matches.

use anyhow::Result;

use crate::common::*;

#[test]
fn javascript_clone_is_invariant_to_quotes_comments_literals_and_renames() -> Result<()> {
    let report = run_report(&fixture("js-normalization-invariance"), 12)?;
    // The two files differ ONLY in quote style (" vs '), comment form
    // (/* */ vs //), literal values, and identifier names — every one of
    // which normalisation erases, so exactly one cross-file clone surfaces.
    assert_eq!(
        clusters(&report).len(),
        1,
        "normalisation-equivalent JS pair must collapse to a single clone: {report:#}"
    );
    let clone = expect_cluster_spanning(&report, &["double_quoted.js", "single_quoted.js"])?;
    assert_eq!(cluster_bucket(clone), "structural_only");
    assert!(approx(signal(clone, "structural"), 1.0));
    Ok(())
}

#[test]
fn typescript_token_layer_is_invariant_to_quotes_comments_literals_and_renames() -> Result<()> {
    // Both pairs are rename-invariant at the token level; the difference is
    // that here the interface + async/arrow program node is distinctive
    // enough that the token-LSH pass independently surfaces the pair, so the
    // recorded token signal is carried and the clone is `nearly_identical`
    // (versus the JS pair above, which the token pass never pairs, leaving it
    // `structural_only`).
    assert_bucketed_clone(
        "ts-comment-literal-invariance",
        12,
        &["orders.ts", "shipments.ts"],
        "nearly_identical",
    )
}

#[test]
fn javascript_structural_change_defeats_the_collapse() -> Result<()> {
    let report = run_report(&fixture("js-structural-control"), 10)?;
    // `baseline.js` and `twin.js` are normalisation-equivalent and collapse
    // into the visible, token-supported `nearly_identical` clone. `mutant.js`
    // changes an operator and adds a statement, so the operator change drops
    // it out of that bucket: it never joins the visible clone (its only
    // overlap is a default-hidden `structural_only` shape match), proving the
    // collapse is not over-eager.
    let clone = expect_cluster_spanning(&report, &["baseline.js", "twin.js"])?;
    assert_eq!(cluster_bucket(clone), "nearly_identical");
    assert!(approx(signal(clone, "token_jaccard"), 1.0));
    assert!(
        clusters(&report)
            .iter()
            .all(|cluster| !cluster_file_set(cluster).contains("mutant.js")),
        "the structurally-mutated file must not join the visible clone: {report:#}"
    );
    Ok(())
}
