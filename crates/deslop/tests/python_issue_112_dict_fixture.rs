//! E2E regression for GH #112 [CLONE-NOISE-PY-DICT-FIXTURE].
//!
//! Small nested dict literals inside pytest test functions share AST
//! shape and token alphabet (`name`, `description`, ...) but encode
//! unrelated request / response payloads. Extracting them into a shared
//! factory would erase the per-test contract.

use crate::common::*;

#[test]
fn nested_dict_literal_fixtures_across_test_files_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-112-dict-fixture");
    let report = run_report(&scan_root, 4)?;
    let offenders = summaries_where(&report, &scan_root, |text| text.contains("\"name\":"))?;
    assert!(
        offenders.is_empty(),
        "small nested dict literals across pytest test files must not surface \
         as duplicate logic: {offenders:#?}"
    );
    Ok(())
}
