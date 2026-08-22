//! E2E regression for GH #96: module `__all__` export lists are
//! package-surface boilerplate, not duplicate business logic.
//! Tests [CLONE-NOISE-PY-ALL-EXPORTS]


use crate::common::*;

#[test]
fn python_all_export_lists_do_not_surface_as_duplicate_logic() -> Result<()> {
    let scan_root = fixture("python-issue-96-all-exports");
    let report = run_report(&scan_root, 4)?;
    let offenders = summaries_where(&report, &scan_root, |text| text.contains("__all__"))?;
    assert!(
        offenders.is_empty(),
        "__all__ export lists must not be reported as duplicate logic: {offenders:#?}"
    );
    Ok(())
}
