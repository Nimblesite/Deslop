//! E2E regression for GH #107 / GH #103 [CLONE-NOISE-PY-DICT-ASSERT].
//!
//! `assert X["k1"]["k2"] == V` chained-subscript assertions across
//! unrelated pytest test functions are a Python idiom for verifying
//! nested response / payload shapes. After identifier normalisation
//! they all collapse to `assert __var__[__str__][__str__] == __const__`,
//! producing cross-file clusters that are not actionable — gh #103
//! class 2, where the literal values change per test and the AST shape
//! is the only thing that clusters. A test suite is *required* to be
//! assertion-dense; that density is not duplication.
//!
//! The fixture stages a false-negative control alongside the three
//! pytest modules: a byte-identical `settle_ledger` pair that must
//! survive whatever hides the assertions. Before it existed this suite
//! asserted an empty report, which a detector that had stopped
//! producing candidates would have satisfied perfectly.
//!
//! The gh #434 `[CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES]` arbitration
//! is what this fixture measured: the pairs that used to publish here
//! were **not** byte-identical at all — adjacent one-line assertions
//! differing in their compared keys and values — so the family grouping
//! was manufacturing "verbatim" families out of differing bytes. It now
//! compares exact source bytes, and this pin is green by default.

use crate::common::{
    negative_pin::{assert_suppressed_family, SuppressedFamily},
    *,
};

/// The three unrelated pytest modules holding the idiom.
const FAMILY: [&str; 3] = [
    "test_configs_patch.py",
    "test_openapi.py",
    "test_sandbox_coverage.py",
];

/// The false-negative control staged in this fixture.
const CONTROL: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];

/// Duplicated lines the control clone accounts for: eight lines, twice.
const CONTROL_LOC: u64 = 16;

/// All three pytest modules and both control files.
const FILES_ANALYSED: u64 = 5;

const FIXTURE: &str = "python-issue-107-chained-dict-assert";
const LABEL: &str = "gh #107/#103 chained-subscript assertion family";
const MIN_NODES: u32 = 4;

/// Components the idiom filter suppresses here — measured, and the
/// number that makes a partially-blind detector fail. With one of the
/// three modules deleted the run still hides a component and reports the
/// control alone with the same 16 duplicated lines, so every `>= 1`
/// bound passes; only this exact count notices that two thirds of the
/// family stopped producing candidates.
const EXPECTED_HIDDEN: u64 = 4;

/// Every half of the contract this fixture is held to, as data: the
/// family judged file by file, the control that must survive it, and the
/// three counts the report must show. Stating it once keeps a pin from
/// quietly asserting less than its neighbours.
const PIN: SuppressedFamily<'static> = SuppressedFamily {
    family_files: &FAMILY,
    control_files: &CONTROL,
    expected_hidden: EXPECTED_HIDDEN,
    control_loc: CONTROL_LOC,
    files_analysed: FILES_ANALYSED,
};

// [CLONE-NOISE-PY-DICT-ASSERT] gh #107, gh #103 class 2.
#[test]
fn chained_dict_assertions_are_suppressed_while_a_real_clone_survives() -> Result<()> {
    let scan_root = fixture(FIXTURE);
    let report = run_report(&scan_root, MIN_NODES)?;

    let offenders = summaries_where(&report, &scan_root, |text| {
        text.contains("assert ") && text.contains("][")
    })?;
    assert!(
        offenders.is_empty(),
        "chained `assert X[k1][k2]` assertions across unrelated test files \
         must not surface as duplicates: {offenders:#?}"
    );
    assert_suppressed_family(&report, LABEL, &PIN)
}
