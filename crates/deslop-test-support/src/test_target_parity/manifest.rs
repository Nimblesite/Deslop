//! [TEST-SELECTION] The Cargo side of the parity gate.
//!
//! Two questions are answered here, both straight out of the TOML: which
//! workspace crates switched integration-test discovery off, and which
//! `tests/*.rs` files a declared `[[test]]` target actually builds. Cargo's
//! own view of its targets is the thing under test, so asking Cargo would
//! be circular.

use std::path::{Component, Path};

use super::Reached;

/// TOML table holding the workspace definition.
const WORKSPACE_TABLE: &str = "workspace";
/// TOML key listing the workspace's member crate directories.
const MEMBERS_KEY: &str = "members";
/// TOML table holding a crate's package metadata.
const PACKAGE_TABLE: &str = "package";
/// TOML key that switches Cargo's integration-test discovery off.
const AUTOTESTS_KEY: &str = "autotests";
/// TOML table holding one explicitly declared Cargo test target.
const TEST_TABLE: &str = "test";
/// TOML key naming that target's source file.
const PATH_KEY: &str = "path";
/// TOML key gating a target behind Cargo features.
const REQUIRED_FEATURES_KEY: &str = "required-features";
/// Directory holding a crate's integration tests, relative to the crate.
pub(super) const TESTS_DIR: &str = "tests";
/// Extension of a Rust source file.
pub(super) const RUST_EXTENSION: &str = "rs";
/// The suite root every crate funnels its integration tests through.
pub(super) const SUITE_FILE: &str = "suite.rs";

/// Every path under `[workspace] members`.
pub(super) fn members(root: &toml::Table) -> Vec<String> {
    root.get(WORKSPACE_TABLE)
        .and_then(|workspace| workspace.get(MEMBERS_KEY))
        .and_then(toml::Value::as_array)
        .map(|listed| {
            listed
                .iter()
                .filter_map(toml::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether this crate turned Cargo's integration-test discovery off, and
/// so depends on the suite root being correct.
pub(super) fn disables_autotests(manifest: &toml::Table) -> bool {
    manifest
        .get(PACKAGE_TABLE)
        .and_then(|package| package.get(AUTOTESTS_KEY))
        .and_then(toml::Value::as_bool)
        == Some(false)
}

/// The top-level test files this crate's `[[test]]` entries build.
pub(super) fn targets(manifest: &toml::Table) -> Reached {
    let declared = manifest.get(TEST_TABLE).and_then(toml::Value::as_array);
    let mut reached = Reached::default();
    for target in declared.into_iter().flatten() {
        if let Some(file) = target_file(target) {
            reached.record(file, is_gated(target));
        }
    }
    reached
}

/// The top-level `tests/<name>.rs` one `[[test]]` entry builds, the suite
/// root aside — it is the funnel, not a leaf.
///
/// The whole declared path has to land directly inside `tests/`. A target
/// reading `path = "elsewhere/regression.rs"` builds a different file, and
/// comparing file names alone would let it certify an unwired
/// `tests/regression.rs` that merely shares a name.
fn target_file(target: &toml::Value) -> Option<String> {
    let declared = target.get(PATH_KEY)?.as_str()?;
    let name = tests_relative(Path::new(declared))?;
    (name != SUITE_FILE).then_some(name)
}

/// Whether Cargo builds this target only when extra features are enabled.
///
/// `required-features` is a way for a test to compile on no ordinary run
/// at all, so such a target cannot stand as proof that its file is built.
fn is_gated(target: &toml::Value) -> bool {
    target
        .get(REQUIRED_FEATURES_KEY)
        .and_then(toml::Value::as_array)
        .is_some_and(|features| !features.is_empty())
}

/// The file name when `path` names a Rust source directly inside `tests/`.
fn tests_relative(path: &Path) -> Option<String> {
    let mut components = path.components();
    let in_tests = components.next() == Some(Component::Normal(TESTS_DIR.as_ref()));
    let name = components.next()?;
    let is_only_child = in_tests && components.next().is_none();
    is_only_child.then(|| rust_file(name)).flatten()
}

/// The file name when `path` is a bare Rust file with no directory at all.
pub(super) fn top_level(path: &Path) -> Option<String> {
    let mut components = path.components();
    let only = components.next()?;
    (components.next().is_none())
        .then(|| rust_file(only))
        .flatten()
}

/// `component` as a string when it is a plain Rust file-name component.
fn rust_file(component: Component<'_>) -> Option<String> {
    let Component::Normal(text) = component else {
        return None;
    };
    let is_rust = Path::new(text).extension().and_then(|ext| ext.to_str()) == Some(RUST_EXTENSION);
    is_rust
        .then(|| text.to_str())
        .flatten()
        .map(ToOwned::to_owned)
}
