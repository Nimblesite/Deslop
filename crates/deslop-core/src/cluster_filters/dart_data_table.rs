//! Dart collection-literal data-table detection
//! ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]).
//!
//! The class-field registry filter ([`super::dart`]) only fires for
//! runs of declarations inside a `class_body`. A top-level
//! `List<Highlight> highlights = [ Highlight(...), Highlight(...), … ]`
//! collection literal has no enclosing `class_body`, so those data tables
//! fell straight through the noise pass at full ranking weight and
//! dominated the report. This predicate recognises them so the
//! [RANK-CATEGORY] policy can demote (default) or drop them.
//!
//! Unlike #169 — which *hides* the cluster — this returns a category so a
//! demoted data table still appears in the report, labelled. The verbatim
//! escape hatch (`raw_snippet_texts_differ`) is identical: a byte-for-byte
//! copy is genuine duplication and must surface as `logic`.

use tree_sitter::Node;

use super::{
    enclosing_kind, node_contains_kind, node_intersects_range, parse_for, raw_snippet_texts_differ,
    Snippet,
};

/// Tree-sitter node kinds for Dart collection literals whose elements may
/// form a data table.
const COLLECTION_LITERAL_KINDS: &[&str] = &["list_literal", "set_or_map_literal"];

/// Returns true when `snippets` form a Dart collection-literal data table
/// ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]): every member covers a run of
/// sibling elements inside a `list_literal` / `set_or_map_literal`, every
/// covered element is a pure data shape (constructor/record/map/literal,
/// no embedded closure body), and at least two members differ in raw bytes
/// so a verbatim-copied table still surfaces as genuine duplication.
pub(super) fn is_dart_collection_data_table_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2
        && snippets.iter().all(covers_only_data_elements)
        && raw_snippet_texts_differ(snippets)
}

/// Returns true when the snippet's range sits inside a Dart collection
/// literal and every element it covers is a pure data shape. A run that
/// covers any closure-bearing element falls through and keeps clustering as
/// logic — closures encode behaviour, not a data row.
fn covers_only_data_elements(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let Some(literal) = enclosing_kind(tree.root_node(), snippet.range, COLLECTION_LITERAL_KINDS)
    else {
        return false;
    };
    let mut cursor = literal.walk();
    let mut covered = 0_usize;
    for element in literal.named_children(&mut cursor) {
        if !node_intersects_range(element, snippet.range) {
            continue;
        }
        if !is_data_element(element) {
            return false;
        }
        covered = covered.saturating_add(1);
    }
    covered >= 1
}

/// Returns true when a collection-literal child is a pure data element.
///
/// `type_arguments` (the `<Highlight>` before the `[`) is structural, not a
/// row, so it is skipped rather than rejected — a run that brushes against
/// it stays a data table. A row is data when it carries no executable body:
/// a constructor / factory call (`call_expression`), a `record_literal`, a
/// map `pair`, or any other literal-only element. An element nesting a
/// `function_body` or `function_expression` is logic (`() { … }` / `=>`
/// callbacks) and disqualifies the run, matching the #169 boundary.
fn is_data_element(element: Node<'_>) -> bool {
    if element.kind() == "type_arguments" {
        return true;
    }
    !node_contains_kind(element, "function_body")
        && !node_contains_kind(element, "function_expression")
}
