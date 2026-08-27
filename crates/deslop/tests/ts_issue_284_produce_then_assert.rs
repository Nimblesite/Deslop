//! gh #284 — produce-then-assert test scenarios are not one clone
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
//!
//! Reported against `Nimblesite/typeDiagram` at `structural = 0.25,
//! token_jaccard = 0.98, fused = 1.00`: TypeScript Option scalar layout
//! generation pooled with Rust generated-module documentation and with
//! optional-versus-required empty-record pointer semantics. They share
//! only the shape every integration test has — produce code, then assert
//! the fragments it must contain.
//!
//! Two root causes, both now fixed and both language-level rather than
//! scenario-level:
//!
//! 1. `call_kinds` had **no ECMAScript entry at all**, so
//!    [CLONE-NOISE-LITERAL-VARIATION-CALLS] could not fire for any
//!    JavaScript or TypeScript cluster however plainly it was
//!    literal-variation scaffolding. A run of
//!    `expect(generated).toContain("…")` lines rendered `identical` at
//!    `fused = 1.00`.
//! 2. `expect(generated).toContain("…")` is one call whose callee is
//!    spelled with a nested `expect(generated)` invocation. Counting
//!    that receiver as an independent sequence position meant the
//!    sequence held a position that carries no literal and therefore can
//!    never vary — and the filter's "every position must vary" rule
//!    refused a family that varies in the only place it has.
//!
//! The `csharp-unrelated-xunit-tests` fixture is the standing control
//! for that rule and stays green: a receiver is part of a callee, while
//! four genuinely invariant sibling assertions are not.

use anyhow::Result;

use crate::common::{
    negative_pin::{
        assert_control_is_the_only_published_cluster, assert_family_hidden_with_control,
        assert_only_the_control_files_carry_duplicated_lines,
    },
    *,
};

/// The two scenario files.
const FAMILY: [&str; 2] = ["typescript-tdbin.test.ts", "rust-tdbin.test.ts"];

/// The false-negative control.
const CONTROL: [&str; 2] = ["control_clone_a.ts", "control_clone_b.ts"];

/// Duplicated lines the control clone accounts for: eleven lines, twice.
const CONTROL_LOC: u64 = 22;

/// Both scenario files and both control files.
const FILES_ANALYSED: u64 = 4;

/// The committed corpus this pin runs against.
const FIXTURE: &str = "ts-issue-284-produce-then-assert";

/// What every failure message here names itself as.
const LABEL: &str = "gh #284 produce-then-assert scenario family";

/// Node floor at which a run of `expect(...).toContain(...)` statements
/// is a candidate window — the geometry gh #284 reports.
const MIN_NODES: u32 = 8;

/// Components [CLONE-NOISE-LITERAL-VARIATION-CALLS] suppresses here —
/// measured. The two scenario files hold three literal-varying assertion
/// runs between them; a *higher* count is the filter eating something
/// this fixture never staged as noise.
const EXPECTED_HIDDEN: u64 = 3;

// [CLONE-NOISE-LITERAL-VARIATION-CALLS] gh #284.
#[test]
fn produce_then_assert_scenarios_are_suppressed_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    for scenarios in FAMILY {
        assert_family_hidden_with_control(&report, LABEL, &[scenarios], &CONTROL, EXPECTED_HIDDEN)?;
    }
    assert_control_is_the_only_published_cluster(&report, LABEL, &CONTROL, CONTROL_LOC)?;
    assert_only_the_control_files_carry_duplicated_lines(&report, LABEL, &CONTROL);
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED),
        "{LABEL}: both scenario files and both control files were analysed, so the \
         suppression was decided rather than the files skipped: {report:#}"
    );
    Ok(())
}
