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
//!
//! The filter is purely additive: it never re-routes a `nearly_identical`
//! cluster as `identical`, only suppresses noise. Any cluster whose
//! member sources cannot be parsed (missing language plug-in, partial
//! source bytes) falls through unchanged.

mod calls;
mod python;
mod python_idioms;
mod python_orm;
mod rust;

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
        || rust::is_rust_language_parser_adapter_cluster(&snippets)
        || python_idioms::is_generated_template_output_cluster(&snippets)
        || python_idioms::is_jwt_hmac_independent_verifier_cluster(&snippets)
        || calls::is_literal_variation_call_cluster(&snippets)
        || python_idioms::is_monkeypatch_scaffolding_literal_cluster(&snippets)
        || python_idioms::is_python_all_exports_cluster(&snippets)
        || python::is_python_assertion_only_cluster(&snippets)
        || python::is_chained_dict_assert_cluster(&snippets)
        || python_orm::is_kwargs_only_constructor_cluster(&snippets)
        || python_orm::is_sqlalchemy_mapped_column_cluster(&snippets)
        || python::is_test_dict_literal_cluster(&snippets)
        || python::is_pytest_fixture_boilerplate_cluster(&snippets)
        || rust::is_rust_top_level_decl_cluster(&snippets)
        || rust::is_rust_iter_collect_idiom_cluster(&snippets)
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

/// One re-parsed cluster member: language, raw bytes, the byte range
/// inside `source` that the fingerprint covered, and the originating
/// [`FileId`] so cross-file uniqueness checks do not depend on
/// pointer identity.
pub(super) struct Snippet<'a> {
    /// Language id used to select the tree-sitter grammar.
    pub(super) language: &'static str,
    /// Full file source bytes for the member.
    pub(super) source: &'a [u8],
    /// Byte range covered by the member fingerprint.
    pub(super) range: ByteRange,
    /// Registry id of the source file containing this member.
    pub(super) file_id: FileId,
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

/// Parses the snippet's full source so we can walk a real tree-sitter
/// CST instead of the normalised one. Returns `None` when the language
/// has no registered grammar here.
pub(super) fn parse_for(snippet: &Snippet<'_>) -> Option<tree_sitter::Tree> {
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
