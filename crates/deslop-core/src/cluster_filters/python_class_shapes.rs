//! Python class-shape cluster filters split out of `python.rs` to keep
//! every file under the 500-LOC budget.
//!
//! Issues addressed (see parent `mod.rs` header):
//! - **#115a** [CLONE-NOISE-PY-STRENUM-CLASS-SHAPE] — every `class X(StrEnum)`
//!   carries a docstring + assignment-only body. After identifier
//!   normalisation those bodies collapse and the enums cluster as
//!   duplicates, but each enum is a closed discriminator the program
//!   depends on by name.
//! - **#115b** [CLONE-NOISE-PY-PYDANTIC-PARTIAL] — Pydantic's `XCreate`
//!   model and matching `XUpdate` mirror cluster after normalisation
//!   because every `XUpdate` field reuses the `XCreate` field name with
//!   `T | None = None`. Pydantic has no native `PartialModel`, so the
//!   mirror is mandated by the framework and not extractable.

use tree_sitter::Node;

use super::{
    enclosing_kind, is_multi_member_language_cluster, parse_for, trimmed_snippet_range, Snippet,
};
use crate::ast::ByteRange;

/// Detects **issue #115a**: every cluster occurrence is a Python
/// `class X(StrEnum)` (or `class X(str, Enum)`) declaration whose body
/// consists only of a docstring and member assignments. Returning true
/// drops the cluster — distinct enum vocabularies are not duplication.
pub(super) fn is_strenum_class_shape_cluster(snippets: &[Snippet<'_>]) -> bool {
    if !is_multi_member_language_cluster(snippets, "python") {
        return false;
    }
    snippets.iter().all(is_strenum_class_snippet)
}

/// Returns true when `snippet` covers exactly one `class_definition`
/// whose superclass list contains `StrEnum` (or `str` + `Enum`) and
/// whose body is docstring + assignments only.
fn is_strenum_class_snippet(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let Some(class) = sole_class_in_range(tree.root_node(), range) else {
        return false;
    };
    class_has_strenum_base(class, snippet.source) && class_body_is_docstring_and_assignments(class)
}

/// Walks `root` collecting `class_definition` nodes fully enclosed by
/// `range` and returns the only one, or `None`.
fn sole_class_in_range(root: Node<'_>, range: ByteRange) -> Option<Node<'_>> {
    let mut classes = Vec::new();
    collect_classes_in_range(root, range, &mut classes);
    let [class] = classes.as_slice() else {
        return None;
    };
    Some(*class)
}

/// Walks the tree appending `class_definition` nodes fully contained in
/// `range`. Stops descending once a class is captured so nested classes
/// are not double-counted.
fn collect_classes_in_range<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    out: &mut Vec<Node<'tree>>,
) {
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return;
    }
    if node.kind() == "class_definition"
        && node.start_byte() >= range.start
        && node.end_byte() <= range.end
    {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_classes_in_range(child, range, out);
    }
}

/// Returns true when `class.superclasses` contains an `StrEnum`
/// identifier, or the pair `str` + `Enum`.
fn class_has_strenum_base(class: Node<'_>, source: &[u8]) -> bool {
    let Some(supers) = class.child_by_field_name("superclasses") else {
        return false;
    };
    let mut cursor = supers.walk();
    let mut has_str = false;
    let mut has_enum = false;
    for child in supers.named_children(&mut cursor) {
        let Some(name) = identifier_bytes(child, source) else {
            continue;
        };
        if name == b"StrEnum" {
            return true;
        }
        has_str = has_str || name == b"str";
        has_enum = has_enum || name == b"Enum";
    }
    has_str && has_enum
}

/// Returns the bytes of `node` when it (or its sole child) is an
/// `identifier`; supports bare-name superclasses only.
fn identifier_bytes<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    if node.kind() != "identifier" {
        return None;
    }
    source.get(node.start_byte()..node.end_byte())
}

/// Returns true when `class.body` (a `block`) contains only an optional
/// leading docstring `expression_statement` whose inner is a `string`,
/// followed by `expression_statement`s wrapping `assignment` nodes.
fn class_body_is_docstring_and_assignments(class: Node<'_>) -> bool {
    let Some(body) = class.child_by_field_name("body") else {
        return false;
    };
    let mut cursor = body.walk();
    let mut saw_assignment = false;
    for child in body.named_children(&mut cursor) {
        match expression_statement_inner_kind(child) {
            Some("string") => {}
            Some("assignment") => saw_assignment = true,
            _ => return false,
        }
    }
    saw_assignment
}

/// Returns the kind of the inner expression for an `expression_statement`
/// node, or `None` when the input is not such a statement or has no
/// inner child.
fn expression_statement_inner_kind(node: Node<'_>) -> Option<&'static str> {
    if node.kind() != "expression_statement" {
        return None;
    }
    let mut cursor = node.walk();
    let inner = node.named_children(&mut cursor).next()?;
    Some(inner.kind())
}

/// Detects **issue #115b**: a cluster of Pydantic `BaseModel` subclasses
/// where every member is a class whose every field is annotated `T | None`
/// (or `Optional[T]`) with a `None` default. Returns true to drop the
/// cluster — the `XUpdate` mirror is mandated by Pydantic's PATCH
/// semantics, not extractable duplication.
pub(super) fn is_pydantic_partial_update_cluster(snippets: &[Snippet<'_>]) -> bool {
    if !is_multi_member_language_cluster(snippets, "python") {
        return false;
    }
    let shapes: Option<Vec<PartialUpdateShape>> =
        snippets.iter().map(partial_update_shape).collect();
    let Some(shapes) = shapes else { return false };
    shapes.iter().all(|shape| shape.is_partial_basemodel)
}

/// Per-member partial-update detection result.
struct PartialUpdateShape {
    /// True when this occurrence is a single `class X(BaseModel)` whose
    /// every field is `T | None`-style optional with a `None` default.
    is_partial_basemodel: bool,
}

/// Returns the shape for `snippet` when its range falls inside a
/// `class_definition` inheriting from `BaseModel`. Accepts both whole-
/// class fingerprints and sub-class fingerprints that cover only a run
/// of field declarations (which is the common case after Type-2
/// normalisation collapses each `T | None = None` field).
fn partial_update_shape(snippet: &Snippet<'_>) -> Option<PartialUpdateShape> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let class = enclosing_or_sole_class(tree.root_node(), range)?;
    if !class_has_base_named(class, snippet.source, b"BaseModel") {
        return None;
    }
    Some(PartialUpdateShape {
        is_partial_basemodel: class_body_is_all_optional_fields(class, snippet.source),
    })
}

/// Returns the `class_definition` either fully containing or fully
/// contained by `range`, preferring the enclosing one when the range
/// is a sub-class fingerprint.
fn enclosing_or_sole_class(root: Node<'_>, range: ByteRange) -> Option<Node<'_>> {
    enclosing_kind(root, range, &["class_definition"]).or_else(|| sole_class_in_range(root, range))
}

/// Returns true when `class.superclasses` contains the bare identifier
/// `needle`.
fn class_has_base_named(class: Node<'_>, source: &[u8], needle: &[u8]) -> bool {
    let Some(supers) = class.child_by_field_name("superclasses") else {
        return false;
    };
    let mut cursor = supers.walk();
    let found = supers
        .named_children(&mut cursor)
        .any(|child| identifier_bytes(child, source) == Some(needle));
    found
}

/// Returns true when every body statement is `field: T | None = None`
/// (or `field: Optional[T] = None`) and at least one such field exists.
fn class_body_is_all_optional_fields(class: Node<'_>, source: &[u8]) -> bool {
    let Some(body) = class.child_by_field_name("body") else {
        return false;
    };
    let mut cursor = body.walk();
    let mut saw_field = false;
    for child in body.named_children(&mut cursor) {
        if !child_is_optional_field_or_docstring(child, source, &mut saw_field) {
            return false;
        }
    }
    saw_field
}

/// Returns true when `child` is either a docstring `expression_statement`
/// or an optional-field assignment statement. Sets `saw_field` to true
/// for the latter.
fn child_is_optional_field_or_docstring(
    child: Node<'_>,
    source: &[u8],
    saw_field: &mut bool,
) -> bool {
    if child.kind() != "expression_statement" {
        return false;
    }
    let mut inner_cursor = child.walk();
    let Some(inner) = child.named_children(&mut inner_cursor).next() else {
        return false;
    };
    if inner.kind() == "string" {
        return true;
    }
    if inner.kind() != "assignment" || !assignment_is_optional_field(inner, source) {
        return false;
    }
    *saw_field = true;
    true
}

/// Returns true when `assignment` declares `name: T | None = None` or
/// `name: Optional[T] = None`.
fn assignment_is_optional_field(assignment: Node<'_>, source: &[u8]) -> bool {
    let Some(left) = assignment.child_by_field_name("left") else {
        return false;
    };
    if left.kind() != "identifier" {
        return false;
    }
    let Some(type_node) = assignment.child_by_field_name("type") else {
        return false;
    };
    let Some(right) = assignment.child_by_field_name("right") else {
        return false;
    };
    if !is_none_literal(right, source) {
        return false;
    }
    type_is_optional_annotation(type_node, source)
}

/// Returns true when `node` is the bare `None` identifier.
fn is_none_literal(node: Node<'_>, source: &[u8]) -> bool {
    matches!(node.kind(), "none") || source.get(node.start_byte()..node.end_byte()) == Some(b"None")
}

/// Returns true when `type_node` is `T | None`, `None | T`, or
/// `Optional[T]`.
fn type_is_optional_annotation(type_node: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = type_node.walk();
    let Some(inner) = type_node.named_children(&mut cursor).next() else {
        return false;
    };
    annotation_inner_is_optional(inner, source)
}

/// Inspects an annotation expression for the `T | None` / `Optional[T]`
/// shape. Handles both PEP 604 unions and the classic `Optional` form.
fn annotation_inner_is_optional(node: Node<'_>, source: &[u8]) -> bool {
    if node.kind() == "binary_operator" {
        return binary_union_has_none(node, source);
    }
    if node.kind() == "subscript" {
        let Some(value) = node.child_by_field_name("value") else {
            return false;
        };
        return source.get(value.start_byte()..value.end_byte()) == Some(b"Optional");
    }
    false
}

/// Returns true for `T | None` (or `None | T`) PEP 604 unions.
fn binary_union_has_none(node: Node<'_>, source: &[u8]) -> bool {
    let Some(operator_byte) = node
        .child_by_field_name("operator")
        .and_then(|operator| source.get(operator.start_byte()..operator.end_byte()))
    else {
        return false;
    };
    if operator_byte != b"|" {
        return false;
    }
    let left = node.child_by_field_name("left");
    let right = node.child_by_field_name("right");
    side_is_none(left, source) || side_is_none(right, source)
}

/// Returns true when one side of a union annotation is the `None` type.
fn side_is_none(side: Option<Node<'_>>, source: &[u8]) -> bool {
    let Some(node) = side else { return false };
    if matches!(node.kind(), "none") {
        return true;
    }
    source.get(node.start_byte()..node.end_byte()) == Some(b"None")
}
