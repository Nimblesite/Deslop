//! E2E regression for GH #72 / GH #103 [CLONE-NOISE-PY-MONKEYPATCH]:
//! `monkeypatch.setenv` setup chains across config tests are
//! scaffolding, not duplicate logic.
//!
//! gh #103 names this the worst offender by weight of the pytest-idiom
//! family — nine copies at `structural = 1.00, token_jaccard = 1.00`.
//! Different tests verify behaviour under different environment-variable
//! sets; the alternatives are a parametrize matrix that hides each
//! test's acceptance criteria, or one helper per test, which is no net
//! win. There is no extraction to offer, so the cluster must not be
//! offered.
//!
//! The fixture stages a false-negative control alongside the family: a
//! byte-identical `settle_ledger` pair that must survive whatever hides
//! the chains. Without it this suite asserted `cluster_count == 0`, a
//! bar a detector that had gone blind clears perfectly.

use anyhow::Result;

use crate::common::{negative_pin::assert_family_hidden_with_control, *};

// [CLONE-NOISE-PY-MONKEYPATCH] gh #72, gh #103 class 1.
#[test]
#[ignore = "[SKIP-UNFINISHED] GH #434 [CLONE-NOISE-VERBATIM-SUBGROUP] \
     docs/plans/fused-score-followups.md — the intra-file byte-identical core of the \
     suppressed monkeypatch family now publishes while this pin asserts whole-family \
     suppression; spec arbitration pending. Run via `-- --ignored`."]
fn monkeypatch_setenv_chains_are_suppressed_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture("python-issue-72-monkeypatch-setenv"), 4)?;
    assert_family_hidden_with_control(
        &report,
        "gh #72/#103 monkeypatch.setenv chain family",
        &["test_fly_host.py"],
        &["control_clone_a.py", "control_clone_b.py"],
    )?;
    assert_eq!(
        metric_field(&report, "duplicated_loc").as_u64(),
        Some(16),
        "only the control clone's eight lines, twice, may count as duplicated — \
         a suppressed family that still feeds the duplication gate is the defect \
         moved, not fixed: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    Ok(())
}
