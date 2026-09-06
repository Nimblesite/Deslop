//! ORM-shaped Python cluster filters split out of `python.rs` to keep
//! every file under the 500-LOC budget.
//!
//! Issues addressed (see parent `mod.rs` header):
//! - [CLONE-NOISE-PY-KWARGS-CTOR] — ORM/dataclass/Pydantic
//!   kwargs-only constructor calls.
//! - [CLONE-NOISE-PY-MAPPED-COLUMN] — `SQLAlchemy`
//!   `Mapped[T] = mapped_column(...)` declarations.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{
    is_multi_member_language_cluster, language_cluster_shapes, node_search::KindSearch, parse_for,
    spans_multiple_files, trimmed_snippet_range, Snippet,
};
use crate::{
    ast::{named_children, ByteRange},
    state::FileId,
};

/// Detects ****: ORM / dataclass / Pydantic constructor calls
/// of the shape `ModelName(field1=val, field2=val, ...)`. Two members
/// from the same cluster passing different field-name sets cannot share
/// a refactor: each model declares its own required columns. The filter
/// fires only when at least one member uses a different keyword-name
/// set, so genuine copy-paste of one constructor stays visible.
pub(super) fn is_kwargs_only_constructor_cluster(snippets: &[Snippet<'_>]) -> bool {
    language_cluster_shapes(snippets, "python", kwargs_constructor_shape).is_some_and(|shapes| {
        spans_multiple_files(shapes.iter().map(|shape| shape.file_id))
            && kwargs_ctor_shapes_differ(&shapes)
    })
}

/// Per-member shape recorded for kwargs-only constructor clusters.
struct KwargsCtorShape {
    /// File providing this member (for cross-file uniqueness).
    file_id: FileId,
    /// Keyword argument names supplied to the constructor.
    keywords: BTreeSet<Vec<u8>>,
}

/// Returns the constructor shape for `snippet` when the reported range
/// contains exactly one `Capitalised(name=value, ...)` call with kwargs
/// only and no positional arguments. Surrounding statements
/// (assignment LHS, expression-statement wrapper) are allowed because
/// the fingerprint frequently covers the enclosing assignment.
fn kwargs_constructor_shape(snippet: &Snippet<'_>) -> Option<KwargsCtorShape> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let call = sole_class_constructor_call(tree.root_node(), range, snippet.source)?;
    let keywords = kwargs_only_keyword_set(call, snippet.source)?;
    Some(KwargsCtorShape {
        file_id: snippet.file_id,
        keywords,
    })
}

/// Returns the single class-constructor `call` contained in `range`, or
/// `None` when the range covers zero, more than one, or any non-class
/// call. The descent walks **down** from the root because the call sits
/// inside the snippet range (the fingerprint may have covered the
/// enclosing assignment statement).
fn sole_class_constructor_call<'tree>(
    root: Node<'tree>,
    range: ByteRange,
    source: &[u8],
) -> Option<Node<'tree>> {
    let constructors: Vec<Node<'tree>> = call_search(range)
        .nodes(root)
        .into_iter()
        .filter(|call| call_is_class_constructor(*call, source))
        .collect();
    let [call] = constructors.as_slice() else {
        return None;
    };
    Some(*call)
}

/// The search for every `call` node fully enclosed by `range`, nested
/// calls included.
fn call_search(range: ByteRange) -> KindSearch<impl Fn(&str) -> bool> {
    KindSearch::enclosed(range, |kind| kind == "call").with_nested_hits()
}

/// Returns true when `call.function` is a single capitalised identifier
/// — the heuristic for a class / model constructor.
fn call_is_class_constructor(call: Node<'_>, source: &[u8]) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "identifier" {
        return false;
    }
    source
        .get(function.start_byte()..function.end_byte())
        .and_then(|bytes| bytes.first().copied())
        .is_some_and(|first| first.is_ascii_uppercase())
}

/// Returns the set of keyword-argument names when the call's argument
/// list contains only `keyword_argument` children. Returns `None` when
/// any positional or splat argument is present.
fn kwargs_only_keyword_set(call: Node<'_>, source: &[u8]) -> Option<BTreeSet<Vec<u8>>> {
    let arguments = call.child_by_field_name("arguments")?;
    let mut keywords = BTreeSet::new();
    let mut saw_kwarg = false;
    for arg in named_children(arguments) {
        if arg.kind() != "keyword_argument" {
            return None;
        }
        let name = arg.child_by_field_name("name")?;
        let bytes = source.get(name.start_byte()..name.end_byte())?;
        let _inserted = keywords.insert(bytes.to_vec());
        saw_kwarg = true;
    }
    saw_kwarg.then_some(keywords)
}

/// Returns true when at least two members carry different keyword-name
/// sets — that's the signal that the cluster groups distinct models.
fn kwargs_ctor_shapes_differ(shapes: &[KwargsCtorShape]) -> bool {
    let Some(first) = shapes.first() else {
        return false;
    };
    shapes.iter().any(|shape| shape.keywords != first.keywords)
}

/// Detects ****: `SQLAlchemy` `name: Mapped[T] = mapped_column(...)`
/// declarations. Each ORM model declares its own columns, so members
/// sharing the same token alphabet (`Mapped`, `mapped_column`,
/// `ForeignKey`, `UUID`, ...) cluster lexically even though their
/// schemas differ. Fires when every member is either a single
/// `mapped_column(...)` declaration or a contiguous block of them, and
/// at least two members declare different attribute names.
pub(super) fn is_sqlalchemy_mapped_column_cluster(snippets: &[Snippet<'_>]) -> bool {
    if !is_multi_member_language_cluster(snippets, "python") {
        return false;
    }
    if snippets.iter().all(is_mapped_column_call_snippet) {
        return true;
    }
    let shapes: Option<Vec<MappedColumnShape>> = snippets.iter().map(mapped_column_shape).collect();
    let Some(shapes) = shapes else { return false };
    if !shapes.iter().all(|shape| !shape.column_names.is_empty()) {
        return false;
    }
    mapped_column_name_sets_differ(&shapes)
}

/// Returns true when the reported range is exactly one `mapped_column(...)`
/// call. Embedding recall can surface call nodes underneath the already
/// filtered declaration statements; those are the same ORM-schema noise.
fn is_mapped_column_call_snippet(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let Some(call) = sole_call_in_range(tree.root_node(), range) else {
        return false;
    };
    call_node_function_equals(call, snippet.source, b"mapped_column")
}

/// Returns the sole Python call fully contained in `range`.
fn sole_call_in_range(root: Node<'_>, range: ByteRange) -> Option<Node<'_>> {
    call_search(range).sole_node(root)
}

/// Per-member shape: set of `mapped_column`-bound attribute names
/// declared inside the reported range. Single-declaration members
/// carry exactly one name; block members carry the block's set.
struct MappedColumnShape {
    /// Attribute names assigned to `mapped_column(...)`.
    column_names: BTreeSet<Vec<u8>>,
}

/// Builds the [`MappedColumnShape`] for one member. Returns `None` when
/// any statement-shaped descendant fully contained in `range` is not a
/// `mapped_column` declaration.
fn mapped_column_shape(snippet: &Snippet<'_>) -> Option<MappedColumnShape> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let names = collect_mapped_column_attr_names(tree.root_node(), range, snippet.source)?;
    if names.is_empty() {
        return None;
    }
    Some(MappedColumnShape {
        column_names: names,
    })
}

/// Walks the tree, requiring every statement fully contained in `range`
/// be a `mapped_column` declaration. Returns the union of declared
/// attribute names, or `None` when an alien statement intrudes.
fn collect_mapped_column_attr_names(
    root: Node<'_>,
    range: ByteRange,
    source: &[u8],
) -> Option<BTreeSet<Vec<u8>>> {
    let mut names = BTreeSet::new();
    if mapped_column_walk(root, range, source, &mut names) {
        Some(names)
    } else {
        None
    }
}

/// Recursive helper for [`collect_mapped_column_attr_names`].
fn mapped_column_walk(
    node: Node<'_>,
    range: ByteRange,
    source: &[u8],
    out: &mut BTreeSet<Vec<u8>>,
) -> bool {
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return true;
    }
    if node.kind() == "expression_statement"
        && node.start_byte() >= range.start
        && node.end_byte() <= range.end
    {
        // A docstring is not a declaration. It only became reachable
        // here once an occurrence could span a whole module
        // ([FUSED-SHARED-SUBTREE] widened which view wins): a bare
        // string parses as an `expression_statement`, the walk read it
        // as an alien statement, and the whole ORM filter stopped
        // firing — so two modules declaring entirely different tables
        // surfaced as duplicate logic (gh #105). Docstrings say nothing
        // about whether the code around them is duplicated.
        if is_docstring_statement(node) {
            return true;
        }
        let Some(name) = mapped_column_declaration_name(node, source) else {
            return false;
        };
        let _inserted = out.insert(name);
        return true;
    }
    for child in named_children(node) {
        if !mapped_column_walk(child, range, source, out) {
            return false;
        }
    }
    true
}

/// True for an `expression_statement` that is nothing but a string —
/// a module, class or function docstring.
fn is_docstring_statement(node: Node<'_>) -> bool {
    matches!(named_children(node).as_slice(), [only] if only.kind() == "string")
}

/// Returns the LHS attribute name for an `attr: Mapped[T] = mapped_column(...)`
/// statement, or `None` when the statement is not that shape.
fn mapped_column_declaration_name(node: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    if node.kind() != "expression_statement" {
        return None;
    }
    let inner = node.named_child(0)?;
    if inner.kind() != "assignment" {
        return None;
    }
    if !assignment_value_calls(inner, source, b"mapped_column") {
        return None;
    }
    let left = inner.child_by_field_name("left")?;
    if left.kind() != "identifier" {
        return None;
    }
    source
        .get(left.start_byte()..left.end_byte())
        .map(<[u8]>::to_vec)
}

/// Returns true when the RHS of `assignment` is a direct call to a
/// function whose name matches `callee`.
fn assignment_value_calls(assignment: Node<'_>, source: &[u8], callee: &[u8]) -> bool {
    let Some(right) = assignment.child_by_field_name("right") else {
        return false;
    };
    if right.kind() != "call" {
        return false;
    }
    call_node_function_equals(right, source, callee)
}

/// Returns true when a call node invokes an identifier named `callee`.
fn call_node_function_equals(call: Node<'_>, source: &[u8], callee: &[u8]) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    function.kind() == "identifier"
        && source.get(function.start_byte()..function.end_byte()) == Some(callee)
}

/// Returns true when at least two members declare different column-name
/// sets — proves the cluster groups distinct ORM models.
fn mapped_column_name_sets_differ(shapes: &[MappedColumnShape]) -> bool {
    let Some(first) = shapes.first() else {
        return false;
    };
    shapes
        .iter()
        .any(|shape| shape.column_names != first.column_names)
}
