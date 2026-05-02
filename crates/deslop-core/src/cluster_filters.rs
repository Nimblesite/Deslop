//! Cluster-level false-positive filters applied during report rendering.
//!
//! Noise patterns that survive structural fingerprinting because Type-2
//! normalisation collapses identifiers and literals. We re-parse each
//! cluster member's original source bytes and look at the *real* tree —
//! the same way a reviewer would — so we can drop clusters that the
//! pipeline cannot tell apart but a human instantly would.
//!
//! Issues addressed:
//! - **#69** — abstract method signatures cluster across every concrete
//!   subclass that implements an `ABC`. The 37-node subtree is the
//!   parameter list; identical-by-contract is not duplication.
//! - **#70** — `_tool_call_response("write_file", {...}, "id")` and
//!   peers cluster when only string literal args vary. Test fixture
//!   data, not duplication.
//! - **#71** — REST endpoint tests cluster on the HTTP-call shape but
//!   the f-string templates encode different routes. Different routes
//!   are different tests, not duplication.
//! - **#72** — `monkeypatch.setenv("KEY", "VAL")` test scaffolding
//!   clusters because every two-setenv test looks the same after
//!   normalisation. Scaffolding is not logic.
//! - **#75** — every first-party Rust language plug-in implements the
//!   same `LanguageParser` trait surface. The adapter shape is required
//!   by the trait contract, not extractable business logic.
//! - **#114** — tests can independently re-implement HS256/JWT signing
//!   to verify a production minter as a black box. Sharing the helper
//!   would make the test check its own implementation.
//! - **#126** — generator template literals that contain generated-file
//!   headers can cluster with the generated output. That relationship is
//!   provenance, not duplicate implementation logic.
//!
//! The filter is purely additive: it never re-routes a `nearly_identical`
//! cluster as `identical`, only suppresses noise. Any cluster whose
//! member sources cannot be parsed (missing language plug-in, partial
//! source bytes) falls through unchanged.

use std::{
    collections::{BTreeSet, HashMap},
    hash::BuildHasher,
};

use tree_sitter::Node;

use crate::{ast::ByteRange, fingerprint::Fingerprint, lang::shared::parse_source, state::FileId};

/// Decides whether `cluster` is a known noise pattern that must not be
/// surfaced as duplication. Returns `true` when the cluster should be
/// hidden from the ranked report.
pub(crate) fn is_noise_pattern<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
) -> bool {
    let Some(language) = uniform_language(members, file_languages) else {
        return false;
    };
    let Some(snippets) = collect_snippets(members, sources, language) else {
        return false;
    };
    is_polymorphic_signature_cluster(&snippets)
        || is_rust_language_parser_adapter_cluster(&snippets)
        || is_generated_template_output_cluster(&snippets)
        || is_jwt_hmac_independent_verifier_cluster(&snippets)
        || is_literal_variation_call_cluster(&snippets)
        || is_monkeypatch_scaffolding_literal_cluster(&snippets)
        || is_python_all_exports_cluster(&snippets)
}

/// Detects **issue #114**: production HS256/JWT signing code and tests
/// that re-implement the same HMAC calculation independently. The
/// test-side duplication is intentional black-box verification; if the
/// test called the production minter/helper, it would stop proving the
/// signing implementation.
fn is_jwt_hmac_independent_verifier_cluster(snippets: &[Snippet<'_>]) -> bool {
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
/// are intentionally related, but the generated output is not a
/// refactor target and the template is already the source of truth.
fn is_generated_template_output_cluster(snippets: &[Snippet<'_>]) -> bool {
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

/// Returns true when `needle` occurs in `bytes`.
fn contains_bytes(bytes: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && bytes.windows(needle.len()).any(|window| window == needle)
}

/// Returns the bytes covered by one cluster occurrence.
fn snippet_range_text<'a>(snippet: &'a Snippet<'_>) -> Option<&'a [u8]> {
    snippet.source.get(snippet.range.start..snippet.range.end)
}

/// Keeps generated-file detection focused on file headers.
fn source_head(source: &[u8]) -> &[u8] {
    let end = source.len().min(1024);
    source.get(..end).unwrap_or(source)
}

/// Trims ASCII whitespace without allocating.
fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let first = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes.get(first..).unwrap_or_default()
}

/// Detects **issue #96**: every cluster member is a Python module-level
/// `__all__ = [...]` assignment. The export list shape is identical
/// across modules by convention, but the listed names always differ —
/// after Type-2 normalisation they look identical, yet the package
/// surface is not duplicate logic and cannot be extracted.
fn is_python_all_exports_cluster(snippets: &[Snippet<'_>]) -> bool {
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

/// Detects **issue #75**: the Rust source files that implement the
/// first-party language plug-ins all carry the same `LanguageParser`
/// adapter surface. Each implementation has language-specific constants
/// and grammar functions, but the trait contract forces the same method
/// outline, so the cluster is not actionable duplication.
fn is_rust_language_parser_adapter_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "rust") {
        return false;
    }
    let mut files = BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    if files.len() < 2 {
        return false;
    }
    let shapes: Option<Vec<RustImplShape>> = snippets
        .iter()
        .map(rust_language_parser_impl_shape)
        .collect();
    let Some(shapes) = shapes else { return false };
    let Some(first) = shapes.first() else {
        return false;
    };
    let expected_methods = language_parser_method_names();
    first.trait_name == b"LanguageParser"
        && first.methods == expected_methods
        && shapes
            .iter()
            .all(|shape| shape.trait_name == first.trait_name && shape.methods == expected_methods)
        && shapes
            .iter()
            .any(|shape| shape.impl_source != first.impl_source)
}

/// Parsed shape of one Rust `impl Trait for Type` block.
struct RustImplShape {
    /// Trait name from the `impl Trait for Type` header.
    trait_name: Vec<u8>,
    /// Method names declared directly inside the impl block.
    methods: BTreeSet<Vec<u8>>,
    /// Raw impl bytes used to avoid suppressing exact copies.
    impl_source: Vec<u8>,
}

/// Returns the `LanguageParser` impl contained in `snippet.range`.
fn rust_language_parser_impl_shape(snippet: &Snippet<'_>) -> Option<RustImplShape> {
    let tree = parse_for(snippet)?;
    let mut shapes = Vec::new();
    collect_rust_impl_shapes(tree.root_node(), snippet.range, snippet.source, &mut shapes);
    shapes
        .into_iter()
        .find(|shape| shape.trait_name == b"LanguageParser")
}

/// Recursively collects Rust impl blocks fully contained in `range`.
fn collect_rust_impl_shapes(
    node: Node<'_>,
    range: ByteRange,
    source: &[u8],
    out: &mut Vec<RustImplShape>,
) {
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return;
    }
    if node.kind() == "impl_item"
        && node.start_byte() >= range.start
        && node.end_byte() <= range.end
    {
        if let Some(shape) = rust_impl_shape_from_node(node, source) {
            out.push(shape);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_rust_impl_shapes(child, range, source, out);
    }
}

/// Extracts trait header and direct method names from one Rust impl node.
fn rust_impl_shape_from_node(node: Node<'_>, source: &[u8]) -> Option<RustImplShape> {
    let impl_source = source.get(node.start_byte()..node.end_byte())?;
    let header = impl_source.split(|byte| *byte == b'{').next()?;
    let header = std::str::from_utf8(header).ok()?.trim();
    let rest = header.strip_prefix("impl ")?;
    let (trait_name, _implementor) = rest.split_once(" for ")?;
    Some(RustImplShape {
        trait_name: trait_name.trim().as_bytes().to_vec(),
        methods: rust_impl_method_names(node, source),
        impl_source: impl_source.to_vec(),
    })
}

/// Returns method names declared directly under a Rust impl block.
fn rust_impl_method_names(node: Node<'_>, source: &[u8]) -> BTreeSet<Vec<u8>> {
    let mut methods = BTreeSet::new();
    collect_rust_function_names(node, source, &mut methods);
    methods
}

/// Walks Rust function items inside an impl block.
fn collect_rust_function_names(node: Node<'_>, source: &[u8], out: &mut BTreeSet<Vec<u8>>) {
    if node.kind() == "function_item" {
        if let Some(name) = node
            .child_by_field_name("name")
            .and_then(|name| source.get(name.start_byte()..name.end_byte()))
        {
            let _inserted = out.insert(name.to_vec());
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_rust_function_names(child, source, out);
    }
}

/// Required `LanguageParser` trait surface.
fn language_parser_method_names() -> BTreeSet<Vec<u8>> {
    BTreeSet::from([
        b"id".to_vec(),
        b"file_extensions".to_vec(),
        b"grammar".to_vec(),
        b"parse_and_normalize".to_vec(),
    ])
}

/// Returns true when `node` (an `expression_statement` or `assignment`)
/// binds the identifier `__all__` to a list/tuple literal.
fn python_node_assigns_all_exports(node: Node<'_>, source: &[u8]) -> bool {
    let assignment = python_descend_to_assignment(node);
    let Some(left) = assignment.child_by_field_name("left") else {
        return false;
    };
    if !python_node_text_equals(left, source, b"__all__") {
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

/// Returns true when `node`'s source byte range exactly matches `needle`.
fn python_node_text_equals(node: Node<'_>, source: &[u8], needle: &[u8]) -> bool {
    source.get(node.start_byte()..node.end_byte()) == Some(needle)
}

/// One re-parsed cluster member: language, raw bytes, the byte range
/// inside `source` that the fingerprint covered, and the originating
/// [`FileId`] so cross-file uniqueness checks do not depend on
/// pointer identity.
struct Snippet<'a> {
    /// Language id used to select the tree-sitter grammar.
    language: &'static str,
    /// Full file source bytes for the member.
    source: &'a [u8],
    /// Byte range covered by the member fingerprint.
    range: ByteRange,
    /// Registry id of the source file containing this member.
    file_id: FileId,
}

/// Returns a single language id when every member shares it.
fn uniform_language<S: BuildHasher>(
    members: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Option<&'static str> {
    let first = file_languages.get(&members.first()?.file_id)?;
    if members
        .iter()
        .all(|member| file_languages.get(&member.file_id) == Some(first))
    {
        Some(*first)
    } else {
        None
    }
}

/// Collects `(language, source, range)` tuples for every member, returning
/// `None` if any member's source bytes are unavailable.
fn collect_snippets<'a>(
    members: &[Fingerprint],
    sources: &'a HashMap<FileId, Vec<u8>>,
    language: &'static str,
) -> Option<Vec<Snippet<'a>>> {
    members
        .iter()
        .map(|member| {
            sources.get(&member.file_id).map(|source| Snippet {
                language,
                source: source.as_slice(),
                range: member.byte_range,
                file_id: member.file_id,
            })
        })
        .collect()
}

/// Detects **issue #69**: every cluster member is a function definition
/// (signature or whole `def`) whose declared name is the same identifier
/// and the members span at least two distinct files. That is the
/// abstract/interface implementation pattern — the contract forces
/// identity, no extraction is possible. We additionally require that
/// the enclosing function bodies are not byte-equivalent so a genuine
/// copy-pasted helper that happens to share a name (e.g. private
/// `_helper` reused in two modules) still fires as a cluster.
fn is_polymorphic_signature_cluster(snippets: &[Snippet<'_>]) -> bool {
    let names: Option<Vec<&[u8]>> = snippets.iter().map(enclosing_function_name).collect();
    let Some(names) = names else { return false };
    let Some(first_name) = names.first() else {
        return false;
    };
    if !names.iter().all(|name| name == first_name) {
        return false;
    }
    let mut files = std::collections::BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    if files.len() < 2 {
        return false;
    }
    enclosing_function_bodies_differ(snippets)
}

/// Returns true when at least two cluster members' enclosing function
/// bodies differ in raw source bytes — distinguishes polymorphism
/// (different implementations of one signature) from genuinely
/// duplicated helper functions that share a name.
fn enclosing_function_bodies_differ(snippets: &[Snippet<'_>]) -> bool {
    let bodies: Option<Vec<Vec<u8>>> = snippets
        .iter()
        .map(|snippet| {
            let tree = parse_for(snippet)?;
            let function = enclosing_kind(
                tree.root_node(),
                snippet.range,
                function_kinds(snippet.language),
            )?;
            let body = function.child_by_field_name("body")?;
            snippet
                .source
                .get(body.start_byte()..body.end_byte())
                .map(<[u8]>::to_vec)
        })
        .collect();
    let Some(bodies) = bodies else { return false };
    let Some(first) = bodies.first() else {
        return false;
    };
    bodies.iter().any(|body| body != first)
}

/// Returns the name of the `function_definition` (or `method_declaration`
/// for C#) that contains `snippet.range`, when one exists.
fn enclosing_function_name<'a>(snippet: &'a Snippet<'_>) -> Option<&'a [u8]> {
    let tree = parse_for(snippet)?;
    let function = enclosing_kind(
        tree.root_node(),
        snippet.range,
        function_kinds(snippet.language),
    )?;
    let name_node = function.child_by_field_name("name")?;
    snippet
        .source
        .get(name_node.start_byte()..name_node.end_byte())
}

/// Returns the set of tree-sitter node kinds that count as function
/// declarations for the purpose of polymorphism detection.
const fn function_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["function_definition"],
        b"csharp" => &["method_declaration", "local_function_statement"],
        b"rust" => &["function_item"],
        _ => &[],
    }
}

/// Detects **issues #70 / #71 / #72**: every cluster member is an
/// expression (or expression statement) whose top-level call has the
/// same callee chain across the cluster but **at least one string
/// literal argument differs**. That is the test-data / scaffolding
/// shape — extraction would erase intentional variation.
fn is_literal_variation_call_cluster(snippets: &[Snippet<'_>]) -> bool {
    let calls: Option<Vec<CallShape>> = snippets.iter().map(call_shape).collect();
    if is_literal_variation_call_set(calls) {
        return true;
    }
    is_literal_variation_call_sequence(snippets)
}

/// Applies the literal-variation rule to one comparable call per
/// cluster member.
fn is_literal_variation_call_set(calls: Option<Vec<CallShape>>) -> bool {
    let Some(calls) = calls else { return false };
    let Some(first) = calls.first() else {
        return false;
    };
    if !calls.iter().all(|call| call.callee == first.callee) {
        return false;
    }
    if !calls.iter().all(|call| call.arity == first.arity) {
        return false;
    }
    has_differing_string_literals(&calls)
}

/// Distilled view of a call expression used to compare cluster members.
#[derive(Clone)]
struct CallShape {
    /// Concrete callee string (e.g. `"client.delete"`,
    /// `"monkeypatch.setenv"`). Captured from the raw source so it
    /// retains identifier text that the normalised AST collapses.
    callee: Vec<u8>,
    /// Number of arguments to the call.
    arity: usize,
    /// Per-argument summary used for literal-variation detection.
    arguments: Vec<ArgShape>,
}

/// Per-argument summary recorded for each call.
#[derive(Clone)]
enum ArgShape {
    /// Raw bytes of a string-literal argument (or string content of an
    /// f-string / interpolated string).
    StringLiteral(Vec<u8>),
    /// Anything else — non-string literal, identifier, sub-expression.
    /// We only compare string literals for variation detection.
    Other,
}

/// Extracts the [`CallShape`] for the call expression covering
/// `snippet.range`. Returns `None` when no call is present.
fn call_shape(snippet: &Snippet<'_>) -> Option<CallShape> {
    let tree = parse_for(snippet)?;
    let call = enclosing_kind(
        tree.root_node(),
        snippet.range,
        call_kinds(snippet.language),
    )?;
    call_shape_from_node(call, snippet.source)
}

/// Extracts a [`CallShape`] from a concrete call node.
fn call_shape_from_node(call: Node<'_>, source: &[u8]) -> Option<CallShape> {
    let callee_node = call.child_by_field_name("function")?;
    let callee = source
        .get(callee_node.start_byte()..callee_node.end_byte())?
        .to_vec();
    let arguments = collect_argument_shapes(call, source);
    Some(CallShape {
        arity: arguments.len(),
        callee,
        arguments,
    })
}

/// Detects body-range clusters whose contained call sequence has the
/// same callees but intentionally different literal test data.
fn is_literal_variation_call_sequence(snippets: &[Snippet<'_>]) -> bool {
    let sequences: Option<Vec<Vec<CallShape>>> =
        snippets.iter().map(call_shapes_in_range).collect();
    let Some(sequences) = sequences else {
        return false;
    };
    let Some(first) = sequences.first() else {
        return false;
    };
    if first.is_empty() || !sequences.iter().all(|seq| same_call_headers(seq, first)) {
        return false;
    }
    (0..first.len()).any(|index| sequence_position_differs(&sequences, index))
}

/// Returns every call fully contained in `snippet.range`, preserving
/// source order.
fn call_shapes_in_range(snippet: &Snippet<'_>) -> Option<Vec<CallShape>> {
    let tree = parse_for(snippet)?;
    let mut shapes = Vec::new();
    collect_call_shapes(
        tree.root_node(),
        snippet.range,
        call_kinds(snippet.language),
        snippet.source,
        &mut shapes,
    );
    Some(shapes)
}

/// Recursively collects call nodes within `range`.
fn collect_call_shapes(
    node: Node<'_>,
    range: ByteRange,
    kinds: &[&str],
    source: &[u8],
    out: &mut Vec<CallShape>,
) {
    if node.end_byte() < range.start || node.start_byte() > range.end {
        return;
    }
    if node.start_byte() >= range.start
        && node.end_byte() <= range.end
        && kinds.contains(&node.kind())
    {
        if let Some(shape) = call_shape_from_node(node, source) {
            out.push(shape);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_call_shapes(child, range, kinds, source, out);
    }
}

/// Compares call sequence shape, ignoring literal payloads.
fn same_call_headers(calls: &[CallShape], expected: &[CallShape]) -> bool {
    calls.len() == expected.len()
        && calls
            .iter()
            .zip(expected)
            .all(|(call, base)| call.callee == base.callee && call.arity == base.arity)
}

/// Returns true when `index` has intentional literal variation across
/// all call sequences.
fn sequence_position_differs(sequences: &[Vec<CallShape>], index: usize) -> bool {
    let calls: Vec<CallShape> = sequences
        .iter()
        .filter_map(|sequence| sequence.get(index).cloned())
        .collect();
    calls.len() == sequences.len() && has_differing_string_literals(&calls)
}

/// Suppresses tiny string literal clusters inside pytest monkeypatch
/// setup tests. Issue #72 covers this exact scaffold: literals differ
/// intentionally because they are environment keys and values.
fn is_monkeypatch_scaffolding_literal_cluster(snippets: &[Snippet<'_>]) -> bool {
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

/// Walks `node` looking for an identifier with the requested bytes.
fn node_contains_identifier(node: Node<'_>, source: &[u8], needle: &[u8]) -> bool {
    if node.kind() == "identifier" && source.get(node.start_byte()..node.end_byte()) == Some(needle)
    {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| node_contains_identifier(child, source, needle));
    found
}

/// Returns the set of tree-sitter node kinds that count as call
/// expressions per language.
const fn call_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["call"],
        b"csharp" => &["invocation_expression"],
        b"rust" => &["call_expression", "macro_invocation"],
        _ => &[],
    }
}

/// Walks the named children of the call's `arguments`/`argument_list`
/// node and produces one [`ArgShape`] per argument.
fn collect_argument_shapes(call: Node<'_>, source: &[u8]) -> Vec<ArgShape> {
    let Some(args) = call
        .child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("argument_list"))
    else {
        return Vec::new();
    };
    let mut shapes = Vec::new();
    let mut cursor = args.walk();
    for arg in args.named_children(&mut cursor) {
        shapes.push(arg_shape(arg, source));
    }
    shapes
}

/// Classifies one argument node into [`ArgShape`].
fn arg_shape(node: Node<'_>, source: &[u8]) -> ArgShape {
    let inner = unwrap_argument(node);
    if let Some(bytes) = string_literal_bytes(inner, source) {
        return ArgShape::StringLiteral(bytes);
    }
    ArgShape::Other
}

/// Strips a C# `argument` wrapper down to its inner expression so the
/// literal-detection match arms below see the same shapes regardless of
/// language.
fn unwrap_argument(node: Node<'_>) -> Node<'_> {
    if node.kind() == "argument" {
        let mut cursor = node.walk();
        let child = node.named_children(&mut cursor).next();
        if let Some(child) = child {
            return child;
        }
    }
    node
}

/// Returns the bytes of `node` when it is a string-literal-like leaf.
/// Covers Python plain `string`, f-string, and C# `string_literal` /
/// `interpolated_string_expression` so f-string template differences in
/// issue #71 are captured.
fn string_literal_bytes(node: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    let kind = node.kind();
    let is_string = matches!(
        kind,
        "string"
            | "concatenated_string"
            | "string_literal"
            | "verbatim_string_literal"
            | "interpolated_string_expression"
    );
    if !is_string {
        return None;
    }
    source
        .get(node.start_byte()..node.end_byte())
        .map(<[u8]>::to_vec)
}

/// Returns true when at least one positional argument index has
/// differing string-literal bytes across the cluster. Non-string
/// arguments are ignored — the heuristic only fires when the
/// distinguishing variation is in literal text.
fn has_differing_string_literals(calls: &[CallShape]) -> bool {
    let Some(first) = calls.first() else {
        return false;
    };
    let mut saw_difference = false;
    let mut saw_string_arg = false;
    for index in 0..first.arguments.len() {
        let Some(ArgShape::StringLiteral(baseline)) = first.arguments.get(index) else {
            continue;
        };
        saw_string_arg = true;
        for call in calls.iter().skip(1) {
            match call.arguments.get(index) {
                Some(ArgShape::StringLiteral(bytes)) if bytes != baseline => {
                    saw_difference = true;
                }
                Some(ArgShape::StringLiteral(_)) => {}
                _ => {
                    return false;
                }
            }
        }
    }
    saw_string_arg && saw_difference
}

/// Walks `root` looking for the smallest descendant of `kinds` whose
/// byte range encloses `range`.
fn enclosing_kind<'tree>(
    root: Node<'tree>,
    range: ByteRange,
    kinds: &[&str],
) -> Option<Node<'tree>> {
    let mut best: Option<Node<'tree>> = None;
    let mut stack: Vec<Node<'tree>> = vec![root];
    while let Some(node) = stack.pop() {
        if node.start_byte() > range.start || node.end_byte() < range.end {
            continue;
        }
        if kinds.contains(&node.kind()) {
            best = Some(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }
    best
}

/// Parses the snippet's full source so we can walk a real tree-sitter
/// CST instead of the normalised one. Returns `None` when the language
/// has no registered grammar here.
fn parse_for(snippet: &Snippet<'_>) -> Option<tree_sitter::Tree> {
    let language = grammar_for(snippet.language)?;
    parse_source(snippet.language, &language, snippet.source).ok()
}

/// Maps a language id to its tree-sitter grammar.
fn grammar_for(language: &str) -> Option<tree_sitter::Language> {
    match language {
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "csharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        _ => None,
    }
}
