//! Python-specific cluster filters.
//!
//! Suppresses false-positive clusters whose shape is fixed by Python or
//! pytest idioms rather than by the program under analysis. Pre-existing
//! idiom filters (#72 / #96 / #114 / #126) live in `python_idioms.rs`
//! and ORM-shaped filters (#100, #105) live in `python_orm.rs` so each
//! file stays under the 500-LOC budget.
//!
//! Issues addressed (see parent `mod.rs` header):
//! - **#99**  [CLONE-NOISE-PY-ASSERT-ONLY] — assert-only test bodies.
//! - **#107** [CLONE-NOISE-PY-DICT-ASSERT] — chained `assert dict[k1][k2]`
//!   shape across unrelated test files.
//! - **#112** [CLONE-NOISE-PY-DICT-FIXTURE] — small nested-dict literals
//!   across pytest fixture files.
//! - **#121** [CLONE-NOISE-PY-PYTEST-FIXTURE] — pytest fixture boilerplate.
//! - **#97**  [CLONE-NOISE-PY-PARAMETRIC-INVARIANT-TESTS] — `test_*`
//!   bodies that vary only by enum-member access (`X.K8S` vs `X.DOCKER`).

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{
    enclosing_kind, node_contains_kind, node_intersects_range, parse_for, raw_snippet_texts_differ,
    trimmed_snippet_range, Snippet,
};
use crate::{ast::ByteRange, state::FileId};

/// Detects **issue #121**: pytest fixture functions that create ORM rows
/// all repeat the same session setup shape. The fixture is already the
/// test abstraction, so surfacing those bodies as refactor targets adds
/// noise instead of useful duplication.
pub(super) fn is_pytest_fixture_boilerplate_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "python") {
        return false;
    }
    let mut files = BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    files.len() >= 2 && snippets.iter().all(is_pytest_fixture_snippet)
}

/// Returns true when the snippet belongs to a Python function decorated
/// with `@fixture` or any dotted fixture decorator such as
/// `@pytest.fixture` / `@pytest_asyncio.fixture`.
fn is_pytest_fixture_snippet(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let Some(range) = trimmed_snippet_range(snippet) else {
        return false;
    };
    let Some(function) = enclosing_python_function(tree.root_node(), range) else {
        return false;
    };
    python_function_has_fixture_decorator(function, snippet.source)
}

/// Finds the Python function containing a reported range, including the
/// common case where the range starts at the decorator and therefore
/// encloses the `function_definition` rather than sitting inside it.
fn enclosing_python_function(root: Node<'_>, range: ByteRange) -> Option<Node<'_>> {
    enclosing_kind(root, range, &["function_definition"]).or_else(|| {
        let decorated = enclosing_kind(root, range, &["decorated_definition"])?;
        let mut cursor = decorated.walk();
        let function = decorated
            .named_children(&mut cursor)
            .find(|child| child.kind() == "function_definition");
        function
    })
}

/// Checks the decorator block immediately above a Python function.
fn python_function_has_fixture_decorator(function: Node<'_>, source: &[u8]) -> bool {
    let Some(prefix) = source
        .get(..function.start_byte())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    for line in prefix.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('@') {
            return false;
        }
        if decorator_line_has_fixture_callee(trimmed) {
            return true;
        }
    }
    false
}

/// Returns true for `@fixture(...)` and dotted variants ending in
/// `.fixture(...)`.
fn decorator_line_has_fixture_callee(line: &str) -> bool {
    let Some(decorator) = line.strip_prefix('@') else {
        return false;
    };
    let callee = decorator
        .split(|ch: char| ch == '(' || ch.is_whitespace())
        .next()
        .unwrap_or_default();
    callee.rsplit('.').next() == Some("fixture")
}

/// Detects **issue #99**: blocks consisting only of Python `assert`
/// statements across test files. Their AST/token shape is intentionally
/// repetitive, but the concrete asserted paths and values differ.
pub(super) fn is_python_assertion_only_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "python") {
        return false;
    }
    let mut files = BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    files.len() >= 2
        && raw_snippet_texts_differ(snippets)
        && snippets.iter().all(is_python_assertion_only_snippet)
}

/// Returns true when one snippet's enclosing Python function body contains
/// only assertion statements within the reported range and no calls/control
/// flow setup.
fn is_python_assertion_only_snippet(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let Some(range) = trimmed_snippet_range(snippet) else {
        return false;
    };
    let Some(function) = enclosing_kind(tree.root_node(), range, &["function_definition"]) else {
        return false;
    };
    let Some(body) = function.child_by_field_name("body") else {
        return false;
    };
    assert_only_body_in_range(body, range)
}

/// Walks `body` and returns true when every named child overlapping
/// `range` is an `assert_statement` with no nested call expressions.
fn assert_only_body_in_range(body: Node<'_>, range: ByteRange) -> bool {
    let mut cursor = body.walk();
    let mut saw_assert = false;
    for child in body.named_children(&mut cursor) {
        if !node_intersects_range(child, range) {
            continue;
        }
        if child.kind() != "assert_statement" || node_contains_kind(child, "call") {
            return false;
        }
        saw_assert = true;
    }
    saw_assert
}

/// Detects **issue #107**: chained `assert <var>[k1][k2] == V` shape
/// across at least two unrelated pytest test functions. Identifier
/// normalisation collapses the variable, keys and value, so the
/// surviving structure is identical even though every test exercises a
/// different contract.
pub(super) fn is_chained_dict_assert_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "python") {
        return false;
    }
    let mut files = BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    files.len() >= 2 && snippets.iter().all(is_chained_dict_assert_snippet)
}

/// Returns true when `snippet` lives inside a pytest `test_*` function
/// and the reported range contains only chained-subscript assertions.
fn is_chained_dict_assert_snippet(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let Some(range) = trimmed_snippet_range(snippet) else {
        return false;
    };
    let Some(function) = enclosing_kind(tree.root_node(), range, &["function_definition"]) else {
        return false;
    };
    if !python_function_name_starts_with(function, snippet.source, b"test_") {
        return false;
    }
    let Some(body) = function.child_by_field_name("body") else {
        return false;
    };
    body_has_only_chained_dict_asserts(body, range, snippet.source)
}

/// Returns true when every named child of `body` overlapping `range` is
/// an `assert_statement` whose left-hand side is a `subscript(subscript)`
/// chain — and at least one such assert is present.
fn body_has_only_chained_dict_asserts(body: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    let mut cursor = body.walk();
    let mut saw = false;
    for child in body.named_children(&mut cursor) {
        if !node_intersects_range(child, range) {
            continue;
        }
        if child.kind() != "assert_statement" {
            return false;
        }
        if !assert_statement_is_chained_dict(child, source) {
            return false;
        }
        saw = true;
    }
    saw
}

/// Returns true for `assert <chain> == <const>` / `is <const>` where the
/// chain is two or more nested subscript accesses against an identifier.
fn assert_statement_is_chained_dict(assert_node: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = assert_node.walk();
    let mut named = assert_node.named_children(&mut cursor);
    let Some(first) = named.next() else {
        return false;
    };
    let chain = match first.kind() {
        "comparison_operator" => comparison_left(first),
        _ => Some(first),
    };
    let Some(chain) = chain else { return false };
    subscript_chain_depth(chain, source) >= 2
}

/// Returns the left operand of a Python `comparison_operator` node.
fn comparison_left(comparison: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = comparison.walk();
    let first = comparison.named_children(&mut cursor).next();
    first
}

/// Counts subscript hops down a `subscript(subscript(identifier))` tower.
fn subscript_chain_depth(node: Node<'_>, source: &[u8]) -> usize {
    if node.kind() != "subscript" {
        return 0;
    }
    let Some(value) = node.child_by_field_name("value") else {
        return 0;
    };
    if value.kind() == "subscript" {
        return subscript_chain_depth(value, source).saturating_add(1);
    }
    if value.kind() == "identifier" && source.get(value.start_byte()..value.end_byte()).is_some() {
        return 1;
    }
    0
}

/// Returns true when the function's declared name starts with `prefix`.
fn python_function_name_starts_with(function: Node<'_>, source: &[u8], prefix: &[u8]) -> bool {
    let Some(name) = function.child_by_field_name("name") else {
        return false;
    };
    source
        .get(name.start_byte()..name.end_byte())
        .is_some_and(|bytes| bytes.starts_with(prefix))
}

/// Detects **issue #112**: small nested-dict literals appearing across
/// multiple pytest test files. Dict-literal fixtures in tests carry the
/// same AST shape and token alphabet (`name`, `description`, ...) but
/// encode unrelated request/response payloads. Fires only when every
/// member is inside a pytest `test_*` function and at least one member
/// uses a different set of dict keys.
pub(super) fn is_test_dict_literal_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "python") {
        return false;
    }
    let shapes: Option<Vec<DictLiteralShape>> =
        snippets.iter().map(test_dict_literal_shape).collect();
    let Some(shapes) = shapes else { return false };
    let mut files = BTreeSet::new();
    for shape in &shapes {
        let _inserted = files.insert(shape.file_id);
    }
    files.len() >= 2 && dict_literal_key_sets_differ(&shapes)
}

/// Per-member shape: the set of string keys declared by the dict
/// literal that encloses the reported range.
struct DictLiteralShape {
    /// File providing this member.
    file_id: FileId,
    /// Top-level string keys of the dict literal.
    keys: BTreeSet<Vec<u8>>,
}

/// Returns the [`DictLiteralShape`] for `snippet` when the reported
/// range covers exactly one dictionary literal sitting inside a pytest
/// `test_*` function. The dictionary is located by descending into the
/// range — `enclosing_kind` cannot find a node smaller than the range
/// itself.
fn test_dict_literal_shape(snippet: &Snippet<'_>) -> Option<DictLiteralShape> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let root = tree.root_node();
    let function = enclosing_kind(root, range, &["function_definition"])?;
    if !python_function_name_starts_with(function, snippet.source, b"test_") {
        return None;
    }
    let dict = sole_dictionary_in_range(root, range)?;
    let keys = dict_literal_top_level_keys(dict, snippet.source);
    if keys.is_empty() {
        return None;
    }
    Some(DictLiteralShape {
        file_id: snippet.file_id,
        keys,
    })
}

/// Returns the single outer `dictionary` node fully enclosed by `range`,
/// or `None` when the range contains zero, more than one top-level
/// dictionary, or any non-dictionary value. Nested dictionaries (inside
/// `pair.value`) are not counted as additional outer dictionaries.
fn sole_dictionary_in_range(root: Node<'_>, range: ByteRange) -> Option<Node<'_>> {
    let mut dictionaries = Vec::new();
    collect_outer_dictionaries(root, range, &mut dictionaries);
    let [dict] = dictionaries.as_slice() else {
        return None;
    };
    Some(*dict)
}

/// Walks the tree collecting `dictionary` nodes fully enclosed by
/// `range`. Stops descending once a `dictionary` is found so inner
/// dictionaries (values of `pair`s) are not double-counted.
fn collect_outer_dictionaries<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    out: &mut Vec<Node<'tree>>,
) {
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return;
    }
    if node.kind() == "dictionary"
        && node.start_byte() >= range.start
        && node.end_byte() <= range.end
    {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_outer_dictionaries(child, range, out);
    }
}

/// Returns the string keys of `dict` (`pair.key` children that are
/// string literals). Ignores non-string keys.
fn dict_literal_top_level_keys(dict: Node<'_>, source: &[u8]) -> BTreeSet<Vec<u8>> {
    let mut cursor = dict.walk();
    let mut keys = BTreeSet::new();
    for pair in dict.named_children(&mut cursor) {
        if pair.kind() != "pair" {
            continue;
        }
        let Some(key) = pair.child_by_field_name("key") else {
            continue;
        };
        if key.kind() != "string" {
            continue;
        }
        if let Some(bytes) = source.get(key.start_byte()..key.end_byte()) {
            let _inserted = keys.insert(bytes.to_vec());
        }
    }
    keys
}

/// Returns true when at least two members declare different key sets.
fn dict_literal_key_sets_differ(shapes: &[DictLiteralShape]) -> bool {
    let Some(first) = shapes.first() else {
        return false;
    };
    shapes.iter().any(|shape| shape.keys != first.keys)
}

/// Detects **issue #97**: every cluster occurrence is fully enclosed by
/// a `test_*` pytest function AND every occurrence contains at least
/// one `Capitalised.UPPER_SNAKE` enum-member access token. Each test
/// name records a distinct spec assertion; collapsing the cluster would
/// silently lose coverage granularity even when the Type-2 normalised
/// bodies are identical across unrelated discriminators.
pub(super) fn is_parametric_invariant_test_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "python") {
        return false;
    }
    snippets.iter().all(snippet_inside_parametric_test)
}

/// Returns true when `snippet` lives inside a pytest `test_*` function
/// AND its byte range carries at least one `Capitalised.UPPER_SNAKE`
/// enum-member access token. The enum signal guards against suppressing
/// genuine non-parametric copy-paste inside test files.
fn snippet_inside_parametric_test(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let Some(function) = enclosing_kind(tree.root_node(), range, &["function_definition"]) else {
        return false;
    };
    if !python_function_name_starts_with(function, snippet.source, b"test_") {
        return false;
    }
    let Some(text) = snippet.source.get(range.start..range.end) else {
        return false;
    };
    contains_enum_member_access_token(text)
}

/// Returns true when `text` contains at least one `Capitalised.UPPER_SNAKE`
/// token (e.g. `Kind.K8S`). Scans bytes only — never regex on source.
fn contains_enum_member_access_token(text: &[u8]) -> bool {
    (0..text.len()).any(|index| consume_enum_access_at(text, index).is_some())
}

/// Tries to consume a `Capitalised.UPPER_SNAKE` token at `index`,
/// returning the number of bytes consumed when matched.
fn consume_enum_access_at(body: &[u8], index: usize) -> Option<usize> {
    if !at_token_boundary(body, index) {
        return None;
    }
    let prefix_len = identifier_run_len(body, index)?;
    if !ident_starts_capitalised(body, index) {
        return None;
    }
    let dot = index.checked_add(prefix_len)?;
    if body.get(dot).copied() != Some(b'.') {
        return None;
    }
    let suffix_start = dot.checked_add(1)?;
    let suffix_len = identifier_run_len(body, suffix_start)?;
    if !ident_is_upper_snake(body, suffix_start, suffix_len) {
        return None;
    }
    prefix_len.checked_add(1)?.checked_add(suffix_len)
}

/// Returns true when the previous byte is not an identifier-continuation
/// byte — i.e. `index` sits at the start of a fresh token.
fn at_token_boundary(body: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    let previous = index.checked_sub(1).and_then(|prev| body.get(prev));
    previous.is_some_and(|byte| !is_ident_cont(*byte))
}

/// Returns the length of the contiguous identifier-byte run starting at
/// `start`, or `None` when no identifier byte sits there.
fn identifier_run_len(body: &[u8], start: usize) -> Option<usize> {
    if start >= body.len() {
        return None;
    }
    let first = *body.get(start)?;
    if !is_ident_start(first) {
        return None;
    }
    let mut end = start.checked_add(1)?;
    while end < body.len() && body.get(end).is_some_and(|byte| is_ident_cont(*byte)) {
        end = end.checked_add(1)?;
    }
    end.checked_sub(start)
}

/// ASCII identifier-start predicate (letters and `_`).
fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

/// ASCII identifier-continuation predicate (letters, digits, `_`).
fn is_ident_cont(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Returns true when the identifier byte at `start` is an ASCII
/// uppercase letter.
fn ident_starts_capitalised(body: &[u8], start: usize) -> bool {
    body.get(start).is_some_and(u8::is_ascii_uppercase)
}

/// Returns true when the identifier at `[start, start+len)` consists of
/// uppercase ASCII letters, digits, or underscores, with at least one
/// uppercase letter present.
fn ident_is_upper_snake(body: &[u8], start: usize, len: usize) -> bool {
    let Some(slice) = body.get(start..start.saturating_add(len)) else {
        return false;
    };
    slice.iter().any(u8::is_ascii_uppercase)
        && slice
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b'_')
}
