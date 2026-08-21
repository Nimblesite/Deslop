//! gh #285 — independent codec diagnostics are not one clone
//! ([CLONE-NOISE-LITERAL-VARIATION-CALLS], [RANK-STRUCTURAL-ONLY]).
//!
//! Reported against `Nimblesite/typeDiagram` as `#7 Nearly identical
//! code [Type-3] 7 copies, structural = 1.00, token_jaccard = 1.00,
//! fused = 1.00`. The seven members are deliberately independent
//! negative scenarios — unsupported field types, `Option` over an
//! unsupported type, typed `Map` and `Any` diagnostics, an invalid empty
//! record list, an unmonomorphized generic, an invalid union variant.
//! They share only the ordinary test idiom: build a schema, run the
//! codec, hand the result to the existing `expectErrorMessages` helper.
//! Parameterising them would hide the error contracts they document.
//!
//! **Fixed:** the two-line
//! `const result = …; expectErrorMessages(result, […])` family that
//! rendered `nearly_identical` at `fused = 0.86`, and the
//! `buildSchema({ … })` pair — both now recognised as literal-variation
//! call sites, the first because ECMAScript was missing from
//! `call_kinds` entirely, the second because a pure literal collection
//! carrying text is a payload exactly as a bare string is.
//!
//! **Not fixed, and pinned as a bounded residual:** one cluster still
//! covers the four whole `test(…)` blocks, demoted to `structural_only`.
//! `[CLONE-NOISE-LITERAL-VARIATION-CALLS]` requires *every* position of
//! a call sequence to vary, which protects
//! `csharp-unrelated-xunit-tests` — two tests that fetch different URLs
//! and then run four identical assertions are a real clone. Here the
//! invariant position is `encodeTdbin(schema)`: the subject under test,
//! carrying no literal at all, so it can never vary and the rule can
//! never fire. Relaxing it to "every position that carries a literal
//! must vary" would reach this family and would also weaken the xunit
//! control, so it is not done here on a demoted cluster. Tracked on
//! gh #285; the count below fails if a *new* family cluster appears or
//! if this one climbs into an act-now bucket.

use anyhow::Result;

mod common;
use crate::common::{negative_pin::assert_family_demoted_with_control, *};

/// The false-negative control.
const CONTROL: [&str; 2] = ["control_clone_a.ts", "control_clone_b.ts"];

// [CLONE-NOISE-LITERAL-VARIATION-CALLS] gh #285.
#[test]
fn diagnostic_scenarios_stay_demoted_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture("ts-issue-285-diagnostic-scenarios"), 8)?;
    assert_family_demoted_with_control(
        &report,
        "gh #285 codec-diagnostic scenario family",
        &["rust-tdbin.test.ts"],
        &CONTROL,
        1,
    )?;
    assert!(
        clusters_hidden(&report) >= 1,
        "the assertion-helper sub-families must be actively hidden and counted: \
         {report:#}"
    );
    Ok(())
}
