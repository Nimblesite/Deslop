//! Pre-existing Python idiom cluster filters split out of `python.rs`
//! to keep each file under the 500-LOC budget.
//!
//! Issues addressed (see parent `mod.rs` header):
//! - **#72**  [CLONE-NOISE-PY-MONKEYPATCH] — `monkeypatch.setenv` literals.
//! - **#96**  [CLONE-NOISE-PY-ALL-EXPORTS] — module-level `__all__` lists.
//! - **#114** [CLONE-NOISE-PY-JWT-HS256] — independent HS256 implementations.
//! - **#126** [CLONE-NOISE-PY-GENERATED-OUTPUT] — generated-output headers
//!   shared between generator templates and generated files.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{
    contains_bytes, enclosing_kind, node_contains_identifier, parse_for, snippet_range_text,
    source_head, trim_ascii_start, Snippet,
};
use crate::state::FileId;

/// Detects **issue #114**: production HS256/JWT signing code and tests
/// that re-implement the same HMAC calculation independently. The
/// test-side duplication is intentional black-box verification; if the
/// test called the production minter/helper, it would stop proving the
/// signing implementation.
pub(super) fn is_jwt_hmac_independent_verifier_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "python") {
        return false;
    }
    let shapes: Option<Vec<JwtHmacShape>> = snippets.iter().map(jwt_hmac_shape).collect();
    let Some(shapes) = shapes else { return false };
    let mut files = BTreeSet::new();
    for shape in &shapes {
        let _inserted = files.insert(shape.file_id);
    }
    files.len() >= 2
        && shapes.iter().all(|shape| shape.is_hs256_body)
        && shapes.iter().any(|shape| shape.is_test_source)
        && shapes.iter().any(|shape| !shape.is_test_source)
}

/// Distilled source-level shape for one HS256 signing occurrence.
struct JwtHmacShape {
    /// Registry id of the source file containing this member.
    file_id: FileId,
    /// True when the full source file looks like a test module.
    is_test_source: bool,
    /// True when the enclosing function body implements the HS256
    /// HMAC/base64url signing pattern.
    is_hs256_body: bool,
}

/// Extracts HS256 signing shape from the enclosing Python function.
fn jwt_hmac_shape(snippet: &Snippet<'_>) -> Option<JwtHmacShape> {
    let tree = parse_for(snippet)?;
    let function = enclosing_kind(tree.root_node(), snippet.range, &["function_definition"])?;
    let body = function.child_by_field_name("body")?;
    let body_source = snippet.source.get(body.start_byte()..body.end_byte())?;
    Some(JwtHmacShape {
        file_id: snippet.file_id,
        is_test_source: python_source_looks_like_test(snippet.source),
        is_hs256_body: python_body_looks_like_hs256(body_source),
    })
}

/// Returns true for Python test modules without relying on filesystem
/// paths, which the cluster filter does not receive.
fn python_source_looks_like_test(source: &[u8]) -> bool {
    contains_bytes(source, b"def test_") || contains_bytes(source, b"expected_hs256")
}

/// Returns true for the stdlib HS256 signing shape:
/// `hmac.new(..., hashlib.sha256).digest()` followed by base64url
/// encoding. Requiring all three calls keeps the filter tighter than a
/// generic "uses hmac" suppression.
fn python_body_looks_like_hs256(body_source: &[u8]) -> bool {
    contains_bytes(body_source, b"hmac.new")
        && contains_bytes(body_source, b"hashlib.sha256")
        && contains_bytes(body_source, b"urlsafe_b64encode")
}

/// Detects **issue #126**: a hand-written generator source contains a
/// template literal for a generated-file header, and the generated file
/// itself carries the same `DO NOT HAND-EDIT` marker. These two ranges
/// are intentionally related, but the generated output is not a refactor
/// target and the template is already the source of truth.
pub(super) fn is_generated_template_output_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "python") {
        return false;
    }
    let mut files = BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    files.len() >= 2
        && snippets.iter().any(is_generated_header_template_snippet)
        && snippets.iter().any(is_generated_output_source)
}

/// Returns true when the reported range itself contains a generated-file
/// marker, but the file is not the generated output. This is the
/// generator-template side of issue #126.
fn is_generated_header_template_snippet(snippet: &Snippet<'_>) -> bool {
    !is_generated_output_source(snippet)
        && snippet_range_text(snippet).is_some_and(contains_generated_marker)
}

/// Returns true for a Python generated output file with a top-of-file
/// docstring/comment that warns users not to edit it directly.
fn is_generated_output_source(snippet: &Snippet<'_>) -> bool {
    let head = source_head(snippet.source);
    let trimmed = trim_ascii_start(head);
    contains_generated_marker(head)
        && (trimmed.starts_with(b"\"\"\"")
            || trimmed.starts_with(b"'''")
            || trimmed.starts_with(b"#"))
}

/// Generated-file marker used by first-party and dogfood fixtures.
fn contains_generated_marker(bytes: &[u8]) -> bool {
    contains_bytes(bytes, b"DO NOT HAND-EDIT")
}

/// Detects **issue #96**: every cluster member is a Python module-level
/// `__all__ = [...]` assignment. The export list shape is identical
/// across modules by convention, but the listed names always differ —
/// after Type-2 normalisation they look identical, yet the package
/// surface is not duplicate logic and cannot be extracted.
pub(super) fn is_python_all_exports_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.iter().all(is_python_all_exports_snippet)
}

/// Returns true when `snippet` covers a Python `__all__ = [...]`
/// module-level assignment (or its enclosing list literal).
fn is_python_all_exports_snippet(snippet: &Snippet<'_>) -> bool {
    if snippet.language != "python" {
        return false;
    }
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    enclosing_kind(tree.root_node(), snippet.range, &["expression_statement"])
        .or_else(|| enclosing_kind(tree.root_node(), snippet.range, &["assignment"]))
        .is_some_and(|node| python_node_assigns_all_exports(node, snippet.source))
}

/// Returns true when `node` (an `expression_statement` or `assignment`)
/// binds the identifier `__all__` to a list/tuple literal.
fn python_node_assigns_all_exports(node: Node<'_>, source: &[u8]) -> bool {
    let assignment = python_descend_to_assignment(node);
    let Some(left) = assignment.child_by_field_name("left") else {
        return false;
    };
    if source.get(left.start_byte()..left.end_byte()) != Some(b"__all__".as_slice()) {
        return false;
    }
    let Some(right) = assignment.child_by_field_name("right") else {
        return false;
    };
    matches!(right.kind(), "list" | "tuple")
}

/// If `node` is an `expression_statement`, descend to its inner
/// `assignment` child; otherwise return `node` unchanged.
fn python_descend_to_assignment(node: Node<'_>) -> Node<'_> {
    if node.kind() != "expression_statement" {
        return node;
    }
    let mut cursor = node.walk();
    let inner = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "assignment");
    inner.unwrap_or(node)
}

/// Suppresses tiny string literal clusters inside pytest monkeypatch
/// setup tests. Issue #72 covers this exact scaffold: literals differ
/// intentionally because they are environment keys and values.
pub(super) fn is_monkeypatch_scaffolding_literal_cluster(snippets: &[Snippet<'_>]) -> bool {
    snippets.iter().all(is_monkeypatch_scaffolding_literal)
}

/// Returns true for a string literal inside a Python function that
/// declares the `monkeypatch` fixture parameter.
fn is_monkeypatch_scaffolding_literal(snippet: &Snippet<'_>) -> bool {
    if snippet.language != "python" {
        return false;
    }
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root = tree.root_node();
    let is_string = enclosing_kind(root, snippet.range, &["string"]).is_some();
    let function = enclosing_kind(root, snippet.range, &["function_definition"]);
    is_string && function.is_some_and(|node| function_has_parameter(node, snippet.source))
}

/// Checks a Python function definition for a `monkeypatch` parameter.
fn function_has_parameter(function: Node<'_>, source: &[u8]) -> bool {
    let Some(parameters) = function.child_by_field_name("parameters") else {
        return false;
    };
    node_contains_identifier(parameters, source, b"monkeypatch")
}
