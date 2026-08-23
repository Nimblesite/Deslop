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
//! Both fixtures now stage a false-negative control — a byte-identical
//! `settle_ledger` pair unrelated to the family — and both assertions go
//! through `negative_pin::assert_family_hidden_with_control`. Before
//! that control existed these suites asserted `cluster_count == 0` and
//! nothing else, which a detector that had stopped producing candidates
//! would have satisfied perfectly.

use anyhow::Result;

use crate::common::{negative_pin::assert_family_hidden_with_control, *};

/// The false-negative control staged in every fixture here.
const CONTROL: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];

/// Duplicated lines the control clone accounts for: eight lines, twice.
const CONTROL_LOC: u64 = 16;

/// Asserts the metric counts the control clone and nothing else — the
/// half a `cluster_count` assertion cannot see, since a suppressed
/// family that still fed `duplicated_loc` would satisfy it.
fn assert_metric_counts_only_the_control(report: &serde_json::Value, label: &str) {
    assert_eq!(
        metric_field(report, "duplicated_loc").as_u64(),
        Some(CONTROL_LOC),
        "{label}: only the control clone's lines may count as duplicated — a \
         suppressed family that still feeds the duplication gate is the defect \
         moved, not fixed: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
}

// GH #70 / #79 regression: four `test_*_write` functions call the same
// helper with differing path/content/id literals — varying test data at
// the call site of an already-extracted helper, not extractable
// duplication. Pinned at min-nodes 8 (like the Dart #70 pin): the family
// members are whole call statements, while 4-node one-line dict
// fragments below that threshold are a separate data-shape question this
// fixture does not pin.
#[test]
#[ignore = "[SKIP-UNFINISHED] GH #434 [CLONE-NOISE-VERBATIM-SUBGROUP] \
     docs/plans/fused-score-followups.md — the intra-file byte-identical core of the \
     suppressed write-file family now publishes while this pin asserts whole-family \
     suppression; spec arbitration pending. Run via `-- --ignored`."]
fn write_file_call_family_is_suppressed_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture("python-issue-70-test-data-variation"), 8)?;
    assert_family_hidden_with_control(
        &report,
        "gh #70/#79 literal-variation call family",
        &["test_write_file_calls.py"],
        &CONTROL,
    )?;
    assert_metric_counts_only_the_control(&report, "gh #70/#79");
    Ok(())
}

// GH #71 regression: four `test_delete_*` endpoint tests differ only in
// their f-string path template and fixture argument. Different resources
// tested independently — `assert_delete_204(client, url, api_key)` would
// obscure what each test is for, so there is no extraction to offer.
#[test]
#[ignore = "[SKIP-UNFINISHED] GH #434 [CLONE-NOISE-VERBATIM-SUBGROUP] \
     docs/plans/fused-score-followups.md — the intra-file byte-identical core of the \
     suppressed rest-endpoint family now publishes while this pin asserts whole-family \
     suppression; spec arbitration pending. Run via `-- --ignored`."]
fn rest_endpoint_family_is_suppressed_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture("python-issue-71-rest-endpoint-shape"), 4)?;
    assert_family_hidden_with_control(
        &report,
        "gh #71 REST endpoint-shape family",
        &["test_endpoints.py"],
        &CONTROL,
    )?;
    assert_metric_counts_only_the_control(&report, "gh #71");
    Ok(())
}
