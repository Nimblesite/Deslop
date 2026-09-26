//! [CORPUS-PRECISION] Is this ranked cluster a table of literals rather than
//! logic?
//!
//! A block that only enumerates values — a font-metrics table, a list of
//! state names, a run of constants — repeats because it lists rows, not
//! because anybody copied logic. Extracting it changes nothing, so it must
//! not sit at the head of the report ahead of copy-paste a reader can act on.
//! The engine's noise filters drop such clusters ([CLONE-NOISE-CONSTANT-TABLE],
//! [CLONE-NOISE-DART-DATA-TABLE-LITERAL]); this predicate is how the corpus
//! gate checks that they did.
//!
//! The verdict is read from the occurrence's parse tree and never from
//! characters of raw source text. The rule this replaces counted digits and
//! separators over non-whitespace characters against a 0.6 boundary — text
//! pattern matching on source, which `AGENTS.md` prohibits — and it was wrong
//! in both directions: a table of string literals scored 0.00 and walked past,
//! while ordinary logic under a version-matrix comment scored 0.74 and was
//! reported as data.
//!
//! **This oracle is deliberately independent of the engine's own filters.**
//! Reusing `cluster_filters`' predicate would make the gate circular — it would
//! assert only that the engine agrees with itself, and could never catch the
//! engine being wrong.
//!
//! The judging lives here, under test, rather than inside the corpus suite,
//! where every test is `#[ignore]`d and needs a pinned clone before it can run:
//! nothing there asserted what the check said, and it spent that time reporting
//! every breach as "0 occurrences" because it read a field the report does not
//! carry.

use anyhow::{anyhow, Result};
use deslop_core::{lang::shared::parse_source, pipeline::default_parsers};
use serde_json::Value;
use tree_sitter::{Language, Node};

use crate::corpus::{field_u64, Failure, OCCURRENCE_COUNT};

mod grammar;

use grammar::{TableGrammar, TABLE_GRAMMARS};

/// Number of top-ranked clusters subjected to the language-agnostic precision
/// checks. Ranking is the product, so the head of the report is where a false
/// positive does the most damage.
pub const RANKED_HEAD: usize = 10;

/// Entries a span must hold before it reads as a table rather than an
/// incidental pair of literals. Two is a coincidence in ordinary logic —
/// `return [0, 1]` — so the floor sits above it and the predicate errs
/// towards calling a span logic.
pub const MIN_TABLE_ENTRIES: usize = 3;

/// How much of the offending text the failure quotes, so a reader can see
/// what was ranked without opening the report.
const SNIPPET_CHARS: usize = 70;

/// The check id this module reports under.
const CHECK: &str = "data_table_rank";

/// [CORPUS-PRECISION] Judges one ranked cluster, given the language of the
/// repository and the text of the cluster's first occurrence. `position` is
/// the cluster's zero-based index in the report.
///
/// Returns the failure when the cluster is a table of literals ranked as
/// logic, and `None` when it is ordinary duplicated code.
///
/// No cluster is exempted by a category it declares. [RANK-CATEGORY] is
/// explicit that a detection-time finding kind "is not carried as
/// clone-cluster similarity metadata", and the wire model carries no such
/// field, so the exemption this check used to apply could never open: it read
/// `category`, got nothing, and told the reader the cluster was "categorised
/// `absent`" — a fact about a field that does not exist.
///
/// # Errors
///
/// Propagates the predicate's own error: a language with no curated table
/// grammar or no registered parser, or a parse failure.
pub fn data_table_failure(
    language: &str,
    position: usize,
    cluster: &Value,
    text: &str,
) -> Result<Option<Failure>> {
    if !occurrence_is_a_literal_table(language, text)? {
        return Ok(None);
    }
    // One-based, like the report's own `rank` field: a message that says
    // "rank 9" about the tenth cluster sends the reader to the wrong finding.
    let rank = position.saturating_add(1);
    Ok(Some(Failure::new(
        CHECK,
        format!(
            "rank {rank}: cluster of {} occurrences is a table of literals, not \
             extractable logic, yet it is visible in the ranked head — a noise \
             filter missed it and it outranks copy-paste a reader can act on. \
             Snippet: {}",
            field_u64(cluster, OCCURRENCE_COUNT),
            snippet(text),
        ),
    )))
}

/// The leading characters of `text`, on one line.
fn snippet(text: &str) -> String {
    text.chars()
        .take(SNIPPET_CHARS)
        .collect::<String>()
        .replace('\n', " ")
}

/// [CORPUS-PRECISION] True when `text`, parsed as `language`, is a table of
/// literals rather than logic.
///
/// Two shapes qualify, both read from the parse tree and never from source
/// text: a collection whose every element is a literal, or a run of
/// declarations whose every value is a literal. A span that calls anything is
/// code however many literals it holds. Comments and string *contents* are
/// nodes the walk never descends into, so neither can sway the verdict.
///
/// # Errors
///
/// Returns an error when `language` has no curated table grammar or no
/// registered parser, or when the parse fails. A language the gate cannot
/// judge must fail loudly rather than answer "not a table" for every
/// occurrence it is handed — that is how a gate quietly stops asserting
/// anything.
pub fn occurrence_is_a_literal_table(language: &str, text: &str) -> Result<bool> {
    let grammar = table_grammar(language)?;
    let (parser_id, parser_grammar) = parser_for(language)?;
    let tree = parse_source(parser_id, &parser_grammar, text.as_bytes())?;
    let root = tree.root_node();
    if holds_logic(root, grammar) {
        return Ok(false);
    }
    Ok(holds_a_literal_collection(root, grammar) || holds_a_literal_declaration_run(root, grammar))
}

/// The curated table grammar for `language`.
fn table_grammar(language: &str) -> Result<&'static TableGrammar> {
    TABLE_GRAMMARS
        .iter()
        .find(|(id, _)| *id == language)
        .map(|(_, grammar)| grammar)
        .ok_or_else(|| {
            anyhow!(
                "language `{language}` carries no curated table grammar here — curate one \
                 rather than letting the data-table precision gate pass without judging \
                 anything"
            )
        })
}

/// The engine's `'static` id and grammar for `language`, which
/// [`parse_source`] needs.
fn parser_for(language: &str) -> Result<(&'static str, Language)> {
    default_parsers()
        .iter()
        .find(|parser| parser.id() == language)
        .map(|parser| (parser.id(), parser.grammar()))
        .ok_or_else(|| anyhow!("no registered parser for language `{language}`"))
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

#[cfg(test)]
mod tests;
