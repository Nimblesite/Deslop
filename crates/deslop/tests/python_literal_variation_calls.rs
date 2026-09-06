//! E2E pins for [CLONE-NOISE-LITERAL-VARIATION-CALLS] on the Python
//! fixtures from GH #70 and #71: a family of test functions whose only
//! variation is the string-literal call arguments is scaffolding, not
//! duplicate logic. These lock the family (three-plus member) arm of the
//! filter so the pair-visibility bound cannot regress it.

use anyhow::Result;

mod common;
use crate::common::*;

// GH #70 regression: four `test_*_write` functions call the same helper
// with differing path/content/id literals — varying test data, not
// extractable duplication. Pinned at min-nodes 8 (like the Dart #70
// pin): the family members are whole call statements, while 4-node
// one-line dict fragments below that threshold are a separate
// data-shape question this fixture does not pin.
#[test]
fn write_file_call_family_is_suppressed_not_reported() -> Result<()> {
    let report = run_report(&fixture("python-issue-70-test-data-variation"), 8)?;
    assert_eq!(
        cluster_count(&report),
        0,
        "a literal-variation call family must not surface as duplication: {report:#}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the family must be actively hidden, not merely absent: {report:#}"
    );
    Ok(())
}

// GH #71 regression: four `test_delete_*` endpoint tests differ only in
// their f-string path template and fixture argument.
#[test]
fn rest_endpoint_family_with_fstring_paths_is_suppressed() -> Result<()> {
    let report = run_report(&fixture("python-issue-71-rest-endpoint-shape"), 4)?;
    assert_eq!(
        cluster_count(&report),
        0,
        "endpoint-shape scaffolding must not surface as duplication: {report:#}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the endpoint family must be actively hidden, not merely absent: {report:#}"
    );
    Ok(())
}
