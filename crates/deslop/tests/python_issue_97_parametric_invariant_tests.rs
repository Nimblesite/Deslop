//! E2E regression for GH #97 [CLONE-NOISE-PY-PARAMETRIC-INVARIANT-TESTS].
//!
//! `def test_register_<variant>()` style tests assert the same invariant
//! against each value of an enum discriminator. After identifier
//! normalisation the bodies collapse to identical Type-2 clones, but
//! each test name records a distinct spec assertion: collapsing them
//! into one cluster would silently lose coverage granularity. Bodies
//! that vary only by enum-style identifier access (`X.K8S` vs
//! `X.DOCKER`) inside `test_*` functions must be dropped.

use crate::common::*;

#[test]
fn parametric_enum_invariant_tests_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-97-parametric-invariant-tests");
    let report = run_report(&scan_root, 4)?;
    let offenders = summaries_where(&report, &scan_root, |text| {
        text.contains("register_host(") || text.contains("register_dispatcher(")
    })?;
    assert!(
        offenders.is_empty(),
        "`test_register_<variant>` bodies varying only by enum member \
         access must not surface as duplicates — each test name is its \
         own spec assertion: {offenders:#?}"
    );
    Ok(())
}
