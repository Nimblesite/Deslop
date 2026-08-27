//! Records the identifiers an assignment writes to. Authored for gh #460.

use tree_sitter::Node;

/// Collects every identifier named on the left of an assignment.
pub fn assignment_targets(node: Node<'_>, source: &[u8], out: &mut Vec<String>) {
    if node.kind() == "assignment" {
        if let Some(left) = node.child_by_field_name("left") {
            collect_identifiers(left, source, out);
        }
    }
}
