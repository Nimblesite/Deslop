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
//! What the chains end in is the whole of gh #72: each one writes a
//! literal into a local and then asserts that local against the same
//! literal. Two call-free statements that together test nothing — and
//! two call-free statements used to block the covered-statement
//! precondition outright, so every noise filter reported `fired=0`, the
//! family published as `structural_only`, and `clusters_hidden` read
//! `0`. [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT-TAUTOLOGY]
//! admits that pair and nothing else, which is why this file also pins
//! the other side: build the asserted value instead of writing it down,
//! and the family must still publish.

use anyhow::Result;

use crate::common::{
    negative_pin::{assert_suppressed_family, SuppressedFamily},
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
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

// Exact suppression contract for the family and its byte-identical control.
const PIN: SuppressedFamily<'static> = SuppressedFamily {
    family_files: &FAMILY,
    control_files: &CONTROL,
    control_loc: CONTROL_LOC,
    files_analysed: FILES_ANALYSED,
};

// [CLONE-NOISE-PY-MONKEYPATCH] gh #72, gh #103 class 1.
// [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT-TAUTOLOGY]
#[test]
fn monkeypatch_setenv_chains_are_suppressed_while_a_real_clone_survives() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_suppressed_family(&report, LABEL, &PIN)
}

/// The same three tests, the same two varying `setenv` calls, the same
/// trailing assertion — with one difference: the asserted value is built
/// (`host_prefix + "1"`) instead of written down. Building it is
/// authored data handling, so the tautology clause must not reach it.
const COMPUTED_FIXTURE: &str = "python-computed-value-not-a-tautology";
const COMPUTED_LABEL: &str = "gh #72 computed-value boundary";

/// The one file this fixture holds. There is no control pair here: the
/// pin's own assertion *is* the false-negative side, failing the moment
/// the family stops publishing.
const COMPUTED_FILES_ANALYSED: u64 = 1;

/// Nothing may be suppressed on this fixture.
const COMPUTED_HIDDEN: u64 = 0;

/// One published cluster, holding one occurrence per test function.
const COMPUTED_CLUSTERS: usize = 1;
const COMPUTED_OCCURRENCES: usize = 3;

/// The bucket the family carries: three functions of one shape, with no
/// rename evidence to promote them.

// [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT-TAUTOLOGY]
// The boundary. A clause widened past `name = <literal>` would hide this
// family too, and over-suppression is the false negative this repository
// ranks worst.
#[test]
fn a_computed_value_is_not_a_tautology_and_keeps_its_cluster() -> Result<()> {
    let report = run_report(&fixture(COMPUTED_FIXTURE), MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(COMPUTED_FILES_ANALYSED),
        "{COMPUTED_LABEL}: the family file must reach the pipeline before any \
         verdict about it means anything: {report:#}"
    );
    assert_eq!(
        clusters_hidden(&report),
        COMPUTED_HIDDEN,
        "{COMPUTED_LABEL}: `host_prefix + \"1\"` is a value the members build, \
         not a literal asserted against itself — hiding it is the tautology \
         clause eating authored data handling: {report:#}"
    );
    assert_eq!(
        cluster_count(&report),
        COMPUTED_CLUSTERS,
        "{COMPUTED_LABEL}: the three sibling tests are one published cluster: \
         {report:#}"
    );
    let family = expect_cluster_spanning(&report, &FAMILY)?;
    assert_eq!(
        occurrences(family).len(),
        COMPUTED_OCCURRENCES,
        "{COMPUTED_LABEL}: every one of the three tests must be shown, not a \
         subset a filter trimmed: {report:#}"
    );
    // [PIPELINE-CLUSTER-CLOSURE] The structural_only bucket is gone; the
    // wire facts that hold the acceptance: the family is admitted,
    // mass-honest and clean-surfaced, and its occurrences are byte-distinct
    // (shape agreement only — the bodies differ).
    assert_structural_only_contract(family, COMPUTED_LABEL);
    assert_no_pair_surface_on_cluster(family, COMPUTED_LABEL);
    assert!(
        !has_verbatim_pair(&fixture(COMPUTED_FIXTURE), family)?,
        "{COMPUTED_LABEL}: the computed family differs in bytes (shape only): \
         {report:#}"
    );
    Ok(())
}
