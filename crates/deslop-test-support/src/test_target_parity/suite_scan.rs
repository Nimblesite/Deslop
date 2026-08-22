//! [TEST-SELECTION] The source side of the parity gate: which top-level
//! test files `tests/suite.rs` pulls in, read off the tree.
//!
//! Everything here answers from the shape of the code. Scanning the text
//! for the word `path` would also match `#[cfg(feature = "path")]` and any
//! doc comment mentioning it, and would decide from spelling rather than
//! structure.

use std::path::Path;

use anyhow::{Context, Result};
use deslop_core::lang::{rust_lang::RustParser, shared::parse_source, LanguageParser};
use tree_sitter::Node;

use super::{
    manifest::{top_level, RUST_EXTENSION},
    Reached,
};

/// The engine's `'static` id for the grammar this scan parses with.
const RUST_LANGUAGE_ID: &str = "rust";
/// tree-sitter-rust kind for `mod name;` and `mod name { .. }`.
const MOD_ITEM: &str = "mod_item";
/// tree-sitter-rust kind for a `#[..]` attribute above an item.
const ATTRIBUTE_ITEM: &str = "attribute_item";
/// tree-sitter-rust kind for a `#![..]` attribute applying to the whole
/// enclosing file.
const INNER_ATTRIBUTE_ITEM: &str = "inner_attribute_item";
/// tree-sitter-rust kind of the attribute body inside `#[..]`.
const ATTRIBUTE: &str = "attribute";
/// tree-sitter-rust kind of an identifier.
const IDENTIFIER: &str = "identifier";
/// tree-sitter-rust field naming an attribute's assigned value.
const VALUE_FIELD: &str = "value";
/// tree-sitter-rust kind of the text inside a string literal, quotes
/// excluded — so the value is read off the tree rather than by stripping
/// characters from the literal's source text.
const STRING_CONTENT: &str = "string_content";
/// tree-sitter-rust field naming the identifier of a `mod_item`.
const NAME_FIELD: &str = "name";
/// tree-sitter-rust field holding an inline module's `{ .. }` body.
const BODY_FIELD: &str = "body";
/// The attribute that redirects a module at an explicit file.
const PATH_ATTRIBUTE: &str = "path";
/// The attribute that makes an item's compilation conditional.
const CFG_ATTRIBUTE: &str = "cfg";
/// The attribute that applies further attributes conditionally.
const CFG_ATTR_ATTRIBUTE: &str = "cfg_attr";
/// The file a bare `mod name;` resolves to when `name` is a directory.
const DIRECTORY_MODULE: &str = "mod.rs";

/// What a suite root's `mod` declarations reach, split by whether Cargo
/// compiles them unconditionally.
///
/// # Errors
///
/// Returns an error when the suite root does not parse.
pub(super) fn scan(source: &str, tests: &Path) -> Result<Reached> {
    let grammar = RustParser::new().grammar();
    let tree = parse_source(RUST_LANGUAGE_ID, &grammar, source.as_bytes())
        .context("the suite root must parse, or its modules cannot be read")?;
    let root = tree.root_node();
    let mut reached = Reached::default();
    collect(root, source, tests, &mut reached);
    if crate_is_gated(root, source) {
        reached.make_all_conditional();
    }
    Ok(reached)
}

/// Whether a `#![cfg(..)]` at the top of the file gates the whole suite.
///
/// An inner attribute applies to everything below it, so one line above
/// the first `mod` can switch every test in the crate off while each
/// module declaration still reads as wired up. Checking the attributes on
/// the modules alone never sees it.
fn crate_is_gated(root: Node<'_>, source: &str) -> bool {
    let mut cursor = root.walk();
    let gated = root
        .named_children(&mut cursor)
        .filter(|child| child.kind() == INNER_ATTRIBUTE_ITEM)
        .any(|inner| is_conditional_attribute(inner, source));
    gated
}

/// Whether one attribute node is a `cfg` or `cfg_attr`.
fn is_conditional_attribute(item: Node<'_>, source: &str) -> bool {
    matches!(
        attribute_name(item, source).as_deref(),
        Some(CFG_ATTRIBUTE | CFG_ATTR_ATTRIBUTE)
    )
}

/// Records the file each `mod` at the top of the suite root resolves to.
///
/// Only the top level counts. A `mod` nested inside an inline module
/// resolves against that module's own directory — `mod helpers { mod
/// regression; }` is `tests/helpers/regression.rs`, not
/// `tests/regression.rs` — so walking into inline modules and recording
/// the bare name certified a top-level file that nothing built. Anything
/// gating the enclosing module was lost on the way down too.
fn collect(root: Node<'_>, source: &str, tests: &Path, reached: &mut Reached) {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == MOD_ITEM {
            record(child, source, tests, reached);
        }
    }
}

/// Files one `mod_item` reaches, on the side its attributes put it.
fn record(item: Node<'_>, source: &str, tests: &Path, reached: &mut Reached) {
    let attributes = attribute_run(item, source);
    if let Some(file) = module_file(item, source, tests, attributes.path.as_deref()) {
        reached.record(file, attributes.is_conditional);
    }
}

/// What the run of `#[..]` attributes directly above a module says about
/// it.
struct Attributes {
    /// The file named by the nearest `#[path = ".."]`, when there is one.
    path: Option<String>,
    /// Whether any `#[cfg]` or `#[cfg_attr]` gates the module.
    is_conditional: bool,
}

/// Reads the contiguous attribute run above `item`, nearest first.
fn attribute_run(item: Node<'_>, source: &str) -> Attributes {
    let mut found = Attributes {
        path: None,
        is_conditional: false,
    };
    let mut sibling = item.prev_named_sibling();
    while let Some(node) = sibling.filter(|node| node.kind() == ATTRIBUTE_ITEM) {
        found.is_conditional |= is_conditional_attribute(node, source);
        if attribute_name(node, source).as_deref() == Some(PATH_ATTRIBUTE) {
            found.path = found.path.or_else(|| attribute_value(node, source));
        }
        sibling = node.prev_named_sibling();
    }
    found
}

/// The name of one `attribute_item` — the `identifier` the grammar gives
/// the attribute body.
fn attribute_name(item: Node<'_>, source: &str) -> Option<String> {
    let attribute = child_of_kind(item, ATTRIBUTE)?;
    child_of_kind(attribute, IDENTIFIER).map(|name| text(name, source))
}

/// The string one `#[name = ".."]` attribute assigns, quotes excluded.
fn attribute_value(item: Node<'_>, source: &str) -> Option<String> {
    let attribute = child_of_kind(item, ATTRIBUTE)?;
    let literal = attribute.child_by_field_name(VALUE_FIELD)?;
    child_of_kind(literal, STRING_CONTENT).map(|content| text(content, source))
}

/// The top-level `tests/*.rs` a `mod_item` names, or `None` when it
/// resolves somewhere that is not one.
///
/// Existence is deliberately not part of the answer: a `mod` naming a file
/// that has since been deleted must still be reported, which is exactly
/// what `dangling()` reads. The single filesystem question asked is
/// whether a bare `mod name;` is a directory module — `name/mod.rs` — the
/// one case Rust resolves away from a top-level file on its own.
fn module_file(
    item: Node<'_>,
    source: &str,
    tests: &Path,
    declared: Option<&str>,
) -> Option<String> {
    if item.child_by_field_name(BODY_FIELD).is_some() {
        return None;
    }
    match declared {
        Some(path) => top_level(Path::new(path)),
        None => bare_module_file(item, source, tests),
    }
}

/// The file a `mod name;` with no `#[path]` resolves to.
fn bare_module_file(item: Node<'_>, source: &str, tests: &Path) -> Option<String> {
    let name = text(item.child_by_field_name(NAME_FIELD)?, source);
    let is_directory_module = tests.join(&name).join(DIRECTORY_MODULE).is_file();
    (!is_directory_module).then(|| format!("{name}.{RUST_EXTENSION}"))
}

/// The first direct named child of `node` with `kind`.
fn child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == kind);
    found
}

/// The source slice `node` spans.
fn text(node: Node<'_>, source: &str) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}
