//! [TEST-SELECTION] Every way a declared Cargo target can look like proof
//! that a test file is exercised without being proof.
//!
//! Each gate is pinned from both sides: the gated form must be reported,
//! and the same declaration without the gate must not be. A one-sided
//! assertion cannot tell "the gate works" from "the scan misread the
//! manifest and reports nothing at all".

use anyhow::Result;

use super::{
    ENABLED_SUITE_TARGET, NO_SUITE_TARGET_MANIFEST, REGRESSION_FILE, REQUIRED_FEATURES_KEY,
    SAMPLE_FEATURE,
};
use crate::test_target_parity::manifest;

/// A `[[test]]` behind `required-features` builds on no ordinary run.
#[test]
fn a_required_features_target_is_not_counted_as_built() -> Result<()> {
    let declared: toml::Table = format!(
        "[[test]]\nname = \"regression\"\npath = \"tests/{REGRESSION_FILE}\"\n\
         required-features = [\"live\"]\n"
    )
    .parse()?;
    let reached = manifest::targets(&declared);
    assert!(
        !reached.always.contains(REGRESSION_FILE),
        "`required-features` means a plain `cargo test` builds no such \
         target, so it cannot prove the file is built: {reached:?}"
    );
    assert!(
        reached.conditional.contains(REGRESSION_FILE),
        "the target must still be recorded, so the gate names the file \
         rather than dropping it: {reached:?}"
    );
    Ok(())
}

/// The same target without the gate is counted — proof the assertion
/// above turns on `required-features`.
#[test]
fn the_same_target_without_required_features_is_counted_as_built() -> Result<()> {
    let declared: toml::Table =
        format!("[[test]]\nname = \"regression\"\npath = \"tests/{REGRESSION_FILE}\"\n").parse()?;
    let reached = manifest::targets(&declared);
    assert!(
        reached.always.contains(REGRESSION_FILE),
        "an ungated [[test]] builds its file: {reached:?}"
    );
    Ok(())
}

/// A target pointing outside `tests/` cannot certify a same-named file.
///
/// Reducing a declared path to its file name made `path =
/// "elsewhere/regression.rs"` look like proof that `tests/regression.rs`
/// is built. It is proof about a different file entirely, and the real
/// `tests/regression.rs` stays unwired and unrun.
#[test]
fn a_target_outside_the_tests_directory_certifies_nothing() -> Result<()> {
    let declared: toml::Table =
        format!("[[test]]\nname = \"regression\"\npath = \"somewhere_else/{REGRESSION_FILE}\"\n")
            .parse()?;
    let reached = manifest::targets(&declared);
    assert!(
        !reached.always.contains(REGRESSION_FILE),
        "a target building somewhere_else/{REGRESSION_FILE} says nothing \
         about tests/{REGRESSION_FILE}: {reached:?}"
    );
    assert!(
        reached.conditional.is_empty(),
        "nor is the file conditionally reached — the target is simply \
         about a different file: {reached:?}"
    );
    Ok(())
}

/// A crate that declares no suite target runs none of its suite.
///
/// This is the widest bypass of all: with `autotests = false` and no
/// `[[test]]` naming `tests/suite.rs`, every `mod` line in the suite still
/// reads as perfectly wired and every file on disk is still mentioned, so
/// a check comparing declarations against files agrees with itself while
/// Cargo compiles none of it.
#[test]
fn a_crate_with_no_suite_target_is_reported_as_unbuilt() -> Result<()> {
    let manifest: toml::Table = NO_SUITE_TARGET_MANIFEST.parse()?;
    let defect = manifest::suite_target_defect(&manifest);
    assert!(
        defect.is_some(),
        "no [[test]] compiles the suite root, which must be reported: \
         {defect:?}"
    );
    Ok(())
}

/// A suite target behind `required-features` builds on no ordinary run.
#[test]
fn a_suite_target_behind_required_features_is_reported_as_unbuilt() -> Result<()> {
    let manifest: toml::Table =
        format!("{ENABLED_SUITE_TARGET}required-features = [\"{SAMPLE_FEATURE}\"]\n").parse()?;
    let defect = manifest::suite_target_defect(&manifest);
    assert!(
        defect.is_some_and(|reason| reason.contains(REQUIRED_FEATURES_KEY)),
        "a feature-gated suite target compiles on no plain `cargo test`"
    );
    Ok(())
}

/// A suite target with `test = false` is compiled and never run.
#[test]
fn a_suite_target_with_test_false_is_reported_as_unbuilt() -> Result<()> {
    let manifest: toml::Table = format!("{ENABLED_SUITE_TARGET}test = false\n").parse()?;
    let defect = manifest::suite_target_defect(&manifest);
    assert!(
        defect.is_some(),
        "`test = false` means Cargo runs none of the suite's tests, which \
         is a silent skip of every one of them: {defect:?}"
    );
    Ok(())
}

/// The plain, enabled suite target is not reported — proof the three
/// assertions above turn on the gating and not on the scan misreading an
/// ordinary manifest.
#[test]
fn an_enabled_suite_target_is_not_reported_as_unbuilt() -> Result<()> {
    let manifest: toml::Table = ENABLED_SUITE_TARGET.parse()?;
    assert_eq!(
        manifest::suite_target_defect(&manifest),
        None,
        "an ordinary [[test]] naming tests/suite.rs builds and runs the \
         suite, and must not be flagged"
    );
    Ok(())
}

/// A leaf `[[test]]` with `test = false` compiles and runs nothing.
///
/// `test = false` was only ever checked on the suite root, so unwiring a
/// file from `suite.rs` and declaring a `test = false` target for it left
/// the gate satisfied while Cargo ran none of it — a silent skip dressed
/// up as an explicit target.
#[test]
fn a_leaf_target_with_test_false_is_not_counted_as_built() -> Result<()> {
    let declared: toml::Table = format!(
        "[[test]]\nname = \"regression\"\npath = \"tests/{REGRESSION_FILE}\"\ntest = false\n"
    )
    .parse()?;
    let reached = manifest::targets(&declared);
    assert!(
        !reached.always.contains(REGRESSION_FILE),
        "Cargo runs no tests in a `test = false` target, so it cannot \
         stand as proof the file is exercised: {reached:?}"
    );
    assert!(
        reached.conditional.contains(REGRESSION_FILE),
        "the target must still be recorded so the gate names the file: \
         {reached:?}"
    );
    Ok(())
}

/// The whole bypass, end to end: a file absent from the suite root and
/// declared only as a `test = false` target must still be reported.
#[test]
fn unwiring_a_file_behind_a_test_false_target_is_still_reported() -> Result<()> {
    let manifest: toml::Table = format!(
        "{ENABLED_SUITE_TARGET}\n[[test]]\nname = \"regression\"\npath = \"tests/{REGRESSION_FILE}\"\ntest = false\n"
    )
    .parse()?;
    assert_eq!(
        manifest::suite_target_defect(&manifest),
        None,
        "the suite target itself is fine here; only the leaf is gated"
    );
    let reached = manifest::targets(&manifest);
    assert!(
        !reached.always.contains(REGRESSION_FILE),
        "the leaf is gated, so tests/{REGRESSION_FILE} is not built on an \
         ordinary run and conditionally_reached() must name it: {reached:?}"
    );
    Ok(())
}
