//! gh #103 class 3 — call sites of an already-extracted helper
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
//!
//! Reported at `4 copies, 39 nodes, weight ≈ 1131, structural = 1.00,
//! token_jaccard = 1.00`. The clustered subtree is the call expression
//! to a helper the test author *already extracted*: every test starts a
//! turn, so every test calls `_post_turn`. Wrapping the wrapper is
//! over-abstraction, and the issue's own words are the contract — "the
//! test author already did the dedup".
//!
//! **Fixed:** every varying literal sat behind a Python
//! `keyword_argument` node (`message="…"`, `conversation_id=None`), so
//! `[CLONE-NOISE-LITERAL-VARIATION-CALLS]` measured *no* string
//! arguments at all and could not fire. Arguments are now unwrapped past
//! the keyword, with the keyword name captured into the call header
//! instead — so `f(alpha="x")` and `f(beta="x")` stay two different call
//! shapes rather than reading as one shape with a varying literal. The
//! four-copy `identical` cluster at `fused = 1.00` is gone.
//!
//! **Not fixed, and pinned as a bounded residual:** three single-line
//! `assert body["k"] == "<literal>"` assertions still cluster, demoted
//! to `structural_only`. [CLONE-NOISE-PY-DICT-ASSERT] covers the
//! *chained*-subscript form (`X[k1][k2]`) and this is the single-subscript
//! one; no filter recognises it. Tracked on gh #103. The count below
//! fails if a new family cluster appears or if this one climbs into an
//! act-now bucket.

use anyhow::Result;

use crate::common::{negative_pin::assert_family_demoted_with_control, *};

/// The false-negative control.
const CONTROL: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];

/// Components suppressed here alongside the demoted residual. Exact
/// rather than the `>= 1` it replaces: that bound could only fail
/// downward, while every over-suppression regression moves this number
/// up.
const EXPECTED_HIDDEN: u64 = 2;

// [CLONE-NOISE-LITERAL-VARIATION-CALLS] gh #103 class 3.
#[test]
fn helper_call_sites_stay_demoted_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture("python-issue-103-helper-call-sites"), 8)?;
    assert_family_demoted_with_control(
        &report,
        "gh #103 already-extracted helper call sites",
        &["test_turns.py"],
        &CONTROL,
        1,
        EXPECTED_HIDDEN,
    )?;
    Ok(())
}
