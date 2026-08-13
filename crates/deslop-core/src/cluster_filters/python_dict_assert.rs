//! Chained nested-dict assertion filter
//! ([CLONE-NOISE-PY-DICT-ASSERT]).
//!
//! `assert payload["k1"]["k2"] == V` over a locally-built literal dict is
//! the pytest idiom for checking a nested response shape. Identifier and
//! literal normalisation collapses the variable, the keys and the
//! expected value, so two tests that verify entirely unrelated contracts
//! — a PATCH response and an `OpenAPI` document — reduce to the same
//! `assert __var__[__str__][__str__] == __const__` skeleton and cluster
//! across files.
//!
//! # Every granularity of the same idiom
//!
//! Fingerprinting emits one subtree per AST node, so the idiom is offered
//! for suppression at several ranges: the assert run alone, the enclosing
//! `test_*` function, and the whole module. They are not
//! interchangeable — cross-cluster subsumption only collapses views that
//! cover the same region in both directions, so the module-wide view
//! survives on its own whenever it names a different file set than the
//! assert-run view.
//!
//! Recognising only the innermost range therefore hid the idiom and
//! published it at the same time: two unrelated pytest modules surfaced
//! as a whole-file `structural_only` duplicate while their assert runs
//! were correctly suppressed. Matching on the `test_*` functions the
//! range *intersects* — enclosing or enclosed — sees one idiom at every
//! depth. Pinned by
//! `python_issue_107::chained_dict_assertions_across_test_files_do_not_cluster`.
//!
//! # Why that reach obliges the proof to be closed
//!
//! Matching at every depth means one misjudged statement erases the
//! assert run, its test function and its whole module together, so
//! nothing inside the range may ride along unproven:
//!
//! - The **right operand** must be a literal. `assert x["a"]["b"] ==
//!   reconcile_amount(...)` compares against computed logic, and logic
//!   duplicated across two modules is the thing this tool exists to
//!   report. Pinned by `python_dict_assert_rhs_logic::
//!   a_computed_right_operand_is_not_payload_noise`.
//! - Every **payload dictionary** in the range must be consumed by an
//!   assertion, and every assertion root must resolve to one when any
//!   are present. A dict no assert reads was never part of the idiom
//!   the filter proved. Pinned by `python_dict_assert_reach::
//!   an_unconsumed_payload_dictionary_is_not_excused`.
//! - **Module scope** may contain only imports, docstrings and the test
//!   functions themselves. Duplicated module-level wiring is executable
//!   logic outside every function the proof walks. Pinned by
//!   `python_dict_assert_reach::
//!   module_level_logic_is_not_excused_by_qualifying_tests`.
//!
//! A range with no payload assignment at all — the assert-run window
//! whose dict sits above it — still qualifies on assertion shape alone;
//! that statement-level behaviour is the original idiom match.

use tree_sitter::Node;

use super::{
    is_multi_member_language_cluster, node_intersects_range, parse_for,
    python::python_function_name_starts_with, raw_snippet_texts_differ, spans_multiple_files,
    trimmed_snippet_range, Snippet,
};
use crate::ast::ByteRange;

/// Detects [CLONE-NOISE-PY-DICT-ASSERT]: the chained
/// `assert <var>[k1][k2] == V` shape across at least two unrelated
/// pytest test functions in different files.
///
/// Members whose reported bytes are all identical are exempt: a verbatim
/// copy of a test is real duplication whatever idiom it is written in,
/// and this filter exists for tests that merely *rhyme*.
pub(super) fn is_chained_dict_assert_cluster(snippets: &[Snippet<'_>]) -> bool {
    if !is_multi_member_language_cluster(snippets, "python") {
        return false;
    }
    spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id))
        && raw_snippet_texts_differ(snippets)
        && snippets.iter().all(is_chained_dict_assert_snippet)
}

/// Returns true when every `test_*` function the reported range touches
/// asserts only chained-subscript lookups over literal payloads, the
/// range touches at least one, and nothing else code-bearing shares the
/// module scope the range covers.
fn is_chained_dict_assert_snippet(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let Some(range) = trimmed_snippet_range(snippet) else {
        return false;
    };
    if !module_scope_is_idiom_only(tree.root_node(), range) {
        return false;
    }
    let mut functions = Vec::new();
    collect_intersecting_functions(tree.root_node(), range, &mut functions);
    !functions.is_empty()
        && functions
            .iter()
            .all(|function| function_is_chained_dict_test(*function, range, snippet.source))
}

/// Returns true when every module-level statement the range covers
/// belongs to the idiom's scope: the test functions themselves,
/// imports, and docstrings. Anything else at module level — an
/// assignment, a call, a class — is executable logic no `test_*` walk
/// ever proves, so the range must fail open and stay visible.
fn module_scope_is_idiom_only(root: Node<'_>, range: ByteRange) -> bool {
    let mut cursor = root.walk();
    let idiom_only = root
        .named_children(&mut cursor)
        .filter(|child| node_intersects_range(*child, range))
        .all(|child| match child.kind() {
            "function_definition"
            | "decorated_definition"
            | "import_statement"
            | "import_from_statement"
            | "future_import_statement"
            | "comment" => true,
            "expression_statement" => is_docstring_statement(child),
            _ => false,
        });
    idiom_only
}

/// Returns true for an expression statement that is only a plain string
/// — a docstring. An f-string is executable and does not count.
fn is_docstring_statement(statement: Node<'_>) -> bool {
    let mut cursor = statement.walk();
    let mut children = statement.named_children(&mut cursor);
    let only = children.next().filter(|_| children.next().is_none());
    only.is_some_and(|child| child.kind() == "string" && !contains_interpolation(child))
}

/// Collects every `function_definition` whose bytes overlap `range` —
/// the function enclosing a statement-level range, and the functions
/// contained in a function- or module-level one.
fn collect_intersecting_functions<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    out: &mut Vec<Node<'tree>>,
) {
    if !node_intersects_range(node, range) {
        return;
    }
    if node.kind() == "function_definition" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_intersecting_functions(child, range, out);
    }
}

/// Returns true for a pytest `test_*` function whose body, within
/// `range`, is a closed chained-dict idiom: payload dictionaries, the
/// assertions that consume them, and nothing else.
fn function_is_chained_dict_test(function: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    python_function_name_starts_with(function, source, b"test_")
        && function
            .child_by_field_name("body")
            .is_some_and(|body| body_is_closed_idiom(body, range, source))
}

/// The payload-and-assertion ledger for one function body. Every
/// in-range statement must be accounted for: a payload dictionary is
/// recorded, an assertion records the root it reads, and anything else
/// fails the proof. When any payload was recorded, every assertion must
/// resolve to one and every payload must be consumed — a dictionary no
/// assertion reads was never proven to be part of the idiom.
fn body_is_closed_idiom(body: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    let mut payloads: Vec<(&[u8], bool)> = Vec::new();
    let mut roots: Vec<&[u8]> = Vec::new();
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        if !node_intersects_range(child, range) || is_docstring_statement(child) {
            continue;
        }
        if !record_statement(child, source, &mut payloads, &mut roots) {
            return false;
        }
    }
    !roots.is_empty() && ledger_balances(&mut payloads, &roots)
}

/// Files one in-range statement into the ledger. Returns false when the
/// statement is neither a fresh payload binding nor a qualifying
/// assertion — a rebound payload name is rejected too, because the
/// key-path resolution below could no longer name one dictionary.
fn record_statement<'a>(
    statement: Node<'_>,
    source: &'a [u8],
    payloads: &mut Vec<(&'a [u8], bool)>,
    roots: &mut Vec<&'a [u8]>,
) -> bool {
    if let Some(binding) = literal_payload_binding(statement, source) {
        if payloads.iter().any(|(name, _)| *name == binding) {
            return false;
        }
        payloads.push((binding, false));
        return true;
    }
    match chained_dict_assert_root(statement, source) {
        Some(root) => {
            roots.push(root);
            true
        }
        None => false,
    }
}

/// Returns true when every assertion root resolves to a recorded
/// payload and every payload was consumed — or no payload was in range
/// at all, the assert-run window whose dictionary sits above it.
fn ledger_balances(payloads: &mut [(&[u8], bool)], roots: &[&[u8]]) -> bool {
    if payloads.is_empty() {
        return true;
    }
    for root in roots {
        match payloads.iter_mut().find(|(name, _)| name == root) {
            Some(payload) => payload.1 = true,
            None => return false,
        }
    }
    payloads.iter().all(|(_, consumed)| *consumed)
}

/// Returns the bound name of `<name> = { ... }` — the literal dict the
/// chained assertions read. Only a plain identifier bound to a
/// dictionary literal counts: a call, a fixture reference, an attribute
/// target or a comprehension is program logic, not test payload.
fn literal_payload_binding<'a>(statement: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    if statement.kind() != "expression_statement" {
        return None;
    }
    let mut cursor = statement.walk();
    let assignment = statement
        .named_children(&mut cursor)
        .find(|child| child.kind() == "assignment")?;
    let left = assignment
        .child_by_field_name("left")
        .filter(|left| left.kind() == "identifier")?;
    let _right: Node<'_> = assignment
        .child_by_field_name("right")
        .filter(|right| right.kind() == "dictionary")?;
    source.get(left.start_byte()..left.end_byte())
}

/// Returns the root identifier of a qualifying chained-dict assertion:
/// `assert <root>[k1][k2]` bare, or compared to a literal with a single
/// `==` / `is`. Anything else — a computed right operand, a chained
/// comparison, a non-literal key — is not the idiom.
fn chained_dict_assert_root<'a>(statement: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    if statement.kind() != "assert_statement" {
        return None;
    }
    let mut cursor = statement.walk();
    let first = statement.named_children(&mut cursor).next()?;
    let chain = match first.kind() {
        "comparison_operator" => comparison_against_literal(first)?,
        _ => first,
    };
    subscript_chain_root(chain, 0, source)
}

/// Returns the left-hand chain of `<chain> <op> <literal>` when the
/// comparison has exactly one operator, that operator is `==` or `is`,
/// and the right operand is a literal. The right operand is what
/// separates payload from logic: a call or an attribute there is
/// executable code the idiom never proves.
fn comparison_against_literal<'tree>(comparison: Node<'tree>) -> Option<Node<'tree>> {
    let mut operand_cursor = comparison.walk();
    let operands: Vec<Node<'tree>> = comparison.named_children(&mut operand_cursor).collect();
    let mut operator_cursor = comparison.walk();
    let operators: Vec<Node<'tree>> = comparison
        .children_by_field_name("operators", &mut operator_cursor)
        .collect();
    let [operator] = operators.as_slice() else {
        return None;
    };
    let [left, right] = operands.as_slice() else {
        return None;
    };
    if !matches!(operator.kind(), "==" | "is") || !is_literal_constant(*right) {
        return None;
    }
    Some(*left)
}

/// Returns true for a scalar literal — the only expected value a
/// nested-shape assertion may carry. Calls, identifiers, subscripts and
/// comprehensions are excluded by construction, and an f-string is
/// executable rather than literal.
fn is_literal_constant(node: Node<'_>) -> bool {
    match node.kind() {
        "string" | "concatenated_string" => !contains_interpolation(node),
        "integer" | "float" | "true" | "false" | "none" => true,
        _ => false,
    }
}

/// Returns true when a string carries an f-string interpolation hole.
fn contains_interpolation(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let interpolated = node
        .named_children(&mut cursor)
        .any(|child| child.kind() == "interpolation" || contains_interpolation(child));
    interpolated
}

/// Walks a `subscript(subscript(identifier))` tower of literal keys and
/// returns the root identifier's bytes once the tower is at least two
/// hops deep. A computed key is not a shape check and fails the walk.
fn subscript_chain_root<'a>(node: Node<'_>, depth: usize, source: &'a [u8]) -> Option<&'a [u8]> {
    if node.kind() != "subscript" || !subscript_key_is_literal(node) {
        return None;
    }
    let value = node.child_by_field_name("value")?;
    match value.kind() {
        "subscript" => subscript_chain_root(value, depth.saturating_add(1), source),
        "identifier" if depth >= 1 => source.get(value.start_byte()..value.end_byte()),
        _ => None,
    }
}

/// Returns true when a subscript's index is a scalar literal key.
fn subscript_key_is_literal(subscript: Node<'_>) -> bool {
    subscript
        .child_by_field_name("subscript")
        .is_some_and(is_literal_constant)
}
