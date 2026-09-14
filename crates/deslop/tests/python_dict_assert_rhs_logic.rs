//! [CLONE-NOISE-PY-DICT-ASSERT] The chained-dict assertion suppression
//! must read the **right** operand, not only the subscript chain.
//!
//! The filter earns its suppression from one claim: a nested-shape
//! assertion over a locally-built literal payload has no logic in it, so
//! two tests that rhyme are not duplication. That claim holds only while
//! the compared value is a constant.
//!
//! `assert ledger["period"]["gross"] == reconcile_amount(...)` is a
//! different statement. The right operand is executable logic, and when
//! two files carry the same call with the same six arguments that logic
//! is copy-pasted. Judging the statement on its left-hand subscript
//! chain alone reads the copy as payload noise and deletes it.
//!
//! The reach makes it worse rather than better: the filter matches every
//! `test_*` function the reported range *intersects*, so the verdict is
//! applied to the assert run, the enclosing test function, and the whole
//! module alike. One unread right operand therefore erases the
//! duplication at every fingerprint depth at once, which is why this
//! control asserts visibility rather than a particular range.

use anyhow::Result;

use crate::common::{
    signals::{assert_no_pair_surface_on_cluster, assert_structural_only_contract},
    verdict::*,
    *,
};

#[test]
fn a_computed_right_operand_is_not_payload_noise() -> Result<()> {
    let scan_root = fixture("python-dict-assert-rhs-logic");
    let report = run_report(&scan_root, 12)?;

    let visible = clusters(&report);
    assert!(
        !visible.is_empty(),
        "two pytest modules sharing a six-argument `reconcile_amount` call are \
         duplicated executable logic, not a nested-shape payload check. \
         Suppressing them as the chained-dict idiom is a false negative: the \
         predicate never looked at the operand that carries the logic. \
         report={report:#}"
    );

    let cluster = expect_cluster_spanning(
        &report,
        &["test_ledger_period.py", "test_statement_window.py"],
    )?;
    assert_eq!(
        cluster_size(cluster),
        2,
        "one occurrence per module: {cluster:#}"
    );
    assert_eq!(
        occurrence_files(cluster),
        vec!["test_ledger_period.py", "test_statement_window.py"],
        "the cluster names both source files: {cluster:#}"
    );

    let texts = occurrence_texts(&scan_root, cluster)?;
    assert!(
        texts.iter().all(|text| text.contains("reconcile_amount")),
        "every reported occurrence must contain the duplicated call — that \
         call is the reason the cluster is real: {texts:#?}"
    );
    assert!(
        texts.iter().any(|text| text.contains("ledger"))
            && texts.iter().any(|text| text.contains("statement")),
        "both sides of the duplication are reported, not one: {texts:#?}"
    );

    let duplicated_loc = duplicated_loc(&report);
    assert!(
        duplicated_loc >= 4,
        "the duplicated call spans multiple lines in both files and must be \
         counted in the metrics: duplicated_loc={duplicated_loc}, report={report:#}"
    );

    // [PIPELINE-CLUSTER-CLOSURE] The shape axis is pair-scoped; the wire
    // facts that hold the acceptance: the cluster is admitted, mass-honest
    // and clean-surfaced.
    assert_structural_only_contract(cluster, "python dict assert rhs");
    assert_no_pair_surface_on_cluster(cluster, "python dict assert rhs");
    Ok(())
}
