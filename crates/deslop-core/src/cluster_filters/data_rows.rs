//! Runs of data rows ([CLONE-NOISE-DATA-ROWS]).
//!
//! A table repeats one row shape with different values in every row, so a
//! window of rows normalises to the same subtree as any other window of the
//! same table, or of an unrelated one. Nothing can be extracted from two
//! windows that share only the shape of a row. A copied table still surfaces
//! through the verbatim escape hatch ([CLONE-NOISE-VERBATIM-SUBGROUP]), and a
//! row holding logic is never a data row, so this filter fails open. A call's
//! arguments and a record's fields are not rows: they say what one thing is.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{
    calls::is_statement_shape, node_intersects_range, node_search::node_enclosed_by_range,
    parse_for, raw_snippet_texts_differ, trimmed_snippet_range, Snippet,
};
use crate::ast::{named_children, ByteRange};

/// The fewest rows that make a run.
const MIN_ROWS: usize = 2;
/// Members sharing one in this many of the smaller one's strings are copies.
const COPY_OVERLAP_DENOMINATOR: usize = 2;

/// Returns true when every member is a run of data rows, the members are not
/// all the same bytes, and they are not all copies of one another.
pub(super) fn is_data_row_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= MIN_ROWS
        && snippets.iter().all(covers_data_rows)
        && raw_snippet_texts_differ(snippets)
        && !members_are_copies(snippets)
}

/// True when every pair of members holds mostly the same strings: a table
/// copied and then edited, which the exact-bytes escape hatch cannot see.
pub(super) fn members_are_copies(snippets: &[Snippet<'_>]) -> bool {
    let payloads: Vec<BTreeSet<&[u8]>> = snippets.iter().map(member_payloads).collect();
    payloads.iter().enumerate().all(|(index, left)| {
        payloads
            .iter()
            .skip(index.saturating_add(1))
            .all(|right| mostly_shared(left, right))
    })
}

/// True when the smaller member's strings are mostly in the larger: a table
/// grown or trimmed after copying shares little of the larger one's strings.
fn mostly_shared(left: &BTreeSet<&[u8]>, right: &BTreeSet<&[u8]>) -> bool {
    let shared = left.intersection(right).count();
    shared.saturating_mul(COPY_OVERLAP_DENOMINATOR) >= left.len().min(right.len())
}

/// The string literals inside one member's range.
fn member_payloads<'src>(snippet: &Snippet<'src>) -> BTreeSet<&'src [u8]> {
    let mut payloads = BTreeSet::new();
    if let Some(tree) = parse_for(snippet) {
        collect_strings(
            tree.root_node(),
            snippet.range,
            snippet.source,
            &mut payloads,
        );
    }
    payloads
}

/// True when the snippet covers whole, different sibling rows of one shape,
/// none holding logic, that hold a string.
fn covers_data_rows(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let end_inclusive = range.end.saturating_sub(1).max(range.start);
    let Some(parent) = tree
        .root_node()
        .descendant_for_byte_range(range.start, end_inclusive)
    else {
        return false;
    };
    is_data_row_run(parent, range, snippet.source)
}

/// True when `parent`'s children in `range` are data rows, or `parent` is a
/// wrapper around a node whose are: a whole table is a declaration around its
/// rows. The search stops at logic, so a function is never read as its body.
fn is_data_row_run(parent: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    if parent.kind().contains("argument") {
        return false;
    }
    let rows: Vec<Node<'_>> = named_children(parent)
        .into_iter()
        .filter(|child| node_intersects_range(*child, range) && !is_trivia(*child))
        .map(unwrap_row)
        .collect();
    let is_run = rows.len() >= MIN_ROWS
        && rows.iter().all(|row| node_enclosed_by_range(*row, range))
        && !rows
            .iter()
            .any(|row| is_record_field(*row) || row_holds_logic(*row))
        && one_row_shape(&rows, source)
        && rows_differ(&rows, source)
        && holds_a_string(&rows, source);
    is_run
        || (!is_logic_kind(parent.kind())
            && largest_child(&rows).is_some_and(|child| is_data_row_run(child, range, source)))
}

/// The row spanning the most bytes.
fn largest_child<'tree>(rows: &[Node<'tree>]) -> Option<Node<'tree>> {
    rows.iter()
        .copied()
        .max_by_key(|row| row.end_byte().saturating_sub(row.start_byte()))
}

/// True for what a run may hold beside its rows: comments, and a closing
/// `return` of plain names, which says nothing about the rows above it.
fn is_trivia(node: Node<'_>) -> bool {
    node.kind().contains("comment") || (node.kind() == "return_statement" && returns_names(node))
}

/// True when a subtree holds only names and `nil`-like leaves.
fn returns_names(node: Node<'_>) -> bool {
    named_children(node).into_iter().all(|child| {
        child.kind() == "identifier"
            || child.kind() == "nil"
            || (child.kind() == "expression_list" && returns_names(child))
    })
}

/// Strips single-child wrappers such as an expression statement.
fn unwrap_row(node: Node<'_>) -> Node<'_> {
    match named_children(node).as_slice() {
        [only] if is_statement_shape(node.kind()) => unwrap_row(*only),
        _ => node,
    }
}

/// True when every row has the same kind, width and leading name.
fn one_row_shape(rows: &[Node<'_>], source: &[u8]) -> bool {
    let shapes: BTreeSet<_> = rows.iter().map(|row| row_shape(*row, source)).collect();
    shapes.len() == 1
}

/// A row's kind, its cell count, and the name a call or attribute row leads with.
fn row_shape<'src>(row: Node<'_>, source: &'src [u8]) -> (String, usize, &'src [u8]) {
    let cells = named_children(row);
    let head = cells
        .first()
        .filter(|cell| is_name(**cell))
        .and_then(|cell| source.get(cell.start_byte()..cell.end_byte()))
        .unwrap_or_default();
    (row.kind().to_owned(), cells.len(), head)
}

/// True for a node that names a callee: an identifier or a member path.
fn is_name(node: Node<'_>) -> bool {
    let kind = node.kind();
    kind.contains("identifier") || kind == "selector_expression" || kind == "member_expression"
}

/// True for a named field: fields of one record are not rows of a table.
fn is_record_field(row: Node<'_>) -> bool {
    let kind = row.kind();
    kind.contains("keyed") || kind.contains("pair") || kind.contains("field")
}

/// True when a row is itself computed or holds a cell that is. The row's own
/// statement wrapper is not logic: a Dart `expect(...)` row keeps one.
fn row_holds_logic(row: Node<'_>) -> bool {
    is_logic_kind(row.kind()) || named_children(row).into_iter().any(holds_logic)
}

/// True when a row's subtree computes or branches instead of holding values.
fn holds_logic(node: Node<'_>) -> bool {
    is_statement_shape(node.kind())
        || is_logic_kind(node.kind())
        || named_children(node).into_iter().any(holds_logic)
}

/// Function bodies, blocks and computed expressions, in every grammar.
fn is_logic_kind(kind: &str) -> bool {
    kind.contains("function")
        || kind.contains("lambda")
        || kind.contains("closure")
        || kind.contains("block")
        || kind.contains("binary")
        || kind.contains("conditional")
        || kind.contains("ternary")
        || kind.contains("assignment")
        || kind == "func_literal"
}

/// True when no two rows are the same bytes: a repeated row is a paste.
fn rows_differ(rows: &[Node<'_>], source: &[u8]) -> bool {
    let texts: BTreeSet<_> = rows
        .iter()
        .filter_map(|row| source.get(row.start_byte()..row.end_byte()))
        .collect();
    texts.len() == rows.len()
}

/// True when the rows hold a string, the payload that makes them data.
fn holds_a_string(rows: &[Node<'_>], source: &[u8]) -> bool {
    let mut payloads = BTreeSet::new();
    for row in rows {
        let range = ByteRange {
            start: row.start_byte(),
            end: row.end_byte(),
        };
        collect_strings(*row, range, source, &mut payloads);
    }
    !payloads.is_empty()
}

/// Gathers the outermost string literals of `node` that overlap `range`.
fn collect_strings<'src>(
    node: Node<'_>,
    range: ByteRange,
    source: &'src [u8],
    out: &mut BTreeSet<&'src [u8]>,
) {
    if !node_intersects_range(node, range) {
        return;
    }
    if is_string_literal(node.kind()) {
        let _ = source
            .get(node.start_byte()..node.end_byte())
            .map(|text| out.insert(text));
        return;
    }
    for child in named_children(node) {
        collect_strings(child, range, source, out);
    }
}

/// String literal kinds across the grammars.
fn is_string_literal(kind: &str) -> bool {
    kind.ends_with("string") || kind.ends_with("string_literal") || kind == "template_string"
}
