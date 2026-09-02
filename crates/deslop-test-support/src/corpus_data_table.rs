//! [CORPUS-PRECISION] Is a ranked occurrence a table of literals?
//!
//! A repeated data structure is not extractable logic: there is no shared
//! control flow and nothing a reader could hoist. The engine's noise
//! filters drop such clusters ([CLONE-NOISE-CONSTANT-TABLE],
//! [CLONE-NOISE-DART-DATA-TABLE-LITERAL]); this predicate is how the
//! corpus gate checks that they did.
//!
//! The shipped rule counted character classes in raw source text — digits
//! and literal separators over non-whitespace characters, against a 0.6
//! boundary. That is text pattern matching on source code, which
//! `AGENTS.md` prohibits outright, and it is the surviving arm of the
//! defect gh #401 removed from the sibling `must_not_rank_first` check. It
//! answered wrong in both directions: a table of string literals scored
//! 0.00 and walked past, while ordinary logic under a version-matrix
//! comment scored 0.74 and was reported as data (gh #452).
//!
//! **This oracle is deliberately independent of the engine's own filters.**
//! Reusing `cluster_filters`' predicate would make the gate circular — it
//! would assert only that the engine agrees with itself, and could never
//! catch the engine being wrong, which is the gh #439 failure mode on a
//! different check.

use anyhow::{anyhow, Result};
use deslop_core::{lang::shared::parse_source, pipeline::default_parsers};
use tree_sitter::Node;

use crate::enclosure::Span;

mod grammar;

use grammar::{TableGrammar, TABLE_GRAMMARS};

/// Entries a span must hold before it reads as a table rather than an
/// incidental pair of literals. Two is a coincidence in ordinary logic —
/// `return [0, 1]` — so the floor sits above it and the predicate errs
/// towards calling a span logic.
const MIN_TABLE_ENTRIES: usize = 3;

/// True when the occurrence `span` delimits in `source` is a table of
/// literals rather than logic.
///
/// Two shapes qualify, both read from the AST and never from source text:
/// a collection whose every element is a literal, or a run of declarations
/// whose every value is a literal. Comments and string *contents* are
/// nodes we never descend into, so neither can sway the verdict.
///
/// # Errors
///
/// Returns an error when `span` lies outside `source`, when `language` has
/// no registered parser, when it carries no curated table grammar, or when
/// the parse fails.
pub fn occurrence_is_a_literal_table(language: &str, source: &str, span: &Span) -> Result<bool> {
    let grammar = TABLE_GRAMMARS
        .iter()
        .find(|(id, _)| *id == language)
        .map(|(_, grammar)| grammar)
        .ok_or_else(|| {
            anyhow!(
                "language `{language}` carries no curated table grammar here — curate one \
                 rather than letting the data-table precision gate pass without judging \
                 anything"
            )
        })?;
    let start = usize::try_from(span.start)?;
    let end = usize::try_from(span.end)?;
    let text = source
        .get(start..end)
        .ok_or_else(|| anyhow!("occurrence range {start}..{end} is outside the source"))?;
    let tree = parse_source(
        parser_id(language)?,
        &grammar_for(language)?,
        text.as_bytes(),
    )?;
    let root = tree.root_node();
    if holds_logic(root, grammar) {
        return Ok(false);
    }
    Ok(holds_a_literal_collection(root, grammar) || holds_a_literal_declaration_run(root, grammar))
}

/// True when the span calls anything. A span that merely *contains* a
/// literal array — an `ESLint` config object, a test setup block listing
/// module names — is code that holds data, not a table, and flagging it
/// fails a report whose ranking was correct.
fn holds_logic(root: Node<'_>, grammar: &TableGrammar) -> bool {
    descendants(root)
        .into_iter()
        .any(|node| grammar.logic.contains(&node.kind()))
}

/// True when any collection in the tree has at least [`MIN_TABLE_ENTRIES`]
/// named children and every one of them is a literal.
fn holds_a_literal_collection(root: Node<'_>, grammar: &TableGrammar) -> bool {
    descendants(root).into_iter().any(|node| {
        grammar.collections.contains(&node.kind()) && every_entry_is_literal(node, grammar)
    })
}

/// True when every element of `collection` is a literal and there are
/// enough of them to be a table.
fn every_entry_is_literal(collection: Node<'_>, grammar: &TableGrammar) -> bool {
    let entries = named_children(collection);
    entries.len() >= MIN_TABLE_ENTRIES
        && entries.into_iter().all(|entry| is_literal(entry, grammar))
}

/// True when the tree holds a run of at least [`MIN_TABLE_ENTRIES`]
/// declarations and *every* declaration in it takes a literal value. One
/// computed value takes the run out of the shape, exactly as
/// [CLONE-NOISE-CONSTANT-TABLE] requires.
fn holds_a_literal_declaration_run(root: Node<'_>, grammar: &TableGrammar) -> bool {
    let declarations: Vec<Node<'_>> = descendants(root)
        .into_iter()
        .filter(|node| grammar.declarations.contains(&node.kind()))
        .collect();
    declarations.len() >= MIN_TABLE_ENTRIES
        && declarations
            .into_iter()
            .all(|declaration| declares_a_literal(declaration, grammar))
}

/// True when a declaration's value slot — its last named child — is a
/// literal.
fn declares_a_literal(declaration: Node<'_>, grammar: &TableGrammar) -> bool {
    named_children(declaration)
        .last()
        .is_some_and(|value| is_literal(*value, grammar))
}

/// True when `node`, once single-child wrappers are unwrapped, is a
/// literal of this grammar.
fn is_literal(node: Node<'_>, grammar: &TableGrammar) -> bool {
    grammar
        .literals
        .contains(&unwrap_wrappers(node, grammar).kind())
}

/// Descends through single-named-child wrappers — Go's `expression_list`
/// and `literal_element`, PHP's `array_element_initializer` — stopping at
/// the first literal so a string's internal nodes are never reached.
fn unwrap_wrappers<'tree>(node: Node<'tree>, grammar: &TableGrammar) -> Node<'tree> {
    let mut current = node;
    while !grammar.literals.contains(&current.kind()) && current.named_child_count() == 1 {
        match current.named_child(0) {
            Some(child) => current = child,
            None => break,
        }
    }
    current
}

/// Every named child of `node`.
fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Every node in the tree, root included.
fn descendants(root: Node<'_>) -> Vec<Node<'_>> {
    let mut pending = vec![root];
    let mut found = Vec::new();
    while let Some(node) = pending.pop() {
        found.push(node);
        pending.extend(named_children(node));
    }
    found
}

/// The engine's grammar for `language`.
fn grammar_for(language: &str) -> Result<tree_sitter::Language> {
    default_parsers()
        .iter()
        .find(|parser| parser.id() == language)
        .map(|parser| parser.grammar())
        .ok_or_else(|| anyhow!("no registered parser for language `{language}`"))
}

/// The engine's `'static` id for `language`, which `parse_source` needs.
fn parser_id(language: &str) -> Result<&'static str> {
    default_parsers()
        .iter()
        .find(|parser| parser.id() == language)
        .map(|parser| parser.id())
        .ok_or_else(|| anyhow!("no registered parser for language `{language}`"))
}

#[cfg(test)]
mod tests;
