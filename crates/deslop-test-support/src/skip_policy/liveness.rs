//! [TEST-SELECTION] Which cargo features the compiled test targets prove
//! are enabled.
//!
//! [`super::conditional_tests`] finds every `#[cfg]` on a test, but a
//! feature predicate is invisible to a static scan: whether
//! `feature = "profiling"` holds is decided by the command that runs the
//! suite, not by the source. Reading the command back out of the
//! Makefile would only move the guess.
//!
//! A test target can answer it directly. An unconditional `#[test]` that
//! asserts `cfg!(feature = "..")` is compiled into every build of that
//! target and fails in any run where the feature is off, so its presence
//! is the proof the gated test beside it is actually being compiled. This
//! module locates those pins so `skip_policy_contract.rs` can require one
//! for every feature a gated test depends on.

use std::fs;

use anyhow::{Context, Result};
use deslop_core::lang::{rust_lang::RustParser, shared::parse_source, LanguageParser};
use tree_sitter::Node;

use super::{
    attribute_path, repo_root, rust_sources, text, workspace_relative, ATTRIBUTE_ITEM,
    BLOCK_COMMENT, CFG_ATTRIBUTE, FUNCTION_ITEM, IDENTIFIER, LINE_COMMENT, RUST_LANGUAGE_ID,
    TEST_ATTRIBUTE, TOKEN_TREE,
};

/// The `cfg!` macro: the only form that reports a feature's state at run
/// time rather than deciding compilation.
const CFG_MACRO: &str = "cfg";
/// The `cfg` predicate naming a cargo feature.
const FEATURE_PREDICATE: &str = "feature";
/// A macro call, as tree-sitter-rust reports it.
const MACRO_INVOCATION: &str = "macro_invocation";
/// A quoted string, which is how a feature name is spelled.
const STRING_LITERAL: &str = "string_literal";
/// The quoted bytes inside a `string_literal`.
const STRING_CONTENT: &str = "string_content";

/// Every `(file, feature)` an unconditionally-compiled test asserts is
/// enabled, ordered by file then feature.
///
/// # Errors
///
/// Returns an error when a source file cannot be read or parsed.
pub fn feature_liveness_pins() -> Result<Vec<(String, String)>> {
    let root = repo_root();
    let mut found = Vec::new();
    for path in rust_sources(&root)? {
        let file = workspace_relative(&root, &path);
        let source = fs::read_to_string(&path)
            .with_context(|| format!("unreadable Rust source: {}", path.display()))?;
        found.extend(feature_liveness_pins_in(&source, &file)?);
    }
    found.sort();
    found.dedup();
    Ok(found)
}

/// Every `(file, feature)` one Rust source pins, attributed to `file`.
///
/// # Errors
///
/// Returns an error when `source` does not parse.
pub fn feature_liveness_pins_in(source: &str, file: &str) -> Result<Vec<(String, String)>> {
    let grammar = RustParser::new().grammar();
    let tree = parse_source(RUST_LANGUAGE_ID, &grammar, source.as_bytes())
        .with_context(|| format!("unparsable Rust source: {file}"))?;
    let mut found = Vec::new();
    visit(tree.root_node(), source, file, &mut found);
    Ok(found)
}

/// Records the features pinned by every unconditional test function.
fn visit(node: Node<'_>, source: &str, file: &str, found: &mut Vec<(String, String)>) {
    if node.kind() == FUNCTION_ITEM && is_unconditional_test(node, source) {
        found.extend(
            pinned_features(node, source)
                .into_iter()
                .map(|feature| (file.to_owned(), feature)),
        );
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, source, file, found);
    }
}

/// Whether `function` carries `#[test]` and no `#[cfg]`.
///
/// A `#[cfg]`-gated pin proves nothing: it is compiled under exactly the
/// configuration it claims to be checking, so it can never fail.
fn is_unconditional_test(function: Node<'_>, source: &str) -> bool {
    let mut is_test = false;
    let mut sibling = function.prev_named_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            ATTRIBUTE_ITEM => match attribute_path(node, source).as_deref() {
                Some(CFG_ATTRIBUTE) => return false,
                Some(TEST_ATTRIBUTE) => is_test = true,
                _ => {}
            },
            LINE_COMMENT | BLOCK_COMMENT => {}
            _ => break,
        }
        sibling = node.prev_named_sibling();
    }
    is_test
}

/// Every feature named by a `cfg!(feature = "..")` anywhere in a
/// function's body.
fn pinned_features(function: Node<'_>, source: &str) -> Vec<String> {
    let mut found = Vec::new();
    collect_pinned_features(function, source, &mut found);
    found
}

/// Walks `node`, recording each `cfg!` operand list's feature names.
fn collect_pinned_features(node: Node<'_>, source: &str, found: &mut Vec<String>) {
    if node.kind() == TOKEN_TREE && belongs_to_the_cfg_macro(node, source) {
        found.extend(feature_operands(node, source));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_pinned_features(child, source, found);
    }
}

/// Whether this operand list is the one a `cfg!` call was given.
///
/// Two shapes reach here and both are read. A `cfg!` written as an
/// expression — `if cfg!(feature = "..")` — is a `macro_invocation`
/// whose macro name is `cfg`. A `cfg!` written *inside* another macro is
/// not: tree-sitter reports a macro's body as a flat token run and
/// re-parses nothing in it, so `assert!(cfg!(feature = ".."), "..")`
/// yields the bare identifier `cfg` followed by its operand list, with no
/// `macro_invocation` node anywhere. That second shape is the one a pin
/// actually takes, and keying only on the first found nothing at all.
fn belongs_to_the_cfg_macro(tree: Node<'_>, source: &str) -> bool {
    let called_as_an_expression = tree.parent().is_some_and(|parent| {
        parent.kind() == MACRO_INVOCATION
            && macro_name(parent, source).as_deref() == Some(CFG_MACRO)
    });
    let called_inside_another_macro = tree
        .prev_named_sibling()
        .is_some_and(|node| node.kind() == IDENTIFIER && text(node, source) == CFG_MACRO);
    called_as_an_expression || called_inside_another_macro
}

/// The name of the macro `invocation` calls.
fn macro_name(invocation: Node<'_>, source: &str) -> Option<String> {
    invocation
        .named_child(0)
        .filter(|node| node.kind() == IDENTIFIER)
        .map(|node| text(node, source))
}

/// Each feature name in one `cfg!` operand list: every string literal
/// following a `feature` identifier, in order.
///
/// Read positionally rather than by matching `feature = "x"` as text —
/// tree-sitter reports a macro's operands as a flat token run, and the
/// `=` between them carries no structure to key on.
fn feature_operands(tree: Node<'_>, source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut cursor = tree.walk();
    let mut expecting_name = false;
    for token in tree.named_children(&mut cursor) {
        match token.kind() {
            IDENTIFIER if text(token, source) == FEATURE_PREDICATE => expecting_name = true,
            STRING_LITERAL if expecting_name => {
                expecting_name = false;
                found.extend(string_content(token, source));
            }
            _ => {}
        }
    }
    found
}

/// The bytes inside a string literal, without its quotes.
fn string_content(literal: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = literal.walk();
    let content = literal
        .named_children(&mut cursor)
        .find(|node| node.kind() == STRING_CONTENT);
    content.map(|node| text(node, source))
}

#[cfg(test)]
mod tests;
