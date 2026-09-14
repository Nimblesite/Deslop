//! [TEST-SELECTION] The fail-closed half of `autotests = false`.
//!
//! The contract test below runs against the real crates. Everything after
//! it feeds the scanners a declaration that is *designed* to slip through,
//! because a gate is only worth its assertion if it can be shown failing:
//! `orphaned()` returning empty proves nothing unless something can make
//! it return non-empty.

mod manifest_gates;
mod scan_gates;

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::{suite_crates, suite_scan, wiring, Reached};
use crate::corpus::repo_root;

/// The crates known to funnel their integration tests through a suite root
/// at the time of writing. The gate derives its own list; this is the
/// independent copy that notices the derivation silently shrinking.
const KNOWN_SUITE_CRATES: [&str; 4] = [
    "crates/deslop-core",
    "crates/deslop",
    "crates/deslop-lsp",
    "crates/deslop-mcp",
];

/// A crate directory whose `tests/` is used as a plausible backdrop for
/// the synthetic scans below.
const SAMPLE_CRATE: &str = "crates/deslop";

/// The file the synthetic declarations all point at.
const REGRESSION_FILE: &str = "regression.rs";

/// A plain, enabled `[[test]]` naming the suite root.
const ENABLED_SUITE_TARGET: &str = "[[test]]\nname = \"suite\"\npath = \"tests/suite.rs\"\n";

/// A manifest that declares a test target, but not one for the suite root.
const NO_SUITE_TARGET_MANIFEST: &str =
    "[[test]]\nname = \"regression\"\npath = \"tests/regression.rs\"\n";

/// A Cargo feature name to gate a synthetic target behind.
const SAMPLE_FEATURE: &str = "live";

/// The manifest key that gates a target behind features.
const REQUIRED_FEATURES_KEY: &str = "required-features";

/// One `[[test]]` naming the regression file, plus `extra` manifest keys.
///
/// Every gate below is the same declaration with one key added, so the
/// declaration itself is written once and the key under test is the only
/// thing each case spells out.
fn regression_target(extra: &str) -> String {
    format!("[[test]]\nname = \"regression\"\npath = \"tests/{REGRESSION_FILE}\"\n{extra}")
}

/// Scans a synthetic suite root against a real `tests/` directory.
fn scan(source: &str) -> Result<Reached> {
    suite_scan::scan(source, &sample_tests_dir())
}

/// Asserts the regression file is built and run on every ordinary run.
fn assert_built(reached: &Reached, why: &str) {
    assert!(
        reached.always.contains(REGRESSION_FILE),
        "{why}, so tests/{REGRESSION_FILE} must count as built: {reached:?}"
    );
    assert!(
        reached.conditional.is_empty(),
        "{why}, so nothing here is conditional: {reached:?}"
    );
}

/// Asserts the regression file is mentioned but not built on an ordinary
/// run — the distinction the whole gate turns on.
///
/// Both halves matter. Not being counted as built is what stops the gate
/// passing; still being mentioned is what lets it name the file instead of
/// dropping it and reporting nothing at all.
fn assert_gated(reached: &Reached, why: &str) {
    assert!(
        !reached.always.contains(REGRESSION_FILE),
        "{why}, so it cannot stand as proof tests/{REGRESSION_FILE} is \
         exercised: {reached:?}"
    );
    assert!(
        reached.conditional.contains(REGRESSION_FILE),
        "{why}, and the file must still be recorded so the gate names it \
         rather than dropping it: {reached:?}"
    );
}

/// Asserts nothing at all reaches the regression file.
fn assert_says_nothing(reached: &Reached, why: &str) {
    assert!(
        !reached.mentions(REGRESSION_FILE),
        "{why}, so it says nothing about tests/{REGRESSION_FILE}, gated or \
         otherwise: {reached:?}"
    );
}

/// A `tests/` directory to resolve bare `mod` declarations against.
fn sample_tests_dir() -> PathBuf {
    repo_root().join(SAMPLE_CRATE).join("tests")
}

/// Every `tests/*.rs` in every suite crate is built by a Cargo test target
/// on an ordinary run, every target names a file that exists, and nothing
/// is reached only behind a `cfg` or a feature.
///
/// This is the assertion that makes `autotests = false` safe. Drop a new
/// `tests/*.rs` in without wiring it into `suite.rs` and it is not a
/// target: `make test`, coverage and all four CI shards stay green while
/// the test never runs once. Nothing else in the tree notices, because
/// every other check starts from the targets Cargo discovered — and the
/// missing file was never discovered.
#[test]
fn every_integration_test_file_is_built_by_a_cargo_target() -> Result<()> {
    let crates = suite_crates()?;
    assert!(
        !crates.is_empty(),
        "no workspace member sets `autotests = false`, so either the \
         derivation is reading the wrong manifests or this gate is \
         guarding a hole that no longer exists"
    );
    for krate in crates {
        let wiring = wiring(&krate)?;
        assert!(
            !wiring.present.is_empty(),
            "{krate}: no top-level tests/*.rs found at all — the scan is \
             looking in the wrong place, and an empty scan would let this \
             gate pass while proving nothing"
        );
        assert_eq!(
            wiring.orphaned(),
            Vec::<&str>::new(),
            "{krate}: these tests/*.rs files are not reachable from \
             tests/suite.rs or any [[test]] target. With `autotests = \
             false` they are not compiled and not run, and every gate \
             stays green regardless of what they assert. Add each one to \
             {krate}/tests/suite.rs, or declare it as its own [[test]] \
             target in {krate}/Cargo.toml."
        );
        assert_eq!(
            wiring.dangling(),
            Vec::<&str>::new(),
            "{krate}: these files are named by tests/suite.rs or a \
             [[test]] target but do not exist on disk"
        );
        assert_eq!(
            wiring.unbuilt_suite(),
            None,
            "{krate}: nothing Cargo builds compiles tests/suite.rs, so every \
             module in it is decoration and the whole suite is skipped"
        );
        assert_eq!(
            wiring.conditionally_reached(),
            Vec::<&str>::new(),
            "{krate}: these tests/*.rs files are reached only behind a \
             `cfg`, a `required-features`, or a `test = false` target, so \
             an ordinary `cargo test` never runs them. Mentioned is not \
             built, and built is not run."
        );
    }
    Ok(())
}

/// The gate can actually see a missing wire — proof it is not vacuous.
///
/// An assertion that passes because the scan found nothing is worth
/// nothing, so this pins the mechanism from the other side: a file that
/// exists and is not wired up must be reported as orphaned. Without it,
/// `orphaned()` could return empty for the wrong reason and the contract
/// above would never fail no matter what landed in `tests/`.
#[test]
fn an_unwired_file_is_reported_as_orphaned() -> Result<()> {
    let mut wiring = wiring(SAMPLE_CRATE)?;
    let unwired = "tests_a_new_regression_nobody_wired_up.rs";
    let _inserted = wiring.present.insert(unwired.to_owned());
    assert_eq!(
        wiring.orphaned(),
        vec![unwired],
        "a tests/*.rs no target reaches must be reported as orphaned"
    );
    assert_eq!(
        wiring.dangling(),
        Vec::<&str>::new(),
        "adding a present file must not invent a dangling one"
    );
    Ok(())
}

/// The guarded set is read out of the workspace, not remembered here.
///
/// A hard-coded list only proves things about the crates on it. A fifth
/// crate could set `autotests = false`, be left off, and have no parity
/// protection at all — which is why the gate derives the set and this
/// asserts the derivation still finds every crate known to need it.
#[test]
fn the_guarded_crate_set_is_derived_from_the_workspace() -> Result<()> {
    let mut derived = suite_crates()?;
    derived.sort();
    let mut known = KNOWN_SUITE_CRATES.map(ToOwned::to_owned).to_vec();
    known.sort();
    assert_eq!(
        derived, known,
        "every workspace member that disables Cargo's test discovery must \
         be guarded. A member appearing here is newly unguarded; one \
         disappearing means the derivation stopped seeing it and its \
         tests can now go missing unnoticed."
    );
    Ok(())
}

/// Every derived crate really does disable auto-discovery, and really has
/// the suite root the derivation assumes.
#[test]
fn every_derived_crate_disables_autotests_and_has_a_suite_root() -> Result<()> {
    for krate in suite_crates()? {
        let suite = repo_root().join(&krate).join("tests").join("suite.rs");
        assert!(
            Path::new(&suite).is_file(),
            "{krate} disables Cargo's test discovery, so its tests only \
             run through {}, which must exist",
            suite.display()
        );
    }
    Ok(())
}
