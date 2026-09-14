//! Module-level constant-table filter ([CLONE-NOISE-CONSTANT-TABLE]).
//!
//! A run of module-level `NAME = <literal>` declarations — a table of
//! SQL query strings, registry values, config defaults, or the
//! test-data blobs a test suite feeds its subject — normalises to the
//! same structural subtree as any other such run once identifiers,
//! literals and comments are stripped. Two unrelated tables then reach
//! `structural = 1.00` and cluster as duplicates. A table of distinct
//! named constants is data: there is no shared control flow and no
//! abstraction to hoist, so there is nothing a reader could extract.
//!
//! The rule is one rule, and only the *grammar* of "a top-level
//! constant declaration" is per-language. Python was the first language
//! it shipped for (#133); gh #362 is the same shape in Rust — two test
//! files whose `const NAME: &str = r"…";` runs share nothing but the
//! shape, reported as the largest single finding in the repository that
//! hosts them, and counted in `duplicated_loc` even after the ranking
//! policy demoted them. Neither the ≥3-file `[CLONE-NOISE-SCAFFOLDING]`
//! hide nor the single-file `[RANK-STRUCTURAL-ONLY]` declaration-family
//! hide covers a two-file spread, so the geometry fell through the gap
//! between them; the answer is not to widen either suppression — both
//! would then be free to eat a real two-file clone — but to recognise
//! the table for what it is, in whatever language it is written.
//!
//! Suppressed **only** when the members differ in raw bytes, so a
//! constants module copied verbatim into two files still surfaces as
//! genuine duplication. Any right-hand side that is not a plain literal
//! — a call, a name, an attribute, an interpolated string — takes the
//! member out of the shape and keeps the cluster visible.

use tree_sitter::Node;

use crate::ast::named_children;

use super::{
    node_intersects_range, parse_for, raw_snippet_texts_differ, trimmed_snippet_range, Snippet,
};

/// Returns true when every cluster member's matched range covers a run
/// of module-level constant declarations and at least two members
/// differ in raw bytes. Returning true drops the cluster — unrelated
/// data tables are not duplication.
pub(super) fn is_constant_table_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.len() >= 2
        && snippets.iter().all(covers_only_constants)
        && raw_snippet_texts_differ(snippets)
}

/// Returns true when the member's range lies at module top level and
/// every top-level item it covers is trivia or a constant declaration —
/// with at least one constant declaration present.
fn covers_only_constants(snippet: &Snippet<'_>) -> bool {
    let Some(grammar) = Grammar::of(snippet.language) else {
        return false;
    };
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root = tree.root_node();
    if root.kind() != grammar.module_kind {
        return false;
    }
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let covered = named_children(root)
        .into_iter()
        .filter(|child| node_intersects_range(*child, range))
        .map(grammar.classify);
    let mut saw_constant = false;
    for item in covered {
        match item {
            TopLevel::Trivia => {}
            TopLevel::ConstantDeclaration => saw_constant = true,
            TopLevel::Other => return false,
        }
    }
    saw_constant
}

/// How one top-level item contributes to the constant-table shape.
enum TopLevel {
    /// A comment, a docstring, or an attribute — inert framing that
    /// neither makes nor breaks the table.
    Trivia,
    /// A `NAME = <literal>` declaration: one table entry.
    ConstantDeclaration,
    /// Anything else — imports, functions, types, control flow, calls.
    /// Its presence means the range is not a pure constant table.
    Other,
}

/// The two grammar facts the shared rule needs from a language: what its
/// module root is called, and how to classify one of that root's
/// children. A language absent here can never match, which is what
/// keeps the walk off every cluster in every other language
/// ([CLONE-NOISE-REPARSE-CACHE]).
struct Grammar {
    /// Node kind of the parse tree's module root.
    module_kind: &'static str,
    /// Classifies one top-level child of that root.
    classify: fn(Node<'_>) -> TopLevel,
}

impl Grammar {
    /// The grammar description for `language`, or `None` when the
    /// language has no constant-table form defined yet.
    fn of(language: &str) -> Option<Self> {
        match language.as_bytes() {
            b"python" => Some(Self {
                module_kind: "module",
                classify: classify_python_item,
            }),
            b"rust" => Some(Self {
                module_kind: "source_file",
                classify: classify_rust_item,
            }),
            b"javascript" | b"typescript" | b"tsx" => Some(Self {
                module_kind: "program",
                classify: classify_ecmascript_item,
            }),
            _ => None,
        }
    }
}

/// Classifies one top-level `module` child in Python.
fn classify_python_item(node: Node<'_>) -> TopLevel {
    match node.kind() {
        "comment" => TopLevel::Trivia,
        "expression_statement" => classify_python_expression_statement(node),
        _ => TopLevel::Other,
    }
}

/// Classifies a Python `expression_statement`: a lone docstring is
/// trivia, a constant assignment is a table entry, everything else
/// (including a chained `a = 1; b = 2`) disqualifies the range so it
/// keeps clustering.
fn classify_python_expression_statement(node: Node<'_>) -> TopLevel {
    let Some(inner) = sole_named_child(node) else {
        return TopLevel::Other;
    };
    match inner.kind() {
        "string" => TopLevel::Trivia,
        "assignment" if python_assignment_is_constant(inner) => TopLevel::ConstantDeclaration,
        _ => TopLevel::Other,
    }
}

/// Returns true when `assignment` binds a bare `NAME` (optionally typed)
/// to a pure literal value — the constant-table entry shape. A target
/// that is an attribute, subscript, or unpacking pattern is mutation,
/// not a constant declaration, and disqualifies the entry.
fn python_assignment_is_constant(assignment: Node<'_>) -> bool {
    let Some(left) = assignment.child_by_field_name("left") else {
        return false;
    };
    if left.kind() != "identifier" {
        return false;
    }
    assignment
        .child_by_field_name("right")
        .is_some_and(python_value_is_constant)
}

/// Returns true when `node` is a Python literal value with no embedded
/// logic: a plain string/number/bool/None, a unary on a literal, or a
/// homogeneous collection of literals. A call, name, attribute,
/// comprehension, lambda, or interpolated f-string is not constant and
/// keeps the cluster visible.
fn python_value_is_constant(node: Node<'_>) -> bool {
    match node.kind() {
        "string" => python_string_is_plain(node),
        "integer" | "float" | "true" | "false" | "none" => true,
        "unary_operator" => node
            .child_by_field_name("argument")
            .is_some_and(python_value_is_constant),
        "parenthesized_expression" => sole_named_child(node).is_some_and(python_value_is_constant),
        "concatenated_string" | "list" | "tuple" | "set" => {
            all_named_children(node, python_value_is_constant)
        }
        "dictionary" => all_named_children(node, python_pair_is_constant),
        _ => false,
    }
}

/// Returns true when a `dictionary` child is a `pair` whose key and
/// value are both constant. A `dictionary_splat` (`**rest`) or any other
/// child disqualifies the dictionary.
fn python_pair_is_constant(node: Node<'_>) -> bool {
    if node.kind() != "pair" {
        return false;
    }
    let key = node.child_by_field_name("key");
    let value = node.child_by_field_name("value");
    key.is_some_and(python_value_is_constant) && value.is_some_and(python_value_is_constant)
}

/// Returns true when `string` carries no `interpolation` child — i.e. it
/// is a plain literal, not an f-string that can embed arbitrary
/// expressions.
fn python_string_is_plain(node: Node<'_>) -> bool {
    !named_children(node)
        .into_iter()
        .any(|child| child.kind() == "interpolation")
}

/// Classifies one top-level `source_file` child in Rust. `const_item`
/// and `static_item` are the two declaration forms that bind a name to a
/// value; a `function_item`, `use_declaration`, `struct_item`,
/// `impl_item` or `mod_item` is not a table entry and takes the whole
/// range out of the shape — which is what stops this filter reaching a
/// run of copy-pasted top-level functions.
fn classify_rust_item(node: Node<'_>) -> TopLevel {
    match node.kind() {
        "line_comment" | "block_comment" | "attribute_item" | "inner_attribute_item" => {
            TopLevel::Trivia
        }
        "const_item" | "static_item" => classify_rust_declaration(node),
        _ => TopLevel::Other,
    }
}

/// Classifies a Rust `const_item` / `static_item` by its initialiser. A
/// declaration with no value (an associated `const` in a trait) or one
/// initialised by a call, a path, or a macro is not a table entry.
fn classify_rust_declaration(node: Node<'_>) -> TopLevel {
    match node.child_by_field_name("value") {
        Some(value) if rust_value_is_constant(value) => TopLevel::ConstantDeclaration,
        _ => TopLevel::Other,
    }
}

/// Returns true when `node` is a Rust literal value with no embedded
/// logic: a string (raw or otherwise), a number, a bool, a char, a
/// reference or unary applied to one of those, or an array/tuple of
/// them. `foo()`, `Path::CONST` and `concat!(…)` are none of these and
/// keep the cluster visible.
fn rust_value_is_constant(node: Node<'_>) -> bool {
    match node.kind() {
        "string_literal" | "raw_string_literal" | "integer_literal" | "float_literal"
        | "boolean_literal" | "char_literal" => true,
        "unary_expression" | "reference_expression" | "parenthesized_expression" => {
            sole_named_child(node).is_some_and(rust_value_is_constant)
        }
        "array_expression" | "tuple_expression" => all_named_children(node, rust_value_is_constant),
        _ => false,
    }
}

/// Classifies one top-level `program` child in JavaScript / TypeScript.
///
/// gh #283 is the value-level spelling of this shape: three modules of
/// `export const NAME = { … }` tables — directional language scalar
/// maps, render themes, and codec error metadata — reported as one
/// seventeen-member top-ranked cluster. `ecmascript::
/// is_ecmascript_data_shape_cluster` covers the *type*-level form
/// (`interface` / `object_type` runs of `property_signature`); a value
/// bound to an object literal is the same argument one level down, and
/// belongs to the same rule as Rust `const` and Python `NAME =`.
fn classify_ecmascript_item(node: Node<'_>) -> TopLevel {
    match node.kind() {
        "comment" => TopLevel::Trivia,
        "export_statement" => match sole_named_child(node) {
            Some(inner) => classify_ecmascript_item(inner),
            None => TopLevel::Other,
        },
        "lexical_declaration" | "variable_declaration" => classify_ecmascript_declaration(node),
        _ => TopLevel::Other,
    }
}

/// Classifies an ECMAScript `const` / `let` / `var` declaration: every
/// declarator must bind a plain name to a literal value. A declarator
/// with no initialiser, or one initialised by a call, an identifier or
/// an arrow function, is not a table entry.
fn classify_ecmascript_declaration(node: Node<'_>) -> TopLevel {
    let mut declarators = named_children(node)
        .into_iter()
        .filter(|child| child.kind() == "variable_declarator")
        .peekable();
    if declarators.peek().is_none() {
        return TopLevel::Other;
    }
    if declarators.all(ecmascript_declarator_is_constant) {
        TopLevel::ConstantDeclaration
    } else {
        TopLevel::Other
    }
}

/// Returns true when a `variable_declarator` binds a bare name to a
/// literal value.
fn ecmascript_declarator_is_constant(node: Node<'_>) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    if name.kind() != "identifier" {
        return false;
    }
    node.child_by_field_name("value")
        .is_some_and(ecmascript_value_is_constant)
}

/// Returns true when `node` is an ECMAScript literal value with no
/// embedded logic. A template string carrying a `template_substitution`
/// can embed arbitrary expressions and is excluded for the same reason
/// an interpolated f-string is in Python.
fn ecmascript_value_is_constant(node: Node<'_>) -> bool {
    match node.kind() {
        "string" | "number" | "true" | "false" | "null" | "undefined" => true,
        "template_string" => !has_child_kind(node, "template_substitution"),
        "unary_expression" | "parenthesized_expression" | "as_expression" => {
            sole_named_child(node).is_some_and(ecmascript_value_is_constant)
        }
        "array" => all_named_children(node, ecmascript_value_is_constant),
        "object" => all_named_children(node, ecmascript_pair_is_constant),
        _ => false,
    }
}

/// Returns true when an `object` child is a `pair` whose key is a plain
/// property name or string and whose value is constant. A spread
/// element, a shorthand property, or a method definition disqualifies
/// the object.
fn ecmascript_pair_is_constant(node: Node<'_>) -> bool {
    if node.kind() != "pair" {
        return false;
    }
    let key_is_plain = node
        .child_by_field_name("key")
        .is_some_and(|key| matches!(key.kind(), "property_identifier" | "string" | "number"));
    key_is_plain
        && node
            .child_by_field_name("value")
            .is_some_and(ecmascript_value_is_constant)
}

/// Returns true when any named child of `node` has kind `kind`.
fn has_child_kind(node: Node<'_>, kind: &str) -> bool {
    named_children(node)
        .into_iter()
        .any(|child| child.kind() == kind)
}

/// True when `node` is a pure literal value in `language`: no call, no
/// name, no interpolation, nothing that could carry logic. Shared with
/// the literal-variation call filter so "a literal payload" means one
/// thing in this crate rather than two that can drift
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]). A language with no
/// constant-table grammar has no literal grammar here either and
/// answers `false`.
pub(super) fn is_literal_value(language: &str, node: Node<'_>) -> bool {
    match language.as_bytes() {
        b"python" => python_value_is_constant(node),
        b"rust" => rust_value_is_constant(node),
        b"javascript" | b"typescript" | b"tsx" => ecmascript_value_is_constant(node),
        _ => false,
    }
}

/// Returns the only named child of `node`, or `None` when it has zero or
/// more than one.
fn sole_named_child(node: Node<'_>) -> Option<Node<'_>> {
    match named_children(node).as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

/// Returns true when `predicate` holds for every named child of `node`
/// (vacuously true for an empty collection such as `[]` or `{}`).
fn all_named_children(node: Node<'_>, predicate: fn(Node<'_>) -> bool) -> bool {
    named_children(node).into_iter().all(predicate)
}
