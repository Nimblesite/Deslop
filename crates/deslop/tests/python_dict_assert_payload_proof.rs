//! [CLONE-NOISE-PY-DICT-ASSERT] The idiom proof must read everything it
//! vouches for — payload *values* and decorators included.
//!
//! Two hides-executable-logic holes are pinned here, plus the positive
//! boundary that keeps the fix from overshooting:
//!
//! - **A call inside a consumed payload value.** `payload = {"period":
//!   {"gross": reconcile_amount(...)}}` bound to the name the asserts
//!   read was excused because the outer node is a `dictionary`. The
//!   value is computed logic, and computed logic duplicated across two
//!   modules is the finding this tool exists for. Every payload value
//!   must be proven static data, recursively.
//! - **Executable decorator arguments.** `@pytest.mark.parametrize(...,
//!   build_cases(...))` sits at module scope, outside every `test_*`
//!   body the proof walks. Accepting any `decorated_definition` lets
//!   duplicated case-generation wiring ride along unread. A decorator
//!   qualifies only when its AST is a dotted name, or a call on a
//!   dotted name whose every argument is static data.
//! - **Static decorators stay inside the idiom.** A literal
//!   `@pytest.mark.parametrize("case", [...])` table is test payload,
//!   not logic; rejecting it would resurface the #107 noise class for
//!   every decorated pytest module.

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

#[test]
fn a_call_inside_a_consumed_payload_value_is_not_excused() -> Result<()> {
    let scan_root = fixture("python-dict-assert-call-in-payload");
    let report = run_report(&scan_root, 8)?;

    let cluster = expect_cluster_spanning(
        &report,
        &["test_billing_period.py", "test_revenue_window.py"],
    )?;
    let texts = occurrence_texts(&scan_root, cluster)?;
    assert!(
        texts.iter().all(|text| text.contains("reconcile_amount")),
        "the duplicated reconciliation call is the executable logic the \
         payload dictionary smuggled past the proof; the report must cover \
         it: {texts:#?}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "one occurrence per module: {cluster:#}"
    );
    assert!(
        signal(cluster, "structural") >= 0.99,
        "the two tests are structurally identical: {cluster:#}"
    );
    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        duplicated_loc >= 2,
        "the duplicated bodies must count toward the metrics: \
         duplicated_loc={duplicated_loc}, report={report:#}"
    );
    Ok(())
}

#[test]
fn executable_decorator_arguments_are_not_excused() -> Result<()> {
    let scan_root = fixture("python-dict-assert-decorator-logic");
    let report = run_report(&scan_root, 8)?;

    let cluster =
        expect_cluster_spanning(&report, &["test_billing_cases.py", "test_invoice_cases.py"])?;
    let texts = occurrence_texts(&scan_root, cluster)?;
    assert!(
        texts.iter().all(|text| text.contains("build_cases")),
        "the duplicated case-generation call lives in the decorator, outside \
         every test body the proof walks; the report must cover it: {texts:#?}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "one occurrence per module: {cluster:#}"
    );
    assert!(
        signal(cluster, "structural") >= 0.99,
        "the two modules are structurally identical: {cluster:#}"
    );
    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        duplicated_loc >= 2,
        "the duplicated modules must count toward the metrics: \
         duplicated_loc={duplicated_loc}, report={report:#}"
    );
    Ok(())
}

#[test]
fn static_decorators_stay_within_the_idiom() -> Result<()> {
    let scan_root = fixture("python-dict-assert-decorator-static");
    let report = run_report(&scan_root, 8)?;

    let visible = report
        .pointer("/metrics/clusters_total")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        visible, 0,
        "a literal parametrize table is test payload, not logic; rejecting \
         it resurfaces the #107 noise class for decorated pytest modules: \
         {report:#}"
    );
    let hidden = clusters_hidden(&report);
    assert!(
        hidden >= 1,
        "the rhyming decorated tests must still be detected and suppressed, \
         not missed: clusters_hidden={hidden}, report={report:#}"
    );
    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        duplicated_loc, 0,
        "suppressed idiom noise must not inflate the metrics: {report:#}"
    );
    Ok(())
}
