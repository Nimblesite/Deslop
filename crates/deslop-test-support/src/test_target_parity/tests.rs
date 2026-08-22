//! [TEST-SELECTION] The fail-closed half of `autotests = false`.

use anyhow::Result;

use super::{wiring, SUITE_CRATES};

/// Every `tests/*.rs` in every suite crate is reachable by exactly one
/// Cargo test target, and every target names a file that exists.
///
/// This is the assertion that makes `autotests = false` safe. Drop a new
/// `tests/*.rs` in without wiring it into `suite.rs` and it is not a
/// target: `make test`, coverage and all four CI shards stay green while
/// the test never runs once. Nothing else in the tree notices, because
/// every other check starts from the targets Cargo discovered — and the
/// missing file was never discovered.
#[test]
fn every_integration_test_file_is_reachable_by_a_cargo_target() -> Result<()> {
    for krate in SUITE_CRATES {
        let wiring = wiring(krate)?;
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
             crates/{krate}/tests/suite.rs, or declare it as its own \
             [[test]] target in crates/{krate}/Cargo.toml."
        );
        assert_eq!(
            wiring.dangling(),
            Vec::<&str>::new(),
            "{krate}: these files are named by tests/suite.rs or a \
             [[test]] target but do not exist on disk"
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
    let mut wiring = wiring("deslop")?;
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

/// Every suite crate really is covered, so the loop cannot silently
/// shrink to nothing.
#[test]
fn the_gate_covers_every_crate_that_disables_autotests() -> Result<()> {
    assert_eq!(
        SUITE_CRATES,
        ["deslop", "deslop-core", "deslop-lsp", "deslop-mcp"],
        "a crate that sets `autotests = false` must be listed here, or its \
         tests can go missing unnoticed"
    );
    for krate in SUITE_CRATES {
        let manifest = std::fs::read_to_string(
            crate::corpus::repo_root()
                .join("crates")
                .join(krate)
                .join("Cargo.toml"),
        )?;
        let manifest: toml::Table = manifest.parse()?;
        assert_eq!(
            manifest
                .get("package")
                .and_then(|package| package.get("autotests"))
                .and_then(toml::Value::as_bool),
            Some(false),
            "{krate} is listed as a suite crate, so it must actually \
             disable Cargo's test auto-discovery — otherwise this gate is \
             guarding a hole that is not there and the real one is \
             elsewhere"
        );
    }
    Ok(())
}
