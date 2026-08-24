//! Whole-body statement census over syntax nodes
//! ([AUTOFIX-EXTRACT-TESTING], [PIPELINE-CLUSTER-EXACT]).
//!
//! A refactor golden is only provably correct if something checks that
//! the plan consumed the *whole* duplicated body. A plan computed from a
//! nested sub-view applies cleanly, names its helper after its own
//! cluster id, and produces a buffer that is self-consistent in every
//! way a byte comparison can see — it just leaves the statements it did
//! not cover duplicated. Counting them is what tells the two apart.
//!
//! The count is taken over **syntax nodes**, never over source text. A
//! statement spelled inside a comment or a string literal never forms a
//! node of its own, so it cannot inflate the count; and node text is
//! compared with whitespace collapsed, so the re-indentation an
//! extracted helper applies to its body cannot make a statement vanish.

use std::path::Path;

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::lang::LanguageParser;

/// Collapses every whitespace run to a single space so two spellings of
/// one statement — the original and the re-indented copy inside the
/// extracted helper — compare equal.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parses `source` with the language registered for `file_name`.
fn parse(source: &str, file_name: &str) -> Result<(tree_sitter::Tree, Box<dyn LanguageParser>)> {
    let language = deslop_core::refactor::parser_for_path(Path::new(file_name))
        .ok_or_else(|| anyhow!("no parser registered for {file_name}"))?;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language.grammar())
        .with_context(|| format!("grammar for {file_name}"))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow!("parsing {file_name} produced no tree"))?;
    Ok((tree, language))
}

/// Counts the named syntax nodes of `source` whose own text is
/// `statement`, comparing with whitespace collapsed.
///
/// A matched node is counted once and not descended into, so a wrapper
/// node that spans exactly the same bytes as its only child (a Python
/// `expression_statement` around its `assignment`, for instance) counts
/// once rather than twice.
///
/// # Errors
///
/// Returns an error when `file_name` has no registered language, its
/// grammar cannot be applied, or the source does not parse.
pub(crate) fn statement_count(source: &str, statement: &str, file_name: &str) -> Result<usize> {
    let (tree, _language) = parse(source, file_name)?;
    let wanted = collapsed(statement);
    ensure!(!wanted.is_empty(), "the census statement must not be empty");
    let mut cursor = tree.walk();
    let mut pending = vec![tree.root_node()];
    let mut count: usize = 0;
    while let Some(node) = pending.pop() {
        let text = source.get(node.byte_range()).unwrap_or_default();
        if node.is_named() && collapsed(text) == wanted {
            count = count.saturating_add(1);
        } else {
            pending.extend(node.named_children(&mut cursor));
        }
    }
    Ok(count)
}

/// Asserts the refactor consumed the *whole* duplicated body: every
/// statement listed occurs as a syntax node exactly twice before the
/// rewrite and exactly once after it.
///
/// # Errors
///
/// Returns an error when a statement is not duplicated in `source`
/// (fixture drift) or does not appear exactly once in `applied` (the
/// plan covered only part of the duplication).
pub(crate) fn assert_body_deduplicated(
    source: &str,
    applied: &str,
    statements: &[&str],
    file_name: &str,
) -> Result<()> {
    ensure!(
        !statements.is_empty(),
        "a census with no statements asserts nothing"
    );
    for statement in statements {
        let before = statement_count(source, statement, file_name)?;
        ensure!(
            before == 2,
            "fixture drift: `{statement}` must occur as a syntax node twice in \
             the source, found {before}",
        );
        let after = statement_count(applied, statement, file_name)?;
        ensure!(
            after == 1,
            "`{statement}` occurs {after} times after the refactor, not once; \
             the plan covered only part of the duplicated body, so it was \
             computed from a nested view of the duplication",
        );
    }
    Ok(())
}
