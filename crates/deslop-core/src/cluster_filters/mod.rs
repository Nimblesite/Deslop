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
//! - **#99** — pure Python assertion blocks in tests share AST shape and
//!   token alphabet while intentionally checking different concrete values.
//! - **#100** [CLONE-NOISE-PY-KWARGS-CTOR] — ORM/dataclass/Pydantic
//!   constructor calls with shared field shape but distinct field names
//!   cluster after identifier normalisation. The constructor's purpose
//!   IS to enumerate per-model fields; extraction is impossible.
//! - **#105** [CLONE-NOISE-PY-MAPPED-COLUMN] — `SQLAlchemy`
//!   `Mapped[T] = mapped_column(...)` declaration blocks across distinct
//!   ORM model classes cluster via token Jaccard alone. Each block is a
//!   different table schema.
//! - **#107** [CLONE-NOISE-PY-DICT-ASSERT] — chained `assert X[k1][k2]`
//!   assertions across unrelated pytest test functions share AST shape
//!   but verify different response/payload contracts.
//! - **#112** [CLONE-NOISE-PY-DICT-FIXTURE] — small nested dict literals
//!   inside pytest test functions share AST shape across files but
//!   encode unrelated request/response payloads.
//! - **#121** — async `SQLAlchemy` row-building pytest fixtures repeat the
//!   same add/commit/refresh/return setup idiom by design.
//! - **#150** — `mod e0001;` / `use foo::Bar;` top-level declarations
//!   cluster across registries because Rust module statements cannot be
//!   macro-generated. They are language scaffolding, not logic.
//! - **#147** — `xs.iter().map(|x| x.field.as_str()).collect()` is a
//!   pure language idiom that clusters across unrelated element types.
//!   Extracting it would require a cross-crate trait, not deduplication.
//! - **#115a** [CLONE-NOISE-PY-STRENUM-CLASS-SHAPE] — `class X(StrEnum)`
//!   declarations carry an identical docstring + assignment body shape
//!   but each enum is a closed discriminator with distinct vocabulary.
//! - **#115b** [CLONE-NOISE-PY-PYDANTIC-PARTIAL] — Pydantic's
//!   `XCreate` / `XUpdate` mirror is mandated by the framework's lack
//!   of a native `PartialModel`.
//! - **#115c** [CLONE-NOISE-PY-WORKSPACE-LOCAL-MIRROR] — out of scope
//!   for a generic filter. The cross-tree duplication between
//!   `src/agent_backend/api/schemas.py` and a sandboxed
//!   `workspaces/.../dispatcher.py` is mandated by the workspace's
//!   inability to import from the backend. The architectural intent
//!   is invisible to the analyser, so users suppress the FP via
//!   `.deslop.toml` `[language.python] exclude = ["workspaces/.../*"]`
//!   (the existing config exclude already supports this).
//! - **#97**  [CLONE-NOISE-PY-PARAMETRIC-INVARIANT-TESTS] — `def
//!   test_register_<variant>()` tests that vary only by enum-member
//!   access tokens are spec assertions, not extractable duplication.
//! - **#154** [CLONE-NOISE-SIGNATURE-ONLY] — every cluster member's
//!   matched subtree is the *signature* (parameter list, return type) of
//!   a function, never the body. After normalisation these collapse to
//!   identical shape but the bodies are unrelated. Token Jaccard cannot
//!   refute the match because identifiers normalise away too.
//! - **#104** [CLONE-NOISE-PY-MODULE-PREAMBLE] — a sibling-window
//!   fingerprint can span a run of >=2 module-level helper/fixture
//!   definitions. Two test files whose preambles share that shape cluster
//!   even though every helper body differs. Suppressed only when no two
//!   members share identical bodies, so a verbatim/renamed copy survives.
//!
//! The filter is purely additive: it never re-routes a `nearly_identical`
//! cluster as `identical`, only suppresses noise. Any cluster whose
//! member sources cannot be parsed (missing language plug-in, partial
//! source bytes) falls through unchanged.

mod calls;
mod dart;
mod python;
mod python_class_shapes;
mod python_idioms;
mod python_module_preamble;
mod python_orm;
mod role_compat;
mod rust;
mod snippets;

use std::{
    collections::{BTreeSet, HashMap},
    hash::BuildHasher,
};

use tree_sitter::Node;

pub(crate) use snippets::ParseCache;
use snippets::{collect_snippets, parse_for, uniform_language, Snippet};

use crate::{ast::ByteRange, fingerprint::Fingerprint, state::FileId};

/// Decides whether `cluster` is a known noise pattern that must not be
/// surfaced as duplication. Returns `true` when the cluster should be
/// hidden from the ranked report.
pub(crate) fn is_noise_pattern<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> bool {
    let Some(language) = uniform_language(members, file_languages) else {
        return false;
    };
    let Some(snippets) = collect_snippets(members, sources, language, cache) else {
        return false;
    };
    // Generic, language-agnostic noise checks run for every language (they
    // key off per-language kind maps). The language-specific idiom filters
    // only fire for their own language, so gate them by `language` rather
    // than walking every Dart/C# cluster's CST through Python/Rust matchers
    // that can never match — that wasted walk dominated analysis time on
    // large codegen-heavy repos ([CLONE-NOISE-REPARSE-CACHE]).
    is_polymorphic_signature_cluster(&snippets)
        || is_signature_only_cluster(&snippets)
        || calls::is_literal_variation_call_cluster(&snippets)
        || language_specific_noise(language, &snippets)
}

/// Language-specific idiom filters, dispatched by language so a cluster is
/// only walked by matchers that can fire for it. C# and Dart have no
/// idiom filters today — they rely on the generic checks plus the fusion
/// and report-hide gates.
fn language_specific_noise(language: &str, snippets: &[Snippet<'_>]) -> bool {
    match language {
        "dart" => dart::is_dart_class_field_declaration_cluster(snippets),
        "python" => python_noise(snippets),
        "rust" => rust_noise(snippets),
        _ => false,
    }
}

/// All Python idiom noise filters (issues #96/#97/#99/#100/#104/#105/#107/
/// #112/#114/#115/#121/#126 and monkeypatch scaffolding).
fn python_noise(snippets: &[Snippet<'_>]) -> bool {
    python_idioms::is_generated_template_output_cluster(snippets)
        || python_idioms::is_jwt_hmac_independent_verifier_cluster(snippets)
        || python_idioms::is_monkeypatch_scaffolding_literal_cluster(snippets)
        || python_idioms::is_python_all_exports_cluster(snippets)
        || python::is_python_assertion_only_cluster(snippets)
        || python::is_chained_dict_assert_cluster(snippets)
        || python_orm::is_kwargs_only_constructor_cluster(snippets)
        || python_orm::is_sqlalchemy_mapped_column_cluster(snippets)
        || python::is_test_dict_literal_cluster(snippets)
        || python::is_pytest_fixture_boilerplate_cluster(snippets)
        || python_class_shapes::is_strenum_class_shape_cluster(snippets)
        || python_class_shapes::is_pydantic_partial_update_cluster(snippets)
        || python::is_parametric_invariant_test_cluster(snippets)
        || python_module_preamble::is_module_preamble_sequence_cluster(snippets)
}

/// All Rust idiom noise filters (issues #75/#147/#150/#155).
fn rust_noise(snippets: &[Snippet<'_>]) -> bool {
    rust::is_rust_language_parser_adapter_cluster(snippets)
        || rust::is_rust_top_level_decl_cluster(snippets)
        || rust::is_rust_iter_collect_idiom_cluster(snippets)
}

/// Decides whether an embedding-dominant `same_behavior` cluster pairs
/// members of incompatible top-level roles — a class/type definition
/// with a function/method (issue **#119**
/// [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]). Returns `true` when the
/// cluster should be hidden. The caller restricts this to the
/// `same_behavior` bucket so deterministic Type-1/2/3 clusters are
/// untouched. Falls through to `false` when sources or a uniform
/// language are unavailable.
pub(crate) fn is_embedding_role_mismatch<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> bool {
    let Some(language) = uniform_language(members, file_languages) else {
        return false;
    };
    let Some(snippets) = collect_snippets(members, sources, language, cache) else {
        return false;
    };
    role_compat::is_role_incompatible_embedding_match(&snippets)
}

/// Trims surrounding ASCII whitespace from a reported snippet range so
/// parser lookups are not defeated by trailing newlines outside the AST
/// node that produced the fingerprint.
pub(super) fn trimmed_snippet_range(snippet: &Snippet<'_>) -> Option<ByteRange> {
    let bytes = snippet_range_text(snippet)?;
    let leading = bytes.iter().position(|byte| !byte.is_ascii_whitespace())?;
    let trailing = bytes.iter().rposition(|byte| !byte.is_ascii_whitespace())?;
    let start = snippet.range.start.checked_add(leading)?;
    let end = snippet.range.start.checked_add(trailing)?.checked_add(1)?;
    Some(ByteRange { start, end })
}

/// Returns true when `node` overlaps `range`.
pub(super) fn node_intersects_range(node: Node<'_>, range: ByteRange) -> bool {
    node.start_byte() < range.end && node.end_byte() > range.start
}

/// Returns true when at least two raw reported snippet ranges differ.
pub(super) fn raw_snippet_texts_differ(snippets: &[Snippet<'_>]) -> bool {
    let Some(first) = snippets.first().and_then(snippet_range_text) else {
        return false;
    };
    snippets
        .iter()
        .filter_map(snippet_range_text)
        .any(|text| text != first)
}

/// Walks `node` looking for a named descendant of `kind`.
pub(super) fn node_contains_kind(node: Node<'_>, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| node_contains_kind(child, kind));
    found
}

/// Returns true when `needle` occurs in `bytes`.
pub(super) fn contains_bytes(bytes: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && bytes.windows(needle.len()).any(|window| window == needle)
}

/// Returns the bytes covered by one cluster occurrence.
pub(super) fn snippet_range_text<'a>(snippet: &'a Snippet<'_>) -> Option<&'a [u8]> {
    snippet.source.get(snippet.range.start..snippet.range.end)
}

/// Keeps generated-file detection focused on file headers.
pub(super) fn source_head(source: &[u8]) -> &[u8] {
    let end = source.len().min(1024);
    source.get(..end).unwrap_or(source)
}

/// Trims ASCII whitespace without allocating.
pub(super) fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let first = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes.get(first..).unwrap_or_default()
}

/// Detects **issue #154** [CLONE-NOISE-SIGNATURE-ONLY]: every cluster
/// member's matched subtree sits entirely inside a function/method
/// signature — it does not overlap the body. After identifier and
/// literal normalisation, `fn check_foo(ctx: &mut Ctx)` collapses to
/// the same shape as `fn check_bar(ctx: &mut Ctx)`, so the structural
/// fingerprint is 1.0 but the function bodies are unrelated. Token
/// Jaccard cannot refute the match because identifier normalisation
/// erases the distinguishing tokens too.
///
/// We suppress these clusters only when at least two of the enclosing
/// function bodies differ in raw source bytes. A real Type-2 clone
/// where two functions share the same signature and body would have
/// identical body bytes, so this check keeps genuine duplication.
fn is_signature_only_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 {
        return false;
    }
    let shapes: Option<Vec<Vec<String>>> = snippets
        .iter()
        .map(snippet_body_shape_when_signature_only)
        .collect();
    let Some(shapes) = shapes else { return false };
    let Some(first) = shapes.first() else {
        return false;
    };
    shapes.iter().any(|shape| shape != first)
}

/// Returns the enclosing function body's AST node-kind sequence when
/// `snippet.range` lies entirely inside that function's signature
/// (before the body) — the signature-only match condition for
/// [CLONE-NOISE-SIGNATURE-ONLY]. The node-kind sequence is the
/// flattened, ordered list of every named descendant's kind so two
/// bodies that share AST shape (and differ only by literals/identifiers)
/// compare equal — i.e. a legitimate near-miss cluster keeps clustering.
/// Returns `None` when the snippet is not contained in a function, when
/// the function has no `body` field, or when the range intersects the
/// body in any way.
fn snippet_body_shape_when_signature_only(snippet: &Snippet<'_>) -> Option<Vec<String>> {
    let tree = parse_for(snippet)?;
    let function = enclosing_kind(
        tree.root_node(),
        snippet.range,
        function_kinds(snippet.language),
    )?;
    let body = function.child_by_field_name("body")?;
    if snippet.range.end > body.start_byte() {
        return None;
    }
    let mut kinds: Vec<String> = Vec::new();
    collect_named_kinds(body, &mut kinds);
    Some(kinds)
}

/// Pushes every named descendant's `kind` into `kinds` in source order.
/// Used by [`snippet_body_shape_when_signature_only`] so cluster members
/// whose bodies share AST shape compare equal regardless of literal or
/// identifier divergence.
fn collect_named_kinds(node: Node<'_>, kinds: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        kinds.push(child.kind().to_owned());
        collect_named_kinds(child, kinds);
    }
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
    let mut files = BTreeSet::new();
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
    let name_node = function_name_node(function)?;
    snippet
        .source
        .get(name_node.start_byte()..name_node.end_byte())
}

/// Resolves the identifier node that names `function`. Python, C#, and
/// Rust expose a direct `name` field on the function node. Dart instead
/// nests it under `signature` — `function_signature.name` for a top-level
/// `function_declaration`, and `method_signature → function_signature.name`
/// for a `method_declaration`. Without this descent
/// [`enclosing_function_name`] returns `None` for every Dart member, so
/// the polymorphic-signature filter (#69) could never fire on Dart even
/// though `function_kinds` lists its node kinds.
fn function_name_node<'tree>(function: Node<'tree>) -> Option<Node<'tree>> {
    if let Some(name) = function.child_by_field_name("name") {
        return Some(name);
    }
    let signature = function.child_by_field_name("signature")?;
    if let Some(name) = signature.child_by_field_name("name") {
        return Some(name);
    }
    let mut cursor = signature.walk();
    let nested = signature
        .named_children(&mut cursor)
        .find_map(|child| child.child_by_field_name("name"));
    nested
}

/// Returns the set of tree-sitter node kinds that count as function
/// declarations for the purpose of polymorphism detection.
const fn function_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["function_definition"],
        b"csharp" => &["method_declaration", "local_function_statement"],
        b"rust" => &["function_item"],
        // Dart `function_declaration`/`method_declaration` both expose a
        // `body` field, so the signature-only filter (#154) can compare
        // bodies after a signature-only structural match.
        b"dart" => &["function_declaration", "method_declaration"],
        _ => &[],
    }
}

/// Walks `node` looking for an identifier with the requested bytes.
pub(super) fn node_contains_identifier(node: Node<'_>, source: &[u8], needle: &[u8]) -> bool {
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

/// Walks `root` looking for the smallest descendant of `kinds` whose
/// byte range encloses `range`.
pub(super) fn enclosing_kind<'tree>(
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
