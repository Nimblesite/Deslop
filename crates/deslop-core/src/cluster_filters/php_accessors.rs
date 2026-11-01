//! PHP class scaffolding ([CLONE-NOISE-PHP-ACCESSORS]).
//!
//! Properties, a constructor that stores its parameters, and getters that
//! return them all normalise to one shape in every small value class. They
//! carry no logic to extract, so a run of nothing else is not a clone. Any
//! statement outside these forms fails the proof and the cluster stays.

use tree_sitter::Node;

use super::{
    enclosing_kind, is_multi_member_language_cluster, node_intersects_range, parse_for,
    raw_snippet_texts_differ, trimmed_snippet_range, Snippet,
};
use crate::ast::{named_children, ByteRange};

const LANGUAGE: &str = "php";
const CLASS_BODY: [&str; 1] = ["declaration_list"];

/// Returns true when every member covers only class scaffolding.
pub(super) fn is_accessor_scaffold_cluster(snippets: &[Snippet<'_>]) -> bool {
    is_multi_member_language_cluster(snippets, LANGUAGE)
        && raw_snippet_texts_differ(snippets)
        && snippets.iter().all(covers_scaffold)
}

/// True when the class members the snippet touches are all scaffolding.
fn covers_scaffold(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let Some(body) = enclosing_kind(tree.root_node(), range, &CLASS_BODY)
        .or_else(|| sole_class_body(tree.root_node(), range))
    else {
        return false;
    };
    let members: Vec<Node<'_>> = named_children(body)
        .into_iter()
        .filter(|member| node_intersects_range(*member, range) && member.kind() != "comment")
        .collect();
    !members.is_empty()
        && members
            .iter()
            .all(|member| is_scaffold_member(*member, snippet.source))
}

/// The body of the one class a whole-file member holds.
fn sole_class_body(root: Node<'_>, range: ByteRange) -> Option<Node<'_>> {
    let classes: Vec<Node<'_>> = named_children(root)
        .into_iter()
        .filter(|node| node_intersects_range(*node, range) && node.kind() == "class_declaration")
        .collect();
    match classes.as_slice() {
        [only] => only.child_by_field_name("body"),
        _ => None,
    }
}

/// A property, or a method made only of stored parameters and returned fields.
fn is_scaffold_member(member: Node<'_>, source: &[u8]) -> bool {
    match member.kind() {
        "property_declaration" => true,
        "method_declaration" => member.child_by_field_name("body").is_some_and(|body| {
            named_children(body)
                .into_iter()
                .filter(|statement| statement.kind() != "comment")
                .all(|statement| is_scaffold_statement(statement, source))
        }),
        _ => false,
    }
}

/// `$this->a = $a;`, `parent::__construct($a);` or `return $this->a;`.
fn is_scaffold_statement(statement: Node<'_>, source: &[u8]) -> bool {
    let inner = named_children(statement);
    match (statement.kind(), inner.as_slice()) {
        ("return_statement", [value]) => is_own_field(*value, source),
        ("expression_statement", [expression]) => match expression.kind() {
            "assignment_expression" => stores_parameter(*expression, source),
            "scoped_call_expression" => passes_only_variables(*expression),
            _ => false,
        },
        _ => false,
    }
}

/// True for `$this->name`.
fn is_own_field(node: Node<'_>, source: &[u8]) -> bool {
    node.kind() == "member_access_expression"
        && node
            .child_by_field_name("object")
            .and_then(|object| source.get(object.start_byte()..object.end_byte()))
            .is_some_and(|object| object == b"$this")
}

/// True for `$this->name = $parameter`.
fn stores_parameter(assignment: Node<'_>, source: &[u8]) -> bool {
    assignment
        .child_by_field_name("left")
        .is_some_and(|left| is_own_field(left, source))
        && assignment
            .child_by_field_name("right")
            .is_some_and(|right| right.kind() == "variable_name")
}

/// True when a static call's arguments are all plain variables.
fn passes_only_variables(call: Node<'_>) -> bool {
    call.child_by_field_name("arguments").is_some_and(|arguments| {
        named_children(arguments).into_iter().all(|argument| {
            matches!(named_children(argument).as_slice(), [value] if value.kind() == "variable_name")
        })
    })
}
