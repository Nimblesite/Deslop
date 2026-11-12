//! [CLONE-NOISE-CONSTANT-TABLE] Rust constant tables are compared by the
//! values they bind, not by their names.
//!
//! The table rule hides two runs of literal constants that share only their
//! shape. Its escape hatch keeps a copy visible, and a renamed constant is
//! still a copy: `const USAGE: &str = "…"` duplicated as `const HELP` binds
//! the same 1.5 KB of text twice. Comparing the full declaration bytes let
//! the new name convict the copy as unrelated data and drop it from the
//! report; comparing the AST-delimited initialisers does not.

use super::{node_intersects_range, parse_for, trimmed_snippet_range, Snippet};
use crate::ast::named_children;

/// Each table is compared with its immediate neighbour.
const ADJACENT_TABLES: usize = 2;

/// Whether any two neighbouring members bind different initialiser bytes.
/// A member that cannot be parsed keeps the cluster visible.
pub(super) fn rust_table_values_differ(snippets: &[Snippet<'_>]) -> bool {
    let tables: Option<Vec<Vec<&[u8]>>> = snippets.iter().map(rust_table_values).collect();
    tables.is_some_and(|tables| {
        tables
            .windows(ADJACENT_TABLES)
            .any(|pair| matches!(pair, [left, right] if left != right))
    })
}

/// The initialiser bytes of every top-level `const` / `static` the member
/// covers, in source order.
fn rust_table_values<'a>(snippet: &Snippet<'a>) -> Option<Vec<&'a [u8]>> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let source = snippet.source;
    Some(
        named_children(tree.root_node())
            .into_iter()
            .filter(|item| node_intersects_range(*item, range))
            .filter_map(|item| item.child_by_field_name("value"))
            .filter_map(|value| source.get(value.byte_range()))
            .collect(),
    )
}
