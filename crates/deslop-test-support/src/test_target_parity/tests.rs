//! [TEST-SELECTION] The fail-closed half of `autotests = false`.
//!
//! The contract test below runs against the real crates. Everything after
//! it feeds the scanners a declaration that is *designed* to slip through,
//! because a gate is only worth its assertion if it can be shown failing:
//! `orphaned()` returning empty proves nothing unless something can make
//! it return non-empty.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::{manifest, suite_crates, suite_scan, wiring};
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
             `cfg` or `required-features` gate, so an ordinary `cargo \
             test` never builds them. Mentioned is not built."
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

/// A `cfg`-gated suite module mentions its file and never compiles it.
///
/// `#[cfg(any())]` is the sharpest form — it is false in every
/// configuration — but any `cfg` will do: the module reads as wired up in
/// `suite.rs` while Cargo builds nothing. Counting it as reached is how a
/// deleted-in-effect test survives review.
#[test]
fn a_cfg_gated_suite_module_is_not_counted_as_built() -> Result<()> {
    let source = format!("#[cfg(any())]\n#[path = \"{REGRESSION_FILE}\"]\nmod regression;\n");
    let reached = suite_scan::scan(&source, &sample_tests_dir())?;
    assert!(
        !reached.always.contains(REGRESSION_FILE),
        "a `cfg`-gated module compiles on no ordinary run, so it must not \
         count as building its file: {reached:?}"
    );
    assert!(
        reached.conditional.contains(REGRESSION_FILE),
        "the file must still be recorded as conditionally reached, so the \
         gate can name it rather than silently dropping it: {reached:?}"
    );
    Ok(())
}

/// The same module without the gate is counted — proof the assertion
/// above turns on the `cfg` and not on some unrelated parse failure.
#[test]
fn the_same_module_without_a_cfg_is_counted_as_built() -> Result<()> {
    let source = format!("#[path = \"{REGRESSION_FILE}\"]\nmod regression;\n");
    let reached = suite_scan::scan(&source, &sample_tests_dir())?;
    assert!(
        reached.always.contains(REGRESSION_FILE),
        "an ungated `#[path]` module builds its file: {reached:?}"
    );
    assert!(
        reached.conditional.is_empty(),
        "nothing is conditional here: {reached:?}"
    );
    Ok(())
}

/// A `mod` naming a file that has been deleted must be reported.
///
/// Resolving the name against the filesystem first and dropping it when
/// absent is what hid this: the dangling wire left the reached set on the
/// way out, so the check that exists to find it never saw it.
#[test]
fn a_module_naming_a_deleted_file_is_reported_as_dangling() -> Result<()> {
    let deleted = "a_regression_whose_file_was_deleted";
    let source = format!("mod {deleted};\n");
    let reached = suite_scan::scan(&source, &sample_tests_dir())?;
    assert!(
        reached.always.contains(&format!("{deleted}.rs")),
        "a `mod` naming a missing file must stay in the reached set so \
         `dangling()` can report it: {reached:?}"
    );
    Ok(())
}

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

/// A subdirectory module is neither built-as-a-target nor orphanable.
///
/// `#[path = "cli/mock_ollama.rs"]` is a helper pulled in from a
/// subdirectory, not a top-level integration test, so it takes no part in
/// the comparison — and must not be mistaken for a dangling top-level
/// file just because no `tests/mock_ollama.rs` exists.
#[test]
fn a_subdirectory_module_takes_no_part_in_the_comparison() -> Result<()> {
    let nested = "cli/mock_ollama.rs";
    let source = format!("#[path = \"{nested}\"]\nmod mock_ollama;\n");
    let reached = suite_scan::scan(&source, &sample_tests_dir())?;
    assert!(
        reached.always.is_empty() && reached.conditional.is_empty(),
        "a module reached through a subdirectory is not a top-level test \
         file: {reached:?}"
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

/// A `#![cfg(..)]` at the top of `suite.rs` switches the whole crate off.
///
/// One inner attribute above the first `mod` disables every test in the
/// binary while each module declaration below it is untouched, so reading
/// the attributes on the modules alone never sees it.
#[test]
fn a_crate_root_cfg_gates_every_module_in_the_suite() -> Result<()> {
    let source = format!("#![cfg(any())]\n#[path = \"{REGRESSION_FILE}\"]\nmod regression;\n");
    let reached = suite_scan::scan(&source, &sample_tests_dir())?;
    assert!(
        reached.always.is_empty(),
        "a crate-root `cfg` compiles nothing, so no module below it may \
         count as built: {reached:?}"
    );
    assert!(
        reached.conditional.contains(REGRESSION_FILE),
        "the module must still be named, so the gate can report it: \
         {reached:?}"
    );
    Ok(())
}

/// An inner attribute that is not a `cfg` leaves the suite alone — proof
/// the assertion above turns on the `cfg` and not on inner attributes in
/// general, of which every real suite root has several.
#[test]
fn a_crate_root_inner_attribute_that_is_not_a_cfg_gates_nothing() -> Result<()> {
    let source =
        format!("#![allow(dead_code)]\n#[path = \"{REGRESSION_FILE}\"]\nmod regression;\n");
    let reached = suite_scan::scan(&source, &sample_tests_dir())?;
    assert!(
        reached.always.contains(REGRESSION_FILE),
        "`#![allow(..)]` does not gate compilation: {reached:?}"
    );
    assert!(
        reached.conditional.is_empty(),
        "nothing is conditional here: {reached:?}"
    );
    Ok(())
}
