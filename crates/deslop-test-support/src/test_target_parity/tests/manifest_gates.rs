//! [TEST-SELECTION] Every way a declared Cargo target can look like proof
//! that a test file is exercised without being proof.
//!
//! Each gate is pinned from both sides: the gated form must be reported,
//! and the same declaration without the gate must not be. A one-sided
//! assertion cannot tell "the gate works" from "the scan misread the
//! manifest and reports nothing at all".

use anyhow::Result;

use super::{
    assert_built, assert_gated, assert_says_nothing, regression_target, ENABLED_SUITE_TARGET,
    NO_SUITE_TARGET_MANIFEST, REGRESSION_FILE, REQUIRED_FEATURES_KEY, SAMPLE_FEATURE,
};
use crate::test_target_parity::manifest;

/// A `[[test]]` behind `required-features` builds on no ordinary run.
#[test]
fn a_required_features_target_is_not_counted_as_built() -> Result<()> {
    let declared: toml::Table = regression_target(&format!(
        "{REQUIRED_FEATURES_KEY} = [\"{SAMPLE_FEATURE}\"]\n"
    ))
    .parse()?;
    assert_gated(
        &manifest::targets(&declared),
        "`required-features` means a plain `cargo test` builds no such target",
    );
    Ok(())
}

/// A `[[test]]` with `test = false` is compiled and never run.
///
/// `test = false` was only ever checked on the suite root, so unwiring a
/// file from `suite.rs` and declaring a `test = false` target for it left
/// the gate satisfied while Cargo ran none of it — a silent skip dressed
/// up as an explicit target.
#[test]
fn a_leaf_target_with_test_false_is_not_counted_as_built() -> Result<()> {
    let declared: toml::Table = regression_target("test = false\n").parse()?;
    assert_gated(
        &manifest::targets(&declared),
        "Cargo runs no tests in a `test = false` target",
    );
    Ok(())
}

/// The same target with neither key is counted — proof the two assertions
/// above turn on the gating and not on the scan misreading an ordinary
/// manifest and reporting everything as unbuilt.
#[test]
fn the_same_target_without_a_gate_is_counted_as_built() -> Result<()> {
    let declared: toml::Table = regression_target("").parse()?;
    assert_built(
        &manifest::targets(&declared),
        "an ungated [[test]] is built",
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
    assert_says_nothing(
        &manifest::targets(&declared),
        "a target building somewhere_else/regression.rs is about a different file",
    );
    Ok(())
}

/// The whole leaf bypass, end to end: a file absent from the suite root
/// and declared only as a `test = false` target must still be reported,
/// and the suite root's own target must not be blamed for it.
#[test]
fn unwiring_a_file_behind_a_test_false_target_is_still_reported() -> Result<()> {
    let manifest: toml::Table = format!(
        "{ENABLED_SUITE_TARGET}\n{}",
        regression_target("test = false\n")
    )
    .parse()?;
    assert_eq!(
        manifest::suite_target_defect(&manifest),
        None,
        "the suite target itself is fine here; only the leaf is gated"
    );
    assert_gated(
        &manifest::targets(&manifest),
        "the leaf target runs nothing",
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
        "no [[test]] compiles the suite root, which must be reported: {defect:?}"
    );
    Ok(())
}

/// A suite target behind `required-features` builds on no ordinary run.
#[test]
fn a_suite_target_behind_required_features_is_reported_as_unbuilt() -> Result<()> {
    let manifest: toml::Table =
        format!("{ENABLED_SUITE_TARGET}{REQUIRED_FEATURES_KEY} = [\"{SAMPLE_FEATURE}\"]\n")
            .parse()?;
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

/// A `[[test]]` with `harness = false` compiles every test and calls none.
///
/// The quietest bypass of the three: the target is enabled, Cargo builds
/// it, the binary runs, and it exits zero — because `harness = false`
/// replaces libtest with the file's own `main`, so no `#[test]` is ever
/// invoked. Pair it with `fn main() {}` in `suite.rs` and the entire suite
/// reports success having executed nothing.
#[test]
fn a_leaf_target_with_no_harness_is_not_counted_as_built() -> Result<()> {
    let declared: toml::Table = regression_target("harness = false\n").parse()?;
    assert_gated(
        &manifest::targets(&declared),
        "`harness = false` means no `#[test]` in the file is ever called",
    );
    Ok(())
}

/// The same, on the suite root itself — where it would silence everything.
#[test]
fn a_suite_target_with_no_harness_is_reported_as_unbuilt() -> Result<()> {
    let manifest: toml::Table = format!("{ENABLED_SUITE_TARGET}harness = false\n").parse()?;
    let defect = manifest::suite_target_defect(&manifest);
    assert!(
        defect.is_some(),
        "a harness-less suite root runs its own `main` and calls no test \
         in any module it pulls in: {defect:?}"
    );
    Ok(())
}
