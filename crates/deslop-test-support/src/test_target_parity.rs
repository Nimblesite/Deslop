//! [TEST-SELECTION] Every integration-test source file is reachable by a
//! Cargo test target — proved without asking Cargo.
//!
//! The four Rust crates set `autotests = false` and funnel their suites
//! through a hand-maintained `tests/suite.rs`, because Cargo otherwise
//! builds one whole-program-linked executable per `tests/*.rs`
//! ([CI-RELEASE-BUILD]). That trade is only safe if adding a file cannot
//! silently skip it: with auto-discovery off, a `tests/new_regression.rs`
//! nobody wired into `suite.rs` is not a target, so `make test`, the CI
//! shards and coverage all stay green while the test never runs. Cargo
//! reports nothing, because Cargo never learned the file exists.
//!
//! That is why this gate never consults Cargo's discovered target list —
//! the list is the thing under test. It reads the filesystem for what
//! exists, `Cargo.toml` for the explicitly declared targets, and
//! `tests/suite.rs` through tree-sitter for the modules the suite pulls
//! in, then requires the two sides to agree exactly. A new file that
//! nobody wired up fails here; so does a `mod` line pointing at a file
//! that has been deleted.
//!
//! It lives in this crate's *unit* tests on purpose. `autotests = false`
//! only suppresses integration targets under `tests/`, so a gate placed
//! there could be removed from the run by the very hole it guards.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use deslop_core::lang::{rust_lang::RustParser, shared::parse_source, LanguageParser};
use tree_sitter::Node;

use crate::corpus::repo_root;

/// The crates that declare `autotests = false` and so depend on this gate.
pub const SUITE_CRATES: [&str; 4] = ["deslop", "deslop-core", "deslop-lsp", "deslop-mcp"];

/// The engine's `'static` id for the grammar this scan parses with.
const RUST_LANGUAGE_ID: &str = "rust";
/// The suite root every crate funnels its integration tests through.
const SUITE_FILE: &str = "suite.rs";
/// Directory holding a crate's integration tests, relative to the crate.
const TESTS_DIR: &str = "tests";
/// Extension of a Rust source file.
const RUST_EXTENSION: &str = "rs";
/// tree-sitter-rust kind for `mod name;` and `mod name { .. }`.
const MOD_ITEM: &str = "mod_item";
/// tree-sitter-rust kind for a `#[..]` attribute above an item.
const ATTRIBUTE_ITEM: &str = "attribute_item";
/// tree-sitter-rust kind of a quoted string.
const STRING_LITERAL: &str = "string_literal";
/// tree-sitter-rust field naming the identifier of a `mod_item`.
const NAME_FIELD: &str = "name";
/// The attribute that redirects a module at an explicit file.
const PATH_ATTRIBUTE: &str = "path";
/// TOML table holding one explicitly declared Cargo test target.
const TEST_TABLE: &str = "test";
/// TOML key naming that target's source file.
const PATH_KEY: &str = "path";

/// What one crate's `tests/` directory holds versus what its Cargo test
/// targets actually reach.
#[derive(Debug)]
pub struct SuiteWiring {
    /// Top-level `tests/*.rs` files present on disk, `suite.rs` excluded.
    pub present: BTreeSet<String>,
    /// Top-level `tests/*.rs` files a Cargo test target reaches, either as
    /// a module of `suite.rs` or as its own `[[test]]` target.
    pub reachable: BTreeSet<String>,
}

impl SuiteWiring {
    /// Files that exist but no target reaches — silently skipped tests.
    #[must_use]
    pub fn orphaned(&self) -> Vec<&str> {
        self.present
            .difference(&self.reachable)
            .map(String::as_str)
            .collect()
    }

    /// Files a target names that are not on disk — a dangling `mod`.
    #[must_use]
    pub fn dangling(&self) -> Vec<&str> {
        self.reachable
            .difference(&self.present)
            .map(String::as_str)
            .collect()
    }
}

/// Reads one crate's integration-test wiring.
///
/// # Errors
///
/// Returns an error when the crate's `tests/` directory, `Cargo.toml` or
/// `tests/suite.rs` cannot be read, or when `suite.rs` does not parse.
pub fn wiring(krate: &str) -> Result<SuiteWiring> {
    let tests = repo_root().join("crates").join(krate).join(TESTS_DIR);
    let mut reachable = suite_modules(&tests)?;
    reachable.extend(explicit_targets(krate)?);
    Ok(SuiteWiring {
        present: present_sources(&tests)?,
        reachable,
    })
}

/// Every top-level `tests/*.rs` on disk except the suite root itself.
fn present_sources(tests: &Path) -> Result<BTreeSet<String>> {
    let entries = fs::read_dir(tests)
        .with_context(|| format!("unreadable tests directory: {}", tests.display()))?;
    let mut found = BTreeSet::new();
    for entry in entries {
        let path = entry?.path();
        if let Some(name) = rust_file_name(&path) {
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
///
/// A module reached through a subdirectory (`#[path = "cli/mock_ollama.rs"]`)
/// is not a top-level file and is not part of this comparison.
fn suite_modules(tests: &Path) -> Result<BTreeSet<String>> {
    let suite = tests.join(SUITE_FILE);
    let source = fs::read_to_string(&suite)
        .with_context(|| format!("unreadable suite root: {}", suite.display()))?;
    let grammar = RustParser::new().grammar();
    let tree = parse_source(RUST_LANGUAGE_ID, &grammar, source.as_bytes())
        .with_context(|| format!("unparsable suite root: {}", suite.display()))?;
    let mut found = BTreeSet::new();
    collect_modules(tree.root_node(), &source, tests, &mut found);
    Ok(found)
}

/// Records the file every `mod_item` under `node` resolves to.
fn collect_modules(node: Node<'_>, source: &str, tests: &Path, found: &mut BTreeSet<String>) {
    if node.kind() == MOD_ITEM {
        if let Some(file) = module_file(node, source, tests) {
            let _inserted = found.insert(file);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_modules(child, source, tests, found);
    }
}

/// The top-level file a `mod_item` names, or `None` when it resolves
/// anywhere else.
///
/// Two cases resolve elsewhere and are not part of this comparison: a
/// `#[path]` reaching into a subdirectory (`cli/mock_ollama.rs`), and a
/// bare `mod common;` that Rust resolves to `common/mod.rs` — a directory
/// module, which has no top-level file to be orphaned from.
fn module_file(item: Node<'_>, source: &str, tests: &Path) -> Option<String> {
    let named = match path_attribute(item, source) {
        Some(path) => path,
        None => format!("{}.{RUST_EXTENSION}", field_text(item, NAME_FIELD, source)?),
    };
    let is_top_level_file = !named.contains('/') && tests.join(&named).is_file();
    is_top_level_file.then_some(named)
}

/// The value of a `#[path = ".."]` attribute sitting above `item`.
fn path_attribute(item: Node<'_>, source: &str) -> Option<String> {
    let mut sibling = item.prev_named_sibling();
    while let Some(node) = sibling {
        if node.kind() != ATTRIBUTE_ITEM {
            return None;
        }
        if text(node, source).contains(PATH_ATTRIBUTE) {
            return string_operand(node, source);
        }
        sibling = node.prev_named_sibling();
    }
    None
}

/// The first quoted string anywhere under `node`, unquoted.
fn string_operand(node: Node<'_>, source: &str) -> Option<String> {
    if node.kind() == STRING_LITERAL {
        return Some(text(node, source).trim_matches('"').to_owned());
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find_map(|child| string_operand(child, source));
    found
}

/// The source text of `node`'s named field, when it has one.
fn field_text(node: Node<'_>, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| text(child, source))
}

/// The source slice `node` spans.
fn text(node: Node<'_>, source: &str) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}

/// Top-level `tests/*.rs` files declared as their own `[[test]]` target,
/// read straight out of `Cargo.toml` rather than from Cargo's own
/// discovery. `suite.rs` is excluded: it is the funnel, not a leaf.
fn explicit_targets(krate: &str) -> Result<BTreeSet<String>> {
    let manifest_path = repo_root().join("crates").join(krate).join("Cargo.toml");
    let body = fs::read_to_string(&manifest_path)
        .with_context(|| format!("unreadable manifest: {}", manifest_path.display()))?;
    let manifest: toml::Table = body
        .parse()
        .with_context(|| format!("unparsable manifest: {}", manifest_path.display()))?;
    Ok(manifest
        .get(TEST_TABLE)
        .and_then(toml::Value::as_array)
        .map(|targets| targets.iter().filter_map(target_file).collect())
        .unwrap_or_default())
}

/// The top-level file one `[[test]]` entry points at, `suite.rs` aside.
fn target_file(target: &toml::Value) -> Option<String> {
    let path = PathBuf::from(target.get(PATH_KEY)?.as_str()?);
    let name = rust_file_name_unchecked(&path)?;
    (name != SUITE_FILE).then_some(name)
}

/// The file-name component of `path`, without touching the filesystem.
fn rust_file_name_unchecked(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests;
