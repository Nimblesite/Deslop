//! [TEST-SELECTION] Every way `tests/suite.rs` can name a file it does not
//! build, read off the tree.
//!
//! Pinned from both sides, for the reason the manifest gates are: an
//! assertion that a gated module goes uncounted proves nothing on its own,
//! because a scan that counted nothing at all would satisfy it too.

use anyhow::Result;

use super::{assert_built, assert_gated, assert_says_nothing, scan, REGRESSION_FILE};

/// A `#[path]` module declaration for the regression file.
const WIRED_MODULE: &str = "#[path = \"regression.rs\"]\nmod regression;\n";

/// A `cfg`-gated suite module mentions its file and never compiles it.
///
/// `#[cfg(any())]` is the sharpest form — it is false in every
/// configuration — but any `cfg` will do: the module reads as wired up in
/// `suite.rs` while Cargo builds nothing. Counting it as reached is how a
/// deleted-in-effect test survives review.
#[test]
fn a_cfg_gated_suite_module_is_not_counted_as_built() -> Result<()> {
    let reached = scan(&format!("#[cfg(any())]\n{WIRED_MODULE}"))?;
    assert_gated(&reached, "a `cfg`-gated module compiles on no ordinary run");
    Ok(())
}

/// A `#![cfg(..)]` at the top of `suite.rs` switches the whole crate off.
///
/// One inner attribute above the first `mod` disables every test in the
/// binary while each module declaration below it is untouched, so reading
/// the attributes attached to each `mod` alone never sees it.
#[test]
fn a_crate_root_cfg_gates_every_module_in_the_suite() -> Result<()> {
    let reached = scan(&format!("#![cfg(any())]\n{WIRED_MODULE}"))?;
    assert_gated(&reached, "a crate-root `cfg` compiles nothing below it");
    Ok(())
}

/// The same module with no gate at all is counted — proof the two
/// assertions above turn on the `cfg` and not on a parse failure that
/// would make every module look unbuilt.
#[test]
fn the_same_module_without_a_cfg_is_counted_as_built() -> Result<()> {
    assert_built(&scan(WIRED_MODULE)?, "an ungated `#[path]` module is built");
    Ok(())
}

/// An inner attribute that is not a `cfg` leaves the suite alone — proof
/// the crate-root check turns on the `cfg` and not on inner attributes in
/// general, of which every real suite root has several.
#[test]
fn a_crate_root_inner_attribute_that_is_not_a_cfg_gates_nothing() -> Result<()> {
    let reached = scan(&format!("#![allow(dead_code)]\n{WIRED_MODULE}"))?;
    assert_built(&reached, "`#![allow(..)]` does not gate compilation");
    Ok(())
}

/// A `mod` nested inside an inline module certifies no top-level file.
///
/// `mod helpers { mod regression; }` resolves to
/// `tests/helpers/regression.rs`. Recording the bare name as it was walked
/// past marked `tests/regression.rs` as reached, so an unwired top-level
/// test could be certified by a module that has nothing to do with it.
#[test]
fn a_module_nested_in_an_inline_module_certifies_no_top_level_file() -> Result<()> {
    let reached = scan("mod helpers {\n    mod regression;\n}\n")?;
    assert_says_nothing(
        &reached,
        "a nested `mod regression;` resolves under helpers/",
    );
    Ok(())
}

/// The same nesting under a `cfg`-gated block is equally inert, and for
/// the same reason: any `cfg` on the enclosing module was lost walking
/// down into it, so a disabled block certified its whole contents.
#[test]
fn a_cfg_gated_inline_module_certifies_nothing_it_contains() -> Result<()> {
    let reached = scan(&format!(
        "#[cfg(any())]\nmod disabled {{\n{WIRED_MODULE}}}\n"
    ))?;
    assert_says_nothing(&reached, "nothing inside a `cfg`-gated block is built");
    Ok(())
}

/// A subdirectory module is neither built-as-a-target nor orphanable.
///
/// `#[path = "cli/mock_ollama.rs"]` is a helper pulled in from a
/// subdirectory, not a top-level integration test, so it takes no part in
/// the comparison — and must not be mistaken for a dangling top-level file
/// just because no `tests/mock_ollama.rs` exists.
#[test]
fn a_subdirectory_module_takes_no_part_in_the_comparison() -> Result<()> {
    let reached = scan("#[path = \"cli/mock_ollama.rs\"]\nmod mock_ollama;\n")?;
    assert!(
        reached.always.is_empty() && reached.conditional.is_empty(),
        "a module reached through a subdirectory is not a top-level test \
         file: {reached:?}"
    );
    Ok(())
}

/// An inline module at the top level names no file of its own either.
#[test]
fn an_inline_module_is_not_itself_a_top_level_file() -> Result<()> {
    let reached = scan("mod inline_helpers {\n    pub const X: u8 = 1;\n}\n")?;
    assert!(
        !reached.mentions("inline_helpers.rs"),
        "a `mod x` with a body has no file of its own: {reached:?}"
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
    let reached = scan(&format!("mod {deleted};\n"))?;
    assert!(
        reached.always.contains(&format!("{deleted}.rs")),
        "a `mod` naming a missing file must stay in the reached set so \
         `dangling()` can report it: {reached:?}"
    );
    assert!(
        !reached.mentions(REGRESSION_FILE),
        "and it must not drag an unrelated file in with it: {reached:?}"
    );
    Ok(())
}
