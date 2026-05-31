//! Dart-specific false-positive filters.

use super::{enclosing_kind, parse_for, Snippet};

/// Returns true for repeated Dart class-field declarations. Field lists inside
/// a class encode data shape, not extractable duplicate logic.
pub(super) fn is_dart_class_field_declaration_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2 && snippets.iter().all(is_class_field_declaration)
}

/// Returns true when a snippet is a declaration inside a Dart class member.
fn is_class_field_declaration(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root = tree.root_node();
    enclosing_kind(root, snippet.range, &["class_declaration"]).is_some()
        && enclosing_kind(root, snippet.range, &["class_member"]).is_some()
        && enclosing_kind(root, snippet.range, &["declaration"]).is_some()
}
