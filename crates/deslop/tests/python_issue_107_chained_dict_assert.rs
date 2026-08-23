//! E2E regression for GH #107 / GH #103 [CLONE-NOISE-PY-DICT-ASSERT].
//!
//! `assert X["k1"]["k2"] == V` chained-subscript assertions across
//! unrelated pytest test functions are a Python idiom for verifying
//! nested response / payload shapes. After identifier normalisation
//! they all collapse to `assert __var__[__str__][__str__] == __const__`,
//! producing cross-file clusters that are not actionable — gh #103
//! class 2, where the literal values change per test and the AST shape
//! is the only thing that clusters. A test suite is *required* to be
//! assertion-dense; that density is not duplication.
//!
//! The fixture stages a false-negative control alongside the three
//! pytest modules: a byte-identical `settle_ledger` pair that must
//! survive whatever hides the assertions. Before it existed this suite
//! asserted an empty report, which a detector that had stopped
//! producing candidates would have satisfied perfectly.

use crate::common::{negative_pin::assert_family_hidden_with_control, verdict::*, *};

/// The three unrelated pytest modules holding the idiom.
const FAMILY: [&str; 3] = [
    "test_configs_patch.py",
    "test_openapi.py",
    "test_sandbox_coverage.py",
];

// [CLONE-NOISE-PY-DICT-ASSERT] gh #107, gh #103 class 2.
#[test]
#[ignore = "[SKIP-UNFINISHED] GH #434 [CLONE-NOISE-VERBATIM-SUBGROUP] \
     docs/plans/fused-score-followups.md — the intra-file byte-identical core of the \
     suppressed dict-assert family now publishes while this pin asserts whole-family \
     suppression; spec arbitration pending. Run via `-- --ignored`."]
fn chained_dict_assertions_are_suppressed_while_a_real_clone_survives() -> Result<()> {
    let scan_root = fixture("python-issue-107-chained-dict-assert");
    let report = run_report(&scan_root, 4)?;

    let offenders = summaries_where(&report, &scan_root, |text| {
        text.contains("assert ") && text.contains("][")
    })?;
    assert!(
        offenders.is_empty(),
        "chained `assert X[k1][k2]` assertions across unrelated test files \
         must not surface as duplicates: {offenders:#?}"
    );
    for module in FAMILY {
        assert_family_hidden_with_control(
            &report,
            "gh #107/#103 chained-subscript assertion family",
            &[module],
            &["control_clone_a.py", "control_clone_b.py"],
        )?;
    }
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(5),
        "all three pytest modules and both control files were analysed, so the \
         suppression was exercised rather than the files skipped: {report:#}"
    );
    assert_eq!(
        clusters(&report).len(),
        1,
        "the three modules share nothing beyond the idiom, so the control clone \
         must be the only surviving cluster of any bucket: {report:#}"
    );
    assert_eq!(
        duplicated_loc(&report),
        16,
        "suppressed idiom matches must not count as duplicated lines; only the \
         control clone's eight lines, twice, may: {report:#}"
    );
    Ok(())
}
