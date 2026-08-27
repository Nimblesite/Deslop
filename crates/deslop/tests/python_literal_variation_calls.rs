//! E2E pins for [CLONE-NOISE-LITERAL-VARIATION-CALLS] on the Python
//! fixtures from GH #70, #71 and #79: a family of test functions whose
//! only variation is the string-literal call arguments is scaffolding,
//! not duplicate logic. These lock the family (three-plus member) arm of
//! the filter so the pair-visibility bound cannot regress it.
//!
//! gh #79 is the same family stated from the other end — "these ARE the
//! deduplicated form": every occurrence is a one-line invocation of an
//! already-extracted helper, varying only in its literal arguments, so
//! there is nothing left to extract. #71 is the REST-endpoint spelling
//! of it, where the varying literal is an f-string route template.
//!
//! Every family member here ends in a call-free `assert` on the value
//! its varying call bound, so both pins are also the E2E pins for
//! [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]: without the
//! lone-assertion admission the covered-statement precondition rejects
//! every member and `hidden` falls to zero. The companion negative pin
//! is `rename_needs_an_anchor`, which keeps two-or-more call-free
//! statements blocking the filter.
//!
//! Both fixtures stage a false-negative control — a byte-identical
//! `settle_ledger` pair unrelated to the family — and both assertions go
//! through `negative_pin`. Before that control existed these suites
//! asserted `cluster_count == 0` and nothing else, which a detector that
//! had stopped producing candidates would have satisfied perfectly.
//!
//! The control is not a spot check either. Its bucket, every one of its
//! signals, its rank, its occurrence count and the four metric figures
//! it accounts for are all fixed by the fixture bytes, so all of them
//! are asserted as determined values ([RANK-SCORE], [METRICS-REPO]).

use anyhow::Result;

use crate::common::{
    negative_pin::{assert_suppressed_family, SuppressedFamily},
    *,
};

/// The false-negative control staged in every fixture here.
const CONTROL: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];

/// Duplicated lines the control clone accounts for: eight lines, twice.
const CONTROL_LOC: u64 = 16;

/// Both fixtures hold the family in one file beside the control pair.
const FILES_ANALYSED: u64 = 3;

/// The gh #70/#79 fixture and the one file its family lives in.
const WRITE_FIXTURE: &str = "python-issue-70-test-data-variation";
const WRITE_FAMILY: [&str; 1] = ["test_write_file_calls.py"];
const WRITE_LABEL: &str = "gh #70/#79 literal-variation call family";
/// Min-nodes 8 (like the Dart #70 pin): the family members are whole
/// call statements, while 4-node one-line dict fragments below that
/// threshold are a separate data-shape question this fixture does not
/// pin.
const WRITE_MIN_NODES: u32 = 8;
/// Components the filter suppresses here: the single call-run family
/// cluster over `test_write_file_calls.py`. One, not zero, is what
/// [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT] buys: each
/// member's trailing `assert` inspects only names the covered call
/// bound, so it is admitted instead of blocking the filter.
const WRITE_HIDDEN: u64 = 1;

/// The gh #71 fixture and the one file its family lives in.
const ENDPOINT_FIXTURE: &str = "python-issue-71-rest-endpoint-shape";
const ENDPOINT_FAMILY: [&str; 1] = ["test_endpoints.py"];
const ENDPOINT_LABEL: &str = "gh #71 REST endpoint-shape family";
const ENDPOINT_MIN_NODES: u32 = 4;
/// Components the filter suppresses here: the single endpoint-shape
/// family cluster over `test_endpoints.py`, admitted through
/// [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT] on the
/// trailing `assert resp.status_code == 204`.
const ENDPOINT_HIDDEN: u64 = 1;

/// Everything both pins assert: the family hidden and counted exactly,
/// the byte-identical control published first and whole, the metrics
/// counting it and nothing else — down to which file each duplicated
/// line is charged to — and every file analysed, so the suppression was
/// *decided* rather than the file skipped.
fn assert_family_is_scaffolding(
    fixture_dir: &str,
    min_nodes: u32,
    label: &str,
    family: &[&str],
    expected_hidden: u64,
) -> Result<()> {
    let report = run_report(&fixture(fixture_dir), min_nodes)?;
    assert_suppressed_family(
        &report,
        label,
        &SuppressedFamily {
            family_files: family,
            control_files: &CONTROL,
            expected_hidden,
            control_loc: CONTROL_LOC,
            files_analysed: FILES_ANALYSED,
        },
    )
}

// GH #70 / #79 regression: four `test_*_write` functions call the same
// helper with differing path/content/id literals — varying test data at
// the call site of an already-extracted helper, not extractable
// duplication.
#[test]
fn write_file_call_family_is_suppressed_while_a_real_clone_survives() -> Result<()> {
    assert_family_is_scaffolding(
        WRITE_FIXTURE,
        WRITE_MIN_NODES,
        WRITE_LABEL,
        &WRITE_FAMILY,
        WRITE_HIDDEN,
    )
}

// GH #71 regression: four `test_delete_*` endpoint tests differ only in
// their f-string path template and fixture argument. Different resources
// tested independently — `assert_delete_204(client, url, api_key)` would
// obscure what each test is for, so there is no extraction to offer.
#[test]
fn rest_endpoint_family_is_suppressed_while_a_real_clone_survives() -> Result<()> {
    assert_family_is_scaffolding(
        ENDPOINT_FIXTURE,
        ENDPOINT_MIN_NODES,
        ENDPOINT_LABEL,
        &ENDPOINT_FAMILY,
        ENDPOINT_HIDDEN,
    )
}
