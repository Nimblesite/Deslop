//! E2E regression for GH #107 [CLONE-NOISE-PY-DICT-ASSERT].
//!
//! `assert X["k1"]["k2"] == V` chained-subscript assertions across
//! unrelated pytest test functions are a Python idiom for verifying
//! nested response / payload shapes. After identifier normalisation
//! they all collapse to `assert __var__[__str__][__str__] == __const__`,
//! producing cross-file clusters that are not actionable.

mod common;

use crate::common::*;

#[test]
fn chained_dict_assertions_across_test_files_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-107-chained-dict-assert");
    let report = run_report(&scan_root, 4)?;
    let offenders =
        summaries_where(&report, &scan_root, |text| text.contains("assert ") && text.contains("]["))?;
    assert!(
        offenders.is_empty(),
        "chained `assert X[k1][k2]` assertions across unrelated test files \
         must not surface as duplicates: {offenders:#?}"
    );
    Ok(())
}
