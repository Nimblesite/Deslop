//! Unwraps a call argument's payload node. Authored for gh #460.

use tree_sitter::Node;

/// Returns the value a keyword argument carries, or the node itself.
pub fn argument_payload(node: Node<'_>) -> Node<'_> {
    if node.kind() == "keyword_argument" {
        if let Some(value) = node.child_by_field_name("value") {
            return value;
        }
    }
    node
}
