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
//! - **A decorated class body is not a test function.** Proving the
//!   decorators static said nothing about *what was decorated*. A
//!   decorated `class_definition` carries statements that no `test_*`
//!   walk ever reaches — `session = build_session(...)` executes at
//!   import time — so the class-body statement rode along unread while
//!   its `test_*` methods vouched for the range. An undecorated class
//!   at module scope already fails open here; a decorator may not buy
//!   one a pass.

use anyhow::Result;

mod common;
use crate::common::{verdict::*, *};

#[test]
fn a_call_inside_a_consumed_payload_value_is_not_excused() -> Result<()> {
    let scan_root = fixture("python-dict-assert-call-in-payload");
    let report = run_report(&scan_root, 8)?;

    let texts = expect_cross_file_duplicate(
        &scan_root,
        &report,
        &["test_billing_period.py", "test_revenue_window.py"],
        2,
        2,
    )?;
    assert!(
        texts.iter().all(|text| text.contains("reconcile_amount")),
        "the duplicated reconciliation call is the executable logic the \
         payload dictionary smuggled past the proof; the report must cover \
         it: {texts:#?}"
    );
    Ok(())
}

#[test]
fn executable_decorator_arguments_are_not_excused() -> Result<()> {
    let scan_root = fixture("python-dict-assert-decorator-logic");
    let report = run_report(&scan_root, 8)?;

    let texts = expect_cross_file_duplicate(
        &scan_root,
        &report,
        &["test_billing_cases.py", "test_invoice_cases.py"],
        2,
        2,
    )?;
    assert!(
        texts.iter().all(|text| text.contains("build_cases")),
        "the duplicated case-generation call lives in the decorator, outside \
         every test body the proof walks; the report must cover it: {texts:#?}"
    );
    Ok(())
}

#[test]
fn class_body_logic_under_a_static_decorator_is_not_excused() -> Result<()> {
    let scan_root = fixture("python-dict-assert-decorated-class");
    let report = run_report(&scan_root, 8)?;

    let texts = expect_cross_file_duplicate(
        &scan_root,
        &report,
        &["test_billing_contract.py", "test_shipping_contract.py"],
        2,
        2,
    )?;
    assert!(
        texts.iter().all(|text| text.contains("build_session")),
        "the duplicated session wiring executes in the class body at import \
         time, outside every test method the proof walks; the report must \
         cover it: {texts:#?}"
    );
    Ok(())
}

#[test]
fn static_decorators_stay_within_the_idiom() -> Result<()> {
    let scan_root = fixture("python-dict-assert-decorator-static");
    let report = run_report(&scan_root, 8)?;

    // A literal parametrize table is test payload, not logic: rejecting it
    // would resurface the #107 noise class for every decorated pytest module.
    assert_fully_suppressed(&report, 1);
    Ok(())
}
