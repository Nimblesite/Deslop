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
//! - A payload's **values** must be static data all the way down. The
//!   outer node being a `dictionary` says nothing about what sits in
//!   the value positions; `{"gross": reconcile_amount(...)}` is a
//!   computed reconciliation wearing a dict around itself. Pinned by
//!   `python_dict_assert_payload_proof::
//!   a_call_inside_a_consumed_payload_value_is_not_excused`.
//! - **Module scope** may contain only imports, docstrings and the test
//!   functions themselves. Duplicated module-level wiring is executable
//!   logic outside every function the proof walks. Pinned by
//!   `python_dict_assert_reach::
//!   module_level_logic_is_not_excused_by_qualifying_tests`.
//! - A **decorator** is module-level wiring too. It qualifies only as a
//!   dotted name or a call on a dotted name whose every argument is
//!   static data: `@pytest.mark.parametrize("case", [...])` is test
//!   payload, `@pytest.mark.parametrize("case", build_cases())` is
//!   case-generation logic the `test_*` walk never reads. Pinned both
//!   ways by `python_dict_assert_payload_proof::
//!   executable_decorator_arguments_are_not_excused` and
//!   `static_decorators_stay_within_the_idiom`.
//! - What a decorator decorates must be a **function**. Proving the
//!   decorators static says nothing about the definition underneath
//!   them, and a decorated *class* body executes at import time where
//!   no `test_*` walk reaches it — `session = build_session(...)`
//!   beside the test methods would ride along unread. An undecorated
//!   class at module scope already fails open, so a decorator may not
//!   buy one a pass. Pinned by `python_dict_assert_payload_proof::
//!   class_body_logic_under_a_static_decorator_is_not_excused`.
//!
//! A range with no payload assignment at all — the assert-run window
//! whose dict sits above it — still qualifies on assertion shape alone;
//! that statement-level behaviour is the original idiom match.

use tree_sitter::Node;

use super::{
    is_multi_member_language_cluster, node_intersects_range, parse_for,
    python::python_function_name_starts_with, raw_snippet_texts_differ,
    trimmed_snippet_range, Snippet,
};
use crate::ast::{named_children, ByteRange};

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
    raw_snippet_texts_differ(snippets)
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
    named_children(root)
        .into_iter()
        .filter(|child| node_intersects_range(*child, range))
        .all(|child| match child.kind() {
            "function_definition"
            | "import_statement"
            | "import_from_statement"
            | "future_import_statement"
            | "comment" => true,
            "decorated_definition" => decorates_a_function(child) && decorators_are_static(child),
            "expression_statement" => is_docstring_statement(child),
            _ => false,
        })
}

/// Returns true when what the decorators decorate is a function.
///
/// Proving the decorators static says nothing about the definition
/// underneath them. A decorated **class** carries statements that no
/// `test_*` walk ever reaches — `session = build_session(...)` in a
/// class body executes at import time, and duplicated wiring is exactly
/// what this filter must not erase. An undecorated class at module
/// scope already fails open here; a decorator may not buy one a pass.
/// Pinned by `python_dict_assert_payload_proof::
/// class_body_logic_under_a_static_decorator_is_not_excused`.
fn decorates_a_function(definition: Node<'_>) -> bool {
    definition
        .child_by_field_name("definition")
        .is_some_and(|inner| inner.kind() == "function_definition")
}

/// Returns true when every decorator on the definition is proven
/// non-executable beyond decoration itself: a dotted name, or a call on
/// a dotted name whose every argument is static data. A computed
/// decorator argument is module-level logic the `test_*` walk never
/// reads, so it must fail the suppression.
fn decorators_are_static(definition: Node<'_>) -> bool {
    named_children(definition)
        .into_iter()
        .filter(|child| child.kind() == "decorator")
        .all(|decorator| {
            named_children(decorator)
                .into_iter()
                .all(decorator_expression_is_static)
        })
}

/// A decorator expression: a dotted name, or a static-argument call.
fn decorator_expression_is_static(expression: Node<'_>) -> bool {
    match expression.kind() {
        "identifier" | "attribute" => is_dotted_name(expression),
        "call" => call_is_static_decorator(expression),
        _ => false,
    }
}

/// `a.b.c` — identifiers joined by attribute access, nothing else.
fn is_dotted_name(node: Node<'_>) -> bool {
    match node.kind() {
        "identifier" => true,
        "attribute" => named_children(node).into_iter().all(is_dotted_name),
        _ => false,
    }
}

/// A decorator call is static when its callee is a dotted name and
/// every argument — positional or keyword — is static data.
fn call_is_static_decorator(call: Node<'_>) -> bool {
    let callee_is_dotted = call
        .child_by_field_name("function")
        .is_some_and(is_dotted_name);
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let arguments_static = named_children(arguments)
        .into_iter()
        .all(decorator_argument_is_static);
    callee_is_dotted && arguments_static
}

/// One decorator argument: a keyword's value, or the expression itself.
fn decorator_argument_is_static(argument: Node<'_>) -> bool {
    if argument.kind() == "keyword_argument" {
        return argument
            .child_by_field_name("value")
            .is_some_and(is_static_data);
    }
    is_static_data(argument)
}

/// Returns true for an expression statement that is only a plain string
/// — a docstring. An f-string is executable and does not count.
fn is_docstring_statement(statement: Node<'_>) -> bool {
    matches!(
        named_children(statement).as_slice(),
        [only] if only.kind() == "string" && !contains_interpolation(*only)
    )
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
    for child in named_children(node) {
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
    for child in named_children(body) {
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
    let Some(root) = chained_dict_assert_root(statement, source) else {
        return false;
    };
    roots.push(root);
    true
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
/// dictionary literal counts, and the dictionary must be static data
/// all the way down: a call, identifier, splat or comprehension in any
/// key or value position is program logic wearing a dict, not test
/// payload.
fn literal_payload_binding<'a>(statement: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    if statement.kind() != "expression_statement" {
        return None;
    }
    let assignment = named_children(statement)
        .into_iter()
        .find(|child| child.kind() == "assignment")?;
    let left = assignment
        .child_by_field_name("left")
        .filter(|left| left.kind() == "identifier")?;
    let _payload: Node<'_> = assignment
        .child_by_field_name("right")
        .filter(|right| right.kind() == "dictionary" && is_static_data(*right))?;
    source.get(left.start_byte()..left.end_byte())
}

/// Returns true when the node is provably static data all the way down:
/// scalar literals, and dictionaries, lists, tuples or sets whose every
/// element is static. A call, identifier, splat, comprehension or
/// f-string anywhere makes the value computed and fails the proof.
fn is_static_data(node: Node<'_>) -> bool {
    if is_literal_constant(node) {
        return true;
    }
    match node.kind() {
        "dictionary" | "list" | "tuple" | "set" | "pair" | "unary_operator" | "comment" => {
            named_children(node).into_iter().all(is_static_data)
        }
        _ => false,
    }
}

/// Returns the root identifier of a qualifying chained-dict assertion:
/// `assert <root>[k1][k2]` bare, or compared to a literal with a single
/// `==` / `is`. Anything else — a computed right operand, a chained
/// comparison, a non-literal key — is not the idiom.
fn chained_dict_assert_root<'a>(statement: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    if statement.kind() != "assert_statement" {
        return None;
    }
    let first = statement.named_child(0)?;
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
    let operands: Vec<Node<'tree>> = named_children(comparison);
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
    named_children(node)
        .into_iter()
        .any(|child| child.kind() == "interpolation" || contains_interpolation(child))
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

#[cfg(test)]
mod tests;
