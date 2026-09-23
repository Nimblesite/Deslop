//! Tree-sitter node lookups shared by the gates that read Rust sources off
//! the tree ([TEST-SELECTION], [TEST-SELECTION-SKIP]).

use tree_sitter::Node;

/// The first direct named child of `node` with `kind`.
pub(crate) fn child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == kind);
    found
}

/// The source slice `node` spans.
pub(crate) fn text(node: Node<'_>, source: &str) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}
