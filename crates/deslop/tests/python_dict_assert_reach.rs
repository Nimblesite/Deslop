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

mod common;
use crate::common::{verdict::*, *};

/// Asserts the fixture's cross-file copy stayed visible across `files`
/// with at least `minimum_loc` duplicated lines, and that `covers`
/// accepts the reported occurrence texts — each control still names the
/// exact ride-along it refuses to let the idiom vouch for. `why` states
/// what a rejected text set would mean.
fn expect_ride_along_reported(
    fixture_name: &str,
    files: &[&str],
    minimum_loc: u64,
    covers: fn(&[String]) -> bool,
    why: &str,
) -> Result<()> {
    let scan_root = fixture(fixture_name);
    let report = run_report(&scan_root, 8)?;
    let texts = expect_cross_file_duplicate(
        &scan_root,
        &report,
        files,
        2,
        minimum_loc,
        deslop_core::pair::SHARED_SUBTREE_MIN_OVERLAP,
    )?;
    assert!(covers(&texts), "{why}: {texts:#?}");
    Ok(())
}

#[test]
fn module_level_logic_is_not_excused_by_qualifying_tests() -> Result<()> {
    expect_ride_along_reported(
        "python-dict-assert-module-logic",
        &["test_billing_flow.py", "test_invoice_flow.py"],
        2,
        |texts| texts.iter().all(|text| text.contains("build_session")),
        "the duplicated module-level session wiring is executable logic and \
         must be what the cluster reports",
    )
}

#[test]
fn an_unconsumed_payload_dictionary_is_not_excused() -> Result<()> {
    expect_ride_along_reported(
        "python-dict-assert-unconsumed",
        &["test_quota_patch.py", "test_quota_put.py"],
        6,
        |texts| {
            texts.iter().any(|text| text.contains("audit"))
                && texts.iter().any(|text| text.contains("ledger"))
        },
        "the copied bodies including their unconsumed trail dictionaries are \
         the duplication; the report must cover them",
    )
}
