//! [TEST-SELECTION] Every way `tests/suite.rs` can name a file it does not
//! build, read off the tree.
//!
//! Pinned from both sides, for the reason the manifest gates are: an
//! assertion that a gated module goes uncounted proves nothing on its own,
//! because a scan that counted nothing at all would satisfy it too.

use anyhow::Result;

use super::{sample_tests_dir, REGRESSION_FILE};
use crate::test_target_parity::suite_scan;

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

/// A `mod` nested inside an inline module certifies no top-level file.
///
/// `mod helpers { mod regression; }` resolves to
/// `tests/helpers/regression.rs`. Recording the bare name as it was
/// walked past marked `tests/regression.rs` as reached, so an unwired
/// top-level test could be certified by a module that has nothing to do
/// with it — and any `cfg` on the enclosing module was lost on the way
/// down, so a disabled block certified its contents too.
#[test]
fn a_module_nested_in_an_inline_module_certifies_no_top_level_file() -> Result<()> {
    let source = "mod helpers {\n    mod regression;\n}\n";
    let reached = suite_scan::scan(source, &sample_tests_dir())?;
    assert!(
        !reached.mentions(REGRESSION_FILE),
        "a nested `mod regression;` resolves under helpers/, so it says \
         nothing about tests/{REGRESSION_FILE}: {reached:?}"
    );
    Ok(())
}

/// The same nesting under a `cfg`-gated block is equally inert.
#[test]
fn a_cfg_gated_inline_module_certifies_nothing_it_contains() -> Result<()> {
    let source = format!(
        "#[cfg(any())]\nmod disabled {{\n    #[path = \"{REGRESSION_FILE}\"]\n    mod regression;\n}}\n"
    );
    let reached = suite_scan::scan(&source, &sample_tests_dir())?;
    assert!(
        !reached.always.contains(REGRESSION_FILE),
        "nothing inside a `cfg`-gated block is built: {reached:?}"
    );
    Ok(())
}

/// An inline module at the top level names no file of its own either.
#[test]
fn an_inline_module_is_not_itself_a_top_level_file() -> Result<()> {
    let source = "mod inline_helpers {\n    pub const X: u8 = 1;\n}\n";
    let reached = suite_scan::scan(source, &sample_tests_dir())?;
    assert!(
        !reached.mentions("inline_helpers.rs"),
        "a `mod x` with a body has no file of its own: {reached:?}"
    );
    Ok(())
}
