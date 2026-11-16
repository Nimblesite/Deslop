//! [CLONE-NOISE-CONSTANT-TABLE] Rust constant tables are compared by what
//! each declaration binds.
//!
//! The table rule hides two runs of literal constants that share only their
//! shape. Its escape hatch keeps a copy visible, and a renamed constant is
//! still a copy: `const USAGE: &str = "…"` duplicated as `const HELP` binds
//! the same 1.5 KB of text twice, so a substantial initialiser is compared
//! on its own bytes and the new name cannot convict the copy as unrelated.
//!
//! A bare scalar is not substance. `const MAX_RETRY_ATTEMPTS: u32 = 3` and
//! `const WORKER_THREADS: u32 = 3` coincide by accident; only the name says
//! what each means. So substance decides: members that bind any string or
//! compound initialiser are compared on those alone, their scalars ignored,
//! and names decide only when a member binds lone scalars and nothing else.

use tree_sitter::Node;

use super::{node_intersects_range, parse_for, trimmed_snippet_range, Snippet};
use crate::ast::named_children;

/// Each table is compared with its immediate neighbour.
const ADJACENT_TABLES: usize = 2;

/// Literal kinds whose value alone cannot show that two constants are one.
const SCALAR_LITERAL_KINDS: [&str; 4] = [
    "integer_literal",
    "float_literal",
    "boolean_literal",
    "char_literal",
];

/// The prefix operator kind that wraps a negative scalar (`-1`).
const UNARY_EXPRESSION_KIND: &str = "unary_expression";

/// Whether any two neighbouring members bind different declarations.
/// A member that cannot be parsed keeps the cluster visible.
pub(super) fn rust_table_values_differ(snippets: &[Snippet<'_>]) -> bool {
    let tables: Option<Vec<Vec<&[u8]>>> = snippets.iter().map(rust_table_values).collect();
    tables.is_some_and(|tables| {
        tables
            .windows(ADJACENT_TABLES)
            .any(|pair| matches!(pair, [left, right] if left != right))
    })
}

/// The comparison bytes of the top-level `const` / `static` items the member
/// covers, in source order ([`compared_nodes`]).
fn rust_table_values<'a>(snippet: &Snippet<'a>) -> Option<Vec<&'a [u8]>> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let bound: Vec<(Node<'_>, Node<'_>)> = named_children(tree.root_node())
        .into_iter()
        .filter(|item| node_intersects_range(*item, range))
        .filter_map(|item| Some((item, item.child_by_field_name("value")?)))
        .collect();
    Some(
        compared_nodes(&bound)
            .into_iter()
            .filter_map(|node| snippet.source.get(node.byte_range()))
            .collect(),
    )
}

/// Every substantial initialiser, or, when the member binds lone scalars
/// only, every whole declaration.
fn compared_nodes<'tree>(bound: &[(Node<'tree>, Node<'tree>)]) -> Vec<Node<'tree>> {
    let substantial: Vec<Node<'tree>> = bound
        .iter()
        .map(|(_, value)| *value)
        .filter(|value| !is_scalar(*value))
        .collect();
    if substantial.is_empty() {
        bound.iter().map(|(item, _)| *item).collect()
    } else {
        substantial
    }
}

/// Whether `value` is a scalar literal, optionally negated.
fn is_scalar(value: Node<'_>) -> bool {
    let literal = if value.kind() == UNARY_EXPRESSION_KIND {
        named_children(value).into_iter().next()
    } else {
        Some(value)
    };
    literal.is_some_and(|node| SCALAR_LITERAL_KINDS.contains(&node.kind()))
}
