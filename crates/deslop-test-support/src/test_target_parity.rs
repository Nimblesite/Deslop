//! [TEST-SELECTION] Every integration-test source file is built by a Cargo
//! test target on an ordinary run — proved without asking Cargo.
//!
//! Four Rust crates set `autotests = false` and funnel their suites through
//! a hand-maintained `tests/suite.rs`, because Cargo otherwise builds one
//! whole-program-linked executable per `tests/*.rs` ([CI-RELEASE-BUILD]).
//! That trade is only safe if adding a file cannot silently skip it: with
//! auto-discovery off, a `tests/new_regression.rs` nobody wired into
//! `suite.rs` is not a target, so `make test`, the CI shards and coverage
//! all stay green while the test never runs. Cargo reports nothing, because
//! Cargo never learned the file exists.
//!
//! That is why this gate never consults Cargo's discovered target list —
//! the list is the thing under test. It reads the filesystem for what
//! exists, the manifests for the declared targets, and `tests/suite.rs`
//! through tree-sitter for the modules the suite pulls in, then requires
//! the two sides to agree exactly.
//!
//! **It fails closed on every way Cargo can decline to build a test**, not
//! just on a plain omission. A file counts as reached only when nothing
//! conditional stands between it and the compiler:
//!
//! - a `#[cfg(..)]` or `#[cfg_attr(..)]` on the `suite.rs` module is
//!   conditional — `#[cfg(any())] #[path = "regression.rs"] mod
//!   regression;` mentions the file and never compiles it;
//! - a `[[test]]` carrying `required-features` is conditional — an
//!   ordinary `cargo test` builds no such target;
//! - a declared path is compared whole, so `path = "elsewhere/regression.rs"`
//!   cannot certify an unwired `tests/regression.rs` of the same name;
//! - the guarded crate set is derived from the workspace members rather
//!   than listed here, so a fifth crate setting `autotests = false` is
//!   covered the moment it does;
//! - a `mod` naming a file that no longer exists stays in the reached set,
//!   so `dangling()` can see it rather than it vanishing on the way.
//!
//! It lives in this crate's *unit* tests on purpose. `autotests = false`
//! only suppresses integration targets under `tests/`, so a gate placed
//! there could be removed from the run by the very hole it guards.

mod manifest;
mod suite_scan;

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result};

use crate::corpus::repo_root;
use manifest::{RUST_EXTENSION, SUITE_FILE, TESTS_DIR};

/// Cargo's manifest file name.
const MANIFEST_FILE: &str = "Cargo.toml";

/// Files some set of declarations reaches, split by whether Cargo builds
/// them on an ordinary run.
#[derive(Debug, Default)]
pub(crate) struct Reached {
    /// Built by every plain `cargo test`.
    always: BTreeSet<String>,
    /// Built only when some `cfg` or feature happens to be enabled.
    conditional: BTreeSet<String>,
}

impl Reached {
    /// Files one declaration reaches, on the side its gating puts it.
    fn record(&mut self, file: String, is_conditional: bool) {
        let side = if is_conditional {
            &mut self.conditional
        } else {
            &mut self.always
        };
        let _inserted = side.insert(file);
    }

    /// Folds `other` in, keeping the unconditional side dominant: a file
    /// one declaration builds unconditionally is built, whatever a second
    /// conditional declaration of it says.
    fn absorb(&mut self, other: Self) {
        self.always.extend(other.always);
        self.conditional.extend(other.conditional);
        self.conditional = &self.conditional - &self.always;
    }

    /// Whether any declaration mentions `file`, however it is gated.
    fn mentions(&self, file: &str) -> bool {
        self.always.contains(file) || self.conditional.contains(file)
    }
}

/// What one crate's `tests/` directory holds versus what its Cargo test
/// targets actually build.
#[derive(Debug)]
pub struct SuiteWiring {
    /// Top-level `tests/*.rs` files present on disk, `suite.rs` excluded.
    pub present: BTreeSet<String>,
    /// What the crate's declarations reach, and how conditionally.
    reached: Reached,
}

impl SuiteWiring {
    /// Files that exist and nothing mentions — silently skipped tests.
    #[must_use]
    pub fn orphaned(&self) -> Vec<&str> {
        self.present
            .iter()
            .filter(|file| !self.reached.mentions(file))
            .map(String::as_str)
            .collect()
    }

    /// Files a declaration names that are not on disk — a dangling `mod`
    /// or a `[[test]]` pointing at nothing.
    #[must_use]
    pub fn dangling(&self) -> Vec<&str> {
        let mut found = difference(&self.reached.always, &self.present);
        found.extend(difference(&self.reached.conditional, &self.present));
        found.sort_unstable();
        found.dedup();
        found
    }

    /// Files reached only behind a `cfg` or `required-features` gate.
    ///
    /// Mentioned is not built. These compile on some invocation and not on
    /// an ordinary one, so they are exactly as skippable as a file nobody
    /// wired up — and far easier to miss, because the wiring looks present.
    #[must_use]
    pub fn conditionally_reached(&self) -> Vec<&str> {
        difference(&self.reached.conditional, &self.reached.always)
    }
}

/// Names in `left` that are absent from `right`.
fn difference<'set>(left: &'set BTreeSet<String>, right: &BTreeSet<String>) -> Vec<&'set str> {
    left.iter()
        .filter(|name| !right.contains(*name))
        .map(String::as_str)
        .collect()
}

/// Every workspace member that turned Cargo's test discovery off, as a
/// repo-relative crate directory.
///
/// Derived rather than listed: a new crate setting `autotests = false` is
/// guarded from the moment it does, with nothing to remember to update.
///
/// # Errors
///
/// Returns an error when the workspace manifest, or any member's manifest,
/// cannot be read or parsed.
pub fn suite_crates() -> Result<Vec<String>> {
    let root = read_manifest(&repo_root().join(MANIFEST_FILE))?;
    let mut found = Vec::new();
    for member in manifest::members(&root) {
        let member_manifest = read_manifest(&repo_root().join(&member).join(MANIFEST_FILE))?;
        if manifest::disables_autotests(&member_manifest) {
            found.push(member);
        }
    }
    Ok(found)
}

/// Reads one crate's integration-test wiring, given its repo-relative
/// directory.
///
/// # Errors
///
/// Returns an error when the crate's `tests/` directory, `Cargo.toml` or
/// `tests/suite.rs` cannot be read, or when `suite.rs` does not parse.
pub fn wiring(member: &str) -> Result<SuiteWiring> {
    let crate_dir = repo_root().join(member);
    let tests = crate_dir.join(TESTS_DIR);
    let mut reached = suite_modules(&tests)?;
    reached.absorb(manifest::targets(&read_manifest(
        &crate_dir.join(MANIFEST_FILE),
    )?));
    Ok(SuiteWiring {
        present: present_sources(&tests)?,
        reached,
    })
}

/// Parses one `Cargo.toml`.
fn read_manifest(path: &Path) -> Result<toml::Table> {
    fs::read_to_string(path)
        .with_context(|| format!("unreadable manifest: {}", path.display()))?
        .parse()
        .with_context(|| format!("unparsable manifest: {}", path.display()))
}

/// Every top-level `tests/*.rs` on disk except the suite root itself.
fn present_sources(tests: &Path) -> Result<BTreeSet<String>> {
    let entries = fs::read_dir(tests)
        .with_context(|| format!("unreadable tests directory: {}", tests.display()))?;
    let mut found = BTreeSet::new();
    for entry in entries {
        if let Some(name) = rust_file_name(&entry?.path()) {
            if name != SUITE_FILE {
                let _inserted = found.insert(name);
            }
        }
    }
    Ok(found)
}

/// The file name when `path` is a Rust source file, else `None`.
fn rust_file_name(path: &Path) -> Option<String> {
    let is_rust =
        path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some(RUST_EXTENSION);
    is_rust
        .then(|| path.file_name().and_then(|name| name.to_str()))
        .flatten()
        .map(ToOwned::to_owned)
}

/// The top-level files `tests/suite.rs` pulls in as modules.
fn suite_modules(tests: &Path) -> Result<Reached> {
    let suite = tests.join(SUITE_FILE);
    let source = fs::read_to_string(&suite)
        .with_context(|| format!("unreadable suite root: {}", suite.display()))?;
    suite_scan::scan(&source, tests)
        .with_context(|| format!("unparsable suite root: {}", suite.display()))
}

#[cfg(test)]
mod tests;
