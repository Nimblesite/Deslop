//! [CLONE-NOISE-PY-DICT-ASSERT] The widened fingerprint reach must not
//! let the chained-dict idiom vouch for code it never inspected.
//!
//! The filter matches every `test_*` function the reported range
//! *intersects*, so its verdict now covers whole-function and
//! whole-module views. That reach is only sound if the idiom proof is
//! closed over everything the range contains: a statement the proof did
//! not read must fail the suppression, not ride along with it.
//!
//! Two rides-along are pinned here:
//!
//! - **Module-level executable logic.** The proof walks `test_*`
//!   functions; a duplicated `SESSION = build_session(...)` call at
//!   module scope is not inside any of them, so a module-view
//!   fingerprint could be suppressed on the strength of tests that
//!   never touched it.
//! - **An unconsumed payload dictionary.** `<name> = {...}` was skipped
//!   as "the payload the assertions read" without checking that any
//!   assertion reads it. A copied test body carrying an `audit` dict no
//!   assert consumes is real duplication, and the dict the filter
//!   excused is precisely the part it never proved.

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

#[test]
fn module_level_logic_is_not_excused_by_qualifying_tests() -> Result<()> {
    let scan_root = fixture("python-dict-assert-module-logic");
    let report = run_report(&scan_root, 8)?;

    let cluster = expect_cluster_spanning(
        &report,
        &["test_billing_flow.py", "test_invoice_flow.py"],
    )?;
    let texts = occurrence_texts(&scan_root, cluster)?;
    assert!(
        texts.iter().all(|text| text.contains("build_session")),
        "the duplicated module-level session wiring is executable logic and \
         must be what the cluster reports: {texts:#?}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "one occurrence per module: {cluster:#}"
    );

    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        duplicated_loc >= 2,
        "the duplicated module-level wiring must count toward the metrics: \
         duplicated_loc={duplicated_loc}, report={report:#}"
    );
    assert!(
        signal(cluster, "structural") >= 0.99,
        "the wiring lines are structurally identical: {cluster:#}"
    );
    Ok(())
}

#[test]
fn an_unconsumed_payload_dictionary_is_not_excused() -> Result<()> {
    let scan_root = fixture("python-dict-assert-unconsumed");
    let report = run_report(&scan_root, 8)?;

    let cluster =
        expect_cluster_spanning(&report, &["test_quota_patch.py", "test_quota_put.py"])?;
    let texts = occurrence_texts(&scan_root, cluster)?;
    assert!(
        texts.iter().any(|text| text.contains("audit"))
            && texts.iter().any(|text| text.contains("ledger")),
        "the copied bodies including their unconsumed trail dictionaries are \
         the duplication; the report must cover them: {texts:#?}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "one occurrence per copied test: {cluster:#}"
    );

    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        duplicated_loc >= 6,
        "both copied bodies are duplicated lines: duplicated_loc={duplicated_loc}, \
         report={report:#}"
    );
    assert!(
        signal(cluster, "structural") >= 0.99,
        "the copied bodies share a shape: {cluster:#}"
    );
    Ok(())
}
