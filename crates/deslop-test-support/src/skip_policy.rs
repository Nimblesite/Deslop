//! [TEST-SELECTION-SKIP] Every `#[ignore]` in the workspace, read off the AST.
//!
//! `cargo test --skip` matches a substring of the *test name*, so it drops
//! whatever happens to share those characters. That is how the corpus gate's
//! own self-tests stopped running while the gate reported green (gh #412).
//! Selection by name is gone. A test that must not run in the release gate
//! now says so at its own declaration — the one place a reader of that test
//! will look.
//!
//! An `#[ignore]` is still a test that protects nothing, so the attribute is
//! only half the mechanism. This module extracts each skip and its stated
//! reason through tree-sitter — never by matching source text — so
//! `crates/deslop/tests/skip_policy_contract.rs` can hold every one of them
//! to the documented policy.

use std::{
    fs,
    iter::Peekable,
    path::{Path, PathBuf},
    str::Chars,
};

use anyhow::{bail, Context, Result};
use deslop_core::lang::{rust_lang::RustParser, shared::parse_source, LanguageParser};
use tree_sitter::Node;

use crate::corpus::repo_root;

/// The engine's `'static` id for the grammar this scan parses with.
const RUST_LANGUAGE_ID: &str = "rust";

/// One outer attribute with its brackets. tree-sitter-rust makes these
/// siblings of the item they decorate, not children of it, so an `#[ignore]`
/// is found by walking forward from the attribute rather than down from the
/// function.
const ATTRIBUTE_ITEM: &str = "attribute_item";
/// The attribute inside an `attribute_item`: its path and optional operand.
const ATTRIBUTE: &str = "attribute";
/// A function declaration — what an `#[ignore]` is allowed to decorate.
const FUNCTION_ITEM: &str = "function_item";
/// A `//` comment, including the `///` doc form, which may sit between an
/// attribute and the function it belongs to.
const LINE_COMMENT: &str = "line_comment";
/// A `/* */` comment, in the same position.
const BLOCK_COMMENT: &str = "block_comment";
/// A bare identifier, which is how an attribute's path and a `cfg_attr`'s
/// operands both appear.
const IDENTIFIER: &str = "identifier";

/// The tree-sitter field naming an `attribute`'s `= "..."` operand.
const VALUE_FIELD: &str = "value";
/// The tree-sitter field naming a `function_item`'s identifier.
const NAME_FIELD: &str = "name";

/// The attribute that removes a test from every default run.
const IGNORE_ATTRIBUTE: &str = "ignore";
/// The conditional form. It would smuggle an `ignore` past a scan that only
/// looks for the bare attribute, so finding one mentioning `ignore` is an
/// error rather than a skip this module reports.
const CFG_ATTR_ATTRIBUTE: &str = "cfg_attr";
/// The plain conditional-compilation attribute. A predicate that no
/// configuration can satisfy deletes the test it decorates outright, which
/// is a skip the registry never sees.
const CFG_ATTRIBUTE: &str = "cfg";
/// The attribute that makes a function a test.
const TEST_ATTRIBUTE: &str = "test";
/// `cfg` predicate that holds when at least one operand does.
const ANY_PREDICATE: &str = "any";
/// `cfg` predicate that holds when every operand does.
const ALL_PREDICATE: &str = "all";
/// `cfg` predicate that inverts its single operand.
const NOT_PREDICATE: &str = "not";
/// A parenthesised operand list, as tree-sitter-rust reports it.
const TOKEN_TREE: &str = "token_tree";

/// Directory names never scanned: build output, corpus clones, and installed
/// dependencies. Anything beginning with `.` is skipped as well.
const EXCLUDED_DIRECTORIES: [&str; 3] = ["target", "node_modules", "coverage"];

/// The `.rs` extension, the only files this scan parses.
const RUST_EXTENSION: &str = "rs";

/// One `#[ignore]`d test, located and quoted.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IgnoredTest {
    /// Workspace-relative path of the file declaring it, `/`-separated.
    pub file: String,
    /// The test function's name.
    pub test: String,
    /// The `#[ignore = "..."]` reason as the compiler sees it — escapes and
    /// line continuations resolved. Empty when the attribute carries none.
    pub reason: String,
}

/// Every `#[ignore]`d test in the workspace, ordered by file then test name.
///
/// # Errors
///
/// Returns an error when a source file cannot be read or parsed, when an
/// `#[ignore]` decorates something other than a function, or when a
/// `#[cfg_attr(..)]` mentions `ignore`.
pub fn ignored_tests() -> Result<Vec<IgnoredTest>> {
    let root = repo_root();
    let mut found = Vec::new();
    for path in rust_sources(&root)? {
        let file = workspace_relative(&root, &path);
        let source = fs::read_to_string(&path)
            .with_context(|| format!("unreadable Rust source: {}", path.display()))?;
        found.extend(ignored_tests_in(&source, &file)?);
    }
    found.sort();
    Ok(found)
}

/// Every `#[ignore]`d test declared by one Rust source, attributed to `file`.
///
/// # Errors
///
/// Returns an error when `source` does not parse, when an `#[ignore]`
/// decorates something other than a function, or when a `#[cfg_attr(..)]`
/// mentions `ignore`.
pub fn ignored_tests_in(source: &str, file: &str) -> Result<Vec<IgnoredTest>> {
    let grammar = RustParser::new().grammar();
    let tree = parse_source(RUST_LANGUAGE_ID, &grammar, source.as_bytes())
        .with_context(|| format!("unparsable Rust source: {file}"))?;
    let mut found = Vec::new();
    visit(tree.root_node(), source, file, &mut found)?;
    Ok(found)
}

/// Walks every named node, recording the skips each `attribute_item` implies.
fn visit(node: Node<'_>, source: &str, file: &str, found: &mut Vec<IgnoredTest>) -> Result<()> {
    if node.kind() == ATTRIBUTE_ITEM {
        record(node, source, file, found)?;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, source, file, found)?;
    }
    Ok(())
}

/// Records one attribute when it is an `#[ignore]`, rejecting the
/// conditional form outright.
fn record(item: Node<'_>, source: &str, file: &str, found: &mut Vec<IgnoredTest>) -> Result<()> {
    let Some(attribute) = child_of_kind(item, ATTRIBUTE) else {
        return Ok(());
    };
    let path = attribute
        .named_child(0)
        .map(|node| text(node, source))
        .unwrap_or_default();
    if path == CFG_ATTR_ATTRIBUTE && mentions_ignore(attribute, source) {
        bail!(
            "{file}: `#[cfg_attr(.., {IGNORE_ATTRIBUTE})]` hides a skip from the \
             [TEST-SELECTION-SKIP] gate. State the skip as a plain `#[ignore = \"..\"]`."
        );
    }
    if path == CFG_ATTRIBUTE && deletes_a_test(item, attribute, source) {
        bail!(
            "{file}: `#[{CFG_ATTRIBUTE}(..)]` on `{}` can be satisfied by no \
             configuration, so the test is compiled by nothing and runs \
             nowhere. That is a skip the [TEST-SELECTION-SKIP] registry never \
             sees. State it as a plain `#[{IGNORE_ATTRIBUTE} = \"..\"]`.",
            decorated_function(item, source, file)?
        );
    }
    if path != IGNORE_ATTRIBUTE {
        return Ok(());
    }
    found.push(ignored_test(item, attribute, source, file)?);
    Ok(())
}

/// Every `#[cfg]`-decorated `#[test]` in the workspace, as
/// `(file, test, condition)`.
///
/// # Errors
///
/// Returns an error when a source file cannot be read or parsed.
pub fn conditional_tests() -> Result<Vec<(String, String, String)>> {
    let root = repo_root();
    let mut found = Vec::new();
    for path in rust_sources(&root)? {
        let file = workspace_relative(&root, &path);
        let source = fs::read_to_string(&path)
            .with_context(|| format!("unreadable Rust source: {}", path.display()))?;
        found.extend(conditional_tests_in(&source, &file)?);
    }
    found.sort();
    Ok(found)
}

/// Every `#[cfg]`-decorated `#[test]` declared by one Rust source.
///
/// # Errors
///
/// Returns an error when `source` does not parse.
pub fn conditional_tests_in(source: &str, file: &str) -> Result<Vec<(String, String, String)>> {
    let grammar = RustParser::new().grammar();
    let tree = parse_source(RUST_LANGUAGE_ID, &grammar, source.as_bytes())
        .with_context(|| format!("unparsable Rust source: {file}"))?;
    let mut found = Vec::new();
    visit_conditionals(tree.root_node(), source, file, &mut found);
    Ok(found)
}

/// Records every `cfg` attribute that decorates a test function.
fn visit_conditionals(
    node: Node<'_>,
    source: &str,
    file: &str,
    found: &mut Vec<(String, String, String)>,
) {
    if node.kind() == ATTRIBUTE_ITEM
        && attribute_path(node, source).as_deref() == Some(CFG_ATTRIBUTE)
        && decorates_a_test(node, source)
    {
        let test = decorated_function(node, source, file).unwrap_or_default();
        found.push((file.to_owned(), test, text(node, source)));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit_conditionals(child, source, file, found);
    }
}

/// Whether this `cfg` deletes a test function outright.
///
/// Both halves have to hold. A `cfg` no configuration satisfies is only a
/// silent skip when what it decorates is a test; and a `cfg` on a test is
/// perfectly ordinary when some configuration satisfies it — a
/// platform-specific or feature-gated test is not a hidden skip, and must
/// not be reported as one.
fn deletes_a_test(item: Node<'_>, attribute: Node<'_>, source: &str) -> bool {
    let unsatisfiable = predicates(attribute)
        .first()
        .is_some_and(|(name, operands)| {
            evaluate(&text(*name, source), *operands, source) == Some(false)
        });
    unsatisfiable && decorates_a_test(item, source)
}

/// Whether the attribute run `item` belongs to also carries `#[test]`.
fn decorates_a_test(item: Node<'_>, source: &str) -> bool {
    let mut sibling = item.next_named_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            ATTRIBUTE_ITEM => {
                if attribute_path(node, source).as_deref() == Some(TEST_ATTRIBUTE) {
                    return true;
                }
                sibling = node.next_named_sibling();
            }
            LINE_COMMENT | BLOCK_COMMENT => sibling = node.next_named_sibling(),
            _ => return false,
        }
    }
    false
}

/// The name of one `attribute_item`'s attribute.
fn attribute_path(item: Node<'_>, source: &str) -> Option<String> {
    let attribute = child_of_kind(item, ATTRIBUTE)?;
    attribute.named_child(0).map(|node| text(node, source))
}

/// Each predicate inside a node's operand list, as its name and its own
/// operand list.
fn predicates(node: Node<'_>) -> Vec<(Node<'_>, Option<Node<'_>>)> {
    child_of_kind(node, TOKEN_TREE)
        .map(nested_predicates)
        .unwrap_or_default()
}

/// The operand list belonging to the predicate at `at`, when it has one.
fn operands_after<'tree>(children: &[Node<'tree>], at: usize) -> Option<Node<'tree>> {
    children
        .get(at.saturating_add(1))
        .filter(|next| next.kind() == TOKEN_TREE)
        .copied()
}

/// Evaluates a `cfg` predicate as far as it goes without knowing the
/// configuration. `None` means the answer depends on one.
fn evaluate(name: &str, operands: Option<Node<'_>>, source: &str) -> Option<bool> {
    let inner = operands.map(nested_predicates).unwrap_or_default();
    let mut values = inner
        .iter()
        .map(|(id, args)| evaluate(&text(*id, source), *args, source));
    match name {
        ANY_PREDICATE => any_of(&mut values),
        ALL_PREDICATE => all_of(&mut values),
        NOT_PREDICATE => values.next().flatten().map(|held| !held),
        _ => None,
    }
}

/// The predicates directly inside an operand list.
fn nested_predicates<'tree>(tree: Node<'tree>) -> Vec<(Node<'tree>, Option<Node<'tree>>)> {
    let mut cursor = tree.walk();
    let children: Vec<Node<'tree>> = tree.named_children(&mut cursor).collect();
    children
        .iter()
        .enumerate()
        .filter(|(_, child)| child.kind() != TOKEN_TREE)
        .map(|(at, child)| (*child, operands_after(&children, at)))
        .collect()
}

/// `any(..)`: true once one operand is, false only when every operand is
/// definitely false — which `any()` over no operands at all vacuously is.
fn any_of(values: &mut dyn Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut depends_on_configuration = false;
    for value in values {
        match value {
            Some(true) => return Some(true),
            None => depends_on_configuration = true,
            Some(false) => {}
        }
    }
    (!depends_on_configuration).then_some(false)
}

/// `all(..)`: false once one operand is, true only when every operand is
/// definitely true — which `all()` over no operands vacuously is.
fn all_of(values: &mut dyn Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut depends_on_configuration = false;
    for value in values {
        match value {
            Some(false) => return Some(false),
            None => depends_on_configuration = true,
            Some(true) => {}
        }
    }
    (!depends_on_configuration).then_some(true)
}

/// Builds the record for one confirmed `#[ignore]`.
fn ignored_test(
    item: Node<'_>,
    attribute: Node<'_>,
    source: &str,
    file: &str,
) -> Result<IgnoredTest> {
    Ok(IgnoredTest {
        file: file.to_owned(),
        test: decorated_function(item, source, file)?,
        reason: attribute
            .child_by_field_name(VALUE_FIELD)
            .map(|value| literal_value(&text(value, source)))
            .unwrap_or_default(),
    })
}

/// True when any identifier under `attribute` is `ignore`.
fn mentions_ignore(attribute: Node<'_>, source: &str) -> bool {
    let mut cursor = attribute.walk();
    let named: Vec<Node<'_>> = attribute.named_children(&mut cursor).collect();
    named.iter().any(|child| {
        (child.kind() == IDENTIFIER && text(*child, source) == IGNORE_ATTRIBUTE)
            || mentions_ignore(*child, source)
    })
}

/// The name of the function `item` decorates. Outer attributes are siblings,
/// so the owner is the next item once further attributes and the doc comments
/// interleaved with them are stepped over.
fn decorated_function(item: Node<'_>, source: &str, file: &str) -> Result<String> {
    let mut sibling = item.next_named_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            ATTRIBUTE_ITEM | LINE_COMMENT | BLOCK_COMMENT => sibling = node.next_named_sibling(),
            FUNCTION_ITEM => return named_child_text(node, source, file),
            other => {
                bail!("{file}: `#[{IGNORE_ATTRIBUTE}]` decorates a `{other}`, not a test function")
            }
        }
    }
    bail!("{file}: `#[{IGNORE_ATTRIBUTE}]` decorates nothing")
}

/// The `name` field of a `function_item`.
fn named_child_text(function: Node<'_>, source: &str, file: &str) -> Result<String> {
    function
        .child_by_field_name(NAME_FIELD)
        .map(|name| text(name, source))
        .ok_or_else(|| anyhow::anyhow!("{file}: an ignored function declares no name"))
}

/// The first child of `node` with the given kind.
fn child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == kind);
    found
}

/// The source text a node spans.
fn text(node: Node<'_>, source: &str) -> String {
    source
        .get(node.start_byte()..node.end_byte())
        .unwrap_or_default()
        .to_owned()
}

/// The value of a Rust string literal, as the compiler sees it.
fn literal_value(literal: &str) -> String {
    match literal.strip_prefix('r') {
        Some(raw) => raw.trim_matches('#').trim_matches('"').to_owned(),
        None => unescape(literal.trim_matches('"')),
    }
}

/// Resolves the escapes an `#[ignore]` reason can carry, including the line
/// continuation `\<newline>`, which drops the newline and the indentation
/// that follows it. Without it a reason wrapped across lines would carry the
/// leading whitespace of every continuation into the text being matched.
fn unescape(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(character) = chars.next() {
        match (character, chars.peek()) {
            ('\\', Some('n')) => push_escaped(&mut out, &mut chars, '\n'),
            ('\\', Some('t')) => push_escaped(&mut out, &mut chars, '\t'),
            ('\\', Some('\n')) => skip_continuation(&mut chars),
            ('\\', Some(_)) => out.extend(chars.next()),
            _ => out.push(character),
        }
    }
    out
}

/// Consumes the escaped character and pushes what it stands for.
fn push_escaped(out: &mut String, chars: &mut Peekable<Chars<'_>>, resolved: char) {
    let _escaped = chars.next();
    out.push(resolved);
}

/// Consumes a line continuation: the newline and all whitespace after it.
fn skip_continuation(chars: &mut Peekable<Chars<'_>>) {
    while chars.peek().is_some_and(|next| next.is_whitespace()) {
        let _whitespace = chars.next();
    }
}

/// Every `.rs` file under `root`, excluding build output and dependencies.
fn rust_sources(root: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    collect_rust_sources(root, &mut found)?;
    found.sort();
    Ok(found)
}

/// Depth-first accumulation of `.rs` paths under `directory`.
fn collect_rust_sources(directory: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("unreadable directory: {}", directory.display()))?;
    for entry in entries {
        let path = entry?.path();
        match (path.is_dir(), is_scanned(&path), is_rust_source(&path)) {
            (true, true, _) => collect_rust_sources(&path, found)?,
            (false, _, true) => found.push(path),
            _ => {}
        }
    }
    Ok(())
}

/// False for build output, dependency trees, and every dotted directory.
fn is_scanned(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    !name.starts_with('.') && !EXCLUDED_DIRECTORIES.contains(&name)
}

/// True for a `.rs` file.
fn is_rust_source(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == RUST_EXTENSION)
}

/// `path` relative to the workspace root, `/`-separated on every platform so
/// the curated set in the contract test reads the same everywhere.
fn workspace_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

mod liveness;

pub use liveness::{feature_liveness_pins, feature_liveness_pins_in};

#[cfg(test)]
mod tests;
