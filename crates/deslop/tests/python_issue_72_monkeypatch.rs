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
//!
//! **Red, and why.** The `[CLONE-NOISE-VERBATIM-SUBGROUP]` arbitration
//! this pin was blocked on is decided and written
//! (`docs/specs/noise.md` §CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE,
//! §-EXACT-BYTES). What is not fixed is the filter: on this fixture
//! every noise filter reports `fired=0`, so the family publishes as
//! `structural_only` and `clusters_hidden` reads `0`. The pin below is
//! the sharp one already — the day the filter fires it goes green with
//! no assertion added.

use anyhow::Result;

use crate::common::{
    negative_pin::{
        assert_control_is_the_only_published_cluster, assert_family_hidden_with_control,
        assert_only_the_control_files_carry_duplicated_lines,
    },
    *,
};

/// The false-negative control staged in this fixture.
const CONTROL: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];

/// Duplicated lines the control clone accounts for: eight lines, twice.
const CONTROL_LOC: u64 = 16;

/// The one file the `monkeypatch.setenv` chain family lives in.
const FAMILY: [&str; 1] = ["test_fly_host.py"];

/// The family file and both control files.
const FILES_ANALYSED: u64 = 3;

const FIXTURE: &str = "python-issue-72-monkeypatch-setenv";
const LABEL: &str = "gh #72/#103 monkeypatch.setenv chain family";
const MIN_NODES: u32 = 4;

/// Components [CLONE-NOISE-PY-MONKEYPATCH] must suppress here: exactly
/// one. The final cluster list holds two — the cross-file control and
/// the single three-member family cluster the subsumption pass elects
/// over `test_fly_host.py` (it absorbs fifteen inner views into one
/// survivor) — and the family is the one that must not publish.
/// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] forbids a verbatim
/// partition here because the family is intra-file, so it is suppressed
/// whole rather than split. A larger number is over-suppression or the
/// hatch re-opening for an intra-file family; zero is today's inert
/// filter.
const EXPECTED_HIDDEN: u64 = 1;

// [CLONE-NOISE-PY-MONKEYPATCH] gh #72, gh #103 class 1.
#[test]
#[ignore = "[SKIP-UNFINISHED] GH #434 [CLONE-NOISE-PY-MONKEYPATCH] \
     docs/plans/fused-score-followups.md — the [CLONE-NOISE-VERBATIM-SUBGROUP] \
     arbitration is decided and written into docs/specs/noise.md, but the filter \
     is inert: every noise filter reports `fired=0` on this fixture, so the family \
     publishes as structural_only and clusters_hidden reads 0. Returns when the \
     filter fires. Run via `-- --ignored`."]
fn monkeypatch_setenv_chains_are_suppressed_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_family_hidden_with_control(&report, LABEL, &FAMILY, &CONTROL, EXPECTED_HIDDEN)?;
    assert_control_is_the_only_published_cluster(&report, LABEL, &CONTROL, CONTROL_LOC)?;
    assert_only_the_control_files_carry_duplicated_lines(&report, LABEL, &CONTROL);
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED),
        "{LABEL}: the family file and both control files were analysed, so the \
         suppression was exercised rather than the file skipped: {report:#}"
    );
    Ok(())
}
