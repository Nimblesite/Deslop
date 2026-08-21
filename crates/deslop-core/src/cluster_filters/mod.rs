//! Cluster-level false-positive filters applied during report rendering.
//!
//! Noise patterns that survive structural fingerprinting because Type-2
//! normalisation collapses identifiers and literals. We re-parse each
//! cluster member's original source bytes and look at the *real* tree —
//! the same way a reviewer would — so we can drop clusters that the
//! pipeline cannot tell apart but a human instantly would.
//!
//! Issues addressed:
//! - abstract method signatures cluster across every concrete
//!   subclass that implements an `ABC`. The 37-node subtree is the
//!   parameter list; identical-by-contract is not duplication.
//! - `_tool_call_response("write_file", {...}, "id")` and
//!   peers cluster when only string literal args vary. Test fixture
//!   data, not duplication.
//! - REST endpoint tests cluster on the HTTP-call shape but
//!   the f-string templates encode different routes. Different routes
//!   are different tests, not duplication.
//! - `monkeypatch.setenv("KEY", "VAL")` test scaffolding
//!   clusters because every two-setenv test looks the same after
//!   normalisation. Scaffolding is not logic.
//! - every first-party Rust language plug-in implements the
//!   same `LanguageParser` trait surface. The adapter shape is required
//!   by the trait contract, not extractable business logic.
//! - tests can independently re-implement HS256/JWT signing
//!   to verify a production minter as a black box. Sharing the helper
//!   would make the test check its own implementation.
//! - generator template literals that contain generated-file
//!   headers can cluster with the generated output. That relationship is
//!   provenance, not duplicate implementation logic.
//! - pure Python assertion blocks in tests share AST shape and
//!   token alphabet while intentionally checking different concrete values.
//! - [CLONE-NOISE-PY-KWARGS-CTOR] — ORM/dataclass/Pydantic
//!   constructor calls with shared field shape but distinct field names
//!   cluster after identifier normalisation. The constructor's purpose
//!   IS to enumerate per-model fields; extraction is impossible.
//! - [CLONE-NOISE-PY-MAPPED-COLUMN] — `SQLAlchemy`
//!   `Mapped[T] = mapped_column(...)` declaration blocks across distinct
//!   ORM model classes cluster via token Jaccard alone. Each block is a
//!   different table schema.
//! - [CLONE-NOISE-PY-DICT-ASSERT] — chained `assert X[k1][k2]`
//!   assertions across unrelated pytest test functions share AST shape
//!   but verify different response/payload contracts.
//! - [CLONE-NOISE-PY-DICT-FIXTURE] — small nested dict literals
//!   inside pytest test functions share AST shape across files but
//!   encode unrelated request/response payloads.
//! - [CLONE-NOISE-PY-COLLECTION-SIBLING-CELLS] — two entries of one
//!   collection literal instance admit as a structural pair at a
//!   permissive `--min-nodes`. Cells of one record are its fields, not
//!   extractable duplication; a byte-identical repeated entry still
//!   surfaces.
//! - async `SQLAlchemy` row-building pytest fixtures repeat the
//!   same add/commit/refresh/return setup idiom by design.
//! - `mod e0001;` / `use foo::Bar;` top-level declarations
//!   cluster across registries because Rust module statements cannot be
//!   macro-generated. They are language scaffolding, not logic.
//! - `xs.iter.map(|x| x.field.as_str).collect` is a
//!   pure language idiom that clusters across unrelated element types.
//!   Extracting it would require a cross-crate trait, not deduplication.
//! - [CLONE-NOISE-RUST-MATCH-DISPATCH] — runs of `match` arms in
//!   a dispatch `match` collapse to one `<path>::<ident> => Ok(<call>(..))`
//!   shape under Type-2 normalisation, so the sibling-window pass matches
//!   one window of arms against another within the same `match`. A routing
//!   table maps distinct keys to distinct handlers; it is not extractable.
//!   Suppressed only when the arm patterns are pairwise distinct and two
//!   members differ in raw bytes, so a verbatim copied run still surfaces.
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
//! - [CLONE-NOISE-PY-PARAMETRIC-INVARIANT-TESTS] — `def
//!   test_register_<variant>()` tests that vary only by enum-member
//!   access tokens are spec assertions, not extractable duplication.
//! - [CLONE-NOISE-SIGNATURE-ONLY] — every cluster member's
//!   matched subtree is the *signature* (parameter list, return type) of
//!   a function, never the body. After normalisation these collapse to
//!   identical shape but the bodies are unrelated. Token Jaccard cannot
//!   refute the match because identifiers normalise away too.
//! - [CLONE-NOISE-PY-MODULE-PREAMBLE] — a sibling-window
//!   fingerprint can span a run of >=2 module-level helper/fixture
//!   definitions. Two test files whose preambles share that shape cluster
//!   even though every helper body differs. Suppressed only when no two
//!   members share identical bodies, so a verbatim/renamed copy survives.
//! - Dart const data registries (`static const Foo NAME =
//!   Foo(<distinct values>);` icon/colour/token tables) cluster via
//!   sibling-window fingerprints over runs of field/const declarations.
//!   They are un-refactorable data, not logic. Suppressed only when the
//!   members differ in raw bytes (a verbatim copy survives) and none holds
//!   a closure/lambda initialiser (logic-bearing fields keep clustering).
//! - [CLONE-NOISE-CONSTANT-TABLE] — a range that is just a run of
//!   module-level `NAME = <literal>` declarations (SQL query strings,
//!   registry/config values, a test suite's data blobs) normalises to the
//!   same subtree as any other such table after identifier/literal/comment
//!   stripping, so two unrelated tables cluster at `structural=1.00`. A
//!   table of distinct named constants is data, not extractable logic.
//!   One rule, per-language only in the grammar of "a top-level constant
//!   declaration": Python `NAME = <literal>` (#133) and Rust `const` /
//!   `static` items (#362). Suppressed only when the members differ in raw
//!   bytes (a verbatim copy survives).
//!
//! The filter is purely additive: it never re-routes a `nearly_identical`
//! cluster as `identical`, only suppresses noise. Any cluster whose
//! member sources cannot be parsed (missing language plug-in, partial
//! source bytes) falls through unchanged.

mod body_shape;
mod calls;
mod constant_table;
mod contract_index;
mod dart;
mod dart_data_table;
mod declaration_family;
mod ecmascript;
mod family;
mod forwarding;
mod polymorphic;
mod python;
mod python_class_shapes;
mod python_collection_cells;
mod python_dict_assert;
mod python_idioms;
mod python_module_preamble;
mod python_orm;
mod role_compat;
mod rust;
mod snippets;
mod structural_families;
mod verbatim_subgroup;

use std::{
    collections::{BTreeSet, HashMap},
    hash::BuildHasher,
};

use tree_sitter::Node;

pub(crate) use declaration_family::is_single_file_declaration_family;
pub(crate) use snippets::ParseCache;
use snippets::{collect_snippets, parse_for, uniform_language, Snippet};
pub(crate) use structural_families::split_structural_families;
pub(crate) use verbatim_subgroup::split_noise_verbatim_families;

use crate::{
    ast::ByteRange, clone_category::CloneCategory, fingerprint::Fingerprint, state::FileId,
};

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
    polymorphic::is_polymorphic_signature_cluster(&snippets, sources, file_languages, cache)
        || is_signature_only_cluster(&snippets)
        || calls::is_literal_variation_call_cluster(&snippets)
        || constant_table::is_constant_table_cluster(&snippets)
        || language_specific_noise(language, &snippets)
}

/// Classifies a cluster's [`CloneCategory`] ([RANK-CATEGORY]) by re-parsing
/// its member sources the same way [`is_noise_pattern`] does. Returns
/// [`CloneCategory::DataTable`] for a data-structure literal whose repeated
/// rows are un-refactorable data; otherwise [`CloneCategory::Logic`]. The
/// verbatim escape hatch (`raw_snippet_texts_differ`) lives inside each
/// per-language predicate, so a byte-for-byte copied table stays `Logic`.
///
/// Distinct from `is_noise_pattern`: a `DataTable` is real repetition the
/// user *may* act on (a builder, an asset file), so the policy demotes or
/// drops it per config rather than silently hiding it.
pub(crate) fn classify_clone_category<S: BuildHasher>(
    members: &[Fingerprint],
    literal_fraction: f64,
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> CloneCategory {
    let Some(language) = uniform_language(members, file_languages) else {
        return CloneCategory::Logic;
    };
    let Some(snippets) = collect_snippets(members, sources, language, cache) else {
        return CloneCategory::Logic;
    };
    if is_data_table_cluster(language, literal_fraction, &snippets) {
        CloneCategory::DataTable
    } else {
        CloneCategory::Logic
    }
}

/// Dispatches data-table detection. The language-agnostic
/// literal-dominance test ([CLONE-NOISE-LITERAL-TABLE]) covers
/// pure value tables in every language; the Dart predicate additionally
/// recognises collection literals of constructor rows
/// ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]), whose identifier-heavy rows
/// sit below the literal-dominance floor.
fn is_data_table_cluster(language: &str, literal_fraction: f64, snippets: &[Snippet<'_>]) -> bool {
    is_literal_dominated_table(literal_fraction, snippets)
        || match language {
            "dart" => dart_data_table::is_dart_collection_data_table_cluster(snippets),
            _ => false,
        }
}

/// Language-agnostic data-table test ([CLONE-NOISE-LITERAL-TABLE]): the
/// cluster's normalised leaves are overwhelmingly literal positions —
/// measured in the pipeline, where the normalised trees live — and at
/// least two members differ in raw bytes. The verbatim escape hatch is
/// the same #190 rule the Dart predicate applies: a byte-for-byte
/// copied table is genuine duplication and stays `logic`.
fn is_literal_dominated_table(literal_fraction: f64, snippets: &[Snippet<'_>]) -> bool {
    literal_fraction >= crate::buckets::LITERAL_TABLE_MIN_FRACTION
        && snippets.len() >= 2
        && raw_snippet_texts_differ(snippets)
}

/// Language-specific idiom filters, dispatched by language so a cluster is
/// only walked by matchers that can fire for it. C# has no idiom filter
/// today; Dart suppresses const-data-registry field clusters. Both
/// also rely on the generic checks plus the fusion and report-hide gates.
fn language_specific_noise(language: &str, snippets: &[Snippet<'_>]) -> bool {
    match language {
        "dart" => {
            dart::is_dart_class_field_declaration_cluster(snippets)
                || dart::is_dart_widget_scaffold_cluster(snippets)
        }
        "python" => python_noise(snippets),
        "rust" => rust_noise(snippets),
        "javascript" | "typescript" | "tsx" => {
            ecmascript::is_ecmascript_data_shape_cluster(snippets)
        }
        _ => false,
    }
}

/// All Python idiom noise filters (/
/// #112/#114/#115/#121/#126/#133 and monkeypatch scaffolding).
fn python_noise(snippets: &[Snippet<'_>]) -> bool {
    python_idioms::is_generated_template_output_cluster(snippets)
        || python_idioms::is_jwt_hmac_independent_verifier_cluster(snippets)
        || python_idioms::is_monkeypatch_scaffolding_literal_cluster(snippets)
        || python_idioms::is_python_all_exports_cluster(snippets)
        || python::is_python_assertion_only_cluster(snippets)
        || python_dict_assert::is_chained_dict_assert_cluster(snippets)
        || python_orm::is_kwargs_only_constructor_cluster(snippets)
        || python_orm::is_sqlalchemy_mapped_column_cluster(snippets)
        || python::is_test_dict_literal_cluster(snippets)
        || python::is_pytest_fixture_boilerplate_cluster(snippets)
        || python_collection_cells::is_collection_sibling_cell_cluster(snippets)
        || python_class_shapes::is_strenum_class_shape_cluster(snippets)
        || python_class_shapes::is_pydantic_partial_update_cluster(snippets)
        || python::is_parametric_invariant_test_cluster(snippets)
        || python_module_preamble::is_module_preamble_sequence_cluster(snippets)
}

/// All Rust idiom noise filters.
fn rust_noise(snippets: &[Snippet<'_>]) -> bool {
    rust::is_rust_language_parser_adapter_cluster(snippets)
        || rust::is_rust_top_level_decl_cluster(snippets)
        || rust::is_rust_iter_collect_idiom_cluster(snippets)
        || rust::is_rust_match_dispatch_cluster(snippets)
        || rust::is_rust_struct_field_declaration_cluster(snippets)
}

/// Decides whether an embedding-dominant `same_behavior` cluster pairs
/// members of incompatible top-level roles — a class/type definition
/// with a function/method (issue
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

/// Returns true when `file_ids` cover at least two distinct files.
///
/// Cluster-noise filters require cross-file spread before they fire: a
/// pattern repeated only within a single file is not the cross-file
/// duplication they guard against. Centralising the `BTreeSet` count keeps
/// every per-language filter's "≥ 2 distinct files" gate identical.
pub(super) fn spans_multiple_files(file_ids: impl IntoIterator<Item = FileId>) -> bool {
    file_ids.into_iter().collect::<BTreeSet<FileId>>().len() >= 2
}

/// Returns true when `snippets` is a multi-member cluster whose every member
/// is written in `language`. This is the precondition every language-specific
/// cluster filter shares before it inspects member shapes — centralising it
/// keeps the "≥ 2 members, all one language" gate identical across filters.
pub(super) fn is_multi_member_language_cluster(snippets: &[Snippet<'_>], language: &str) -> bool {
    snippets.len() >= 2 && snippets.iter().all(|snippet| snippet.language == language)
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

/// Detects **** [CLONE-NOISE-SIGNATURE-ONLY]: every cluster
/// member's matched subtree sits entirely inside a function/method
/// signature — it does not overlap the body. After identifier and
/// literal normalisation, `fn check_foo(ctx: &mut Ctx)` collapses to
/// the same shape as `fn check_bar(ctx: &mut Ctx)`, so the structural
/// fingerprint is 1.0 but the function bodies are unrelated. Token
/// Jaccard cannot refute the match because identifier normalisation
/// erases the distinguishing tokens too.
///
/// We suppress these clusters only when at least two of the enclosing
/// function bodies differ as normalised trees
/// ([`body_shape::body_kind_stream`]). A real Type-1/Type-2 clone
/// where two functions share the same signature and body — byte for
/// byte or under a consistent rename — has equal streams, so genuine
/// duplication keeps clustering.
fn is_signature_only_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 {
        return false;
    }
    let shapes: Option<Vec<Vec<body_shape::ShapeToken<'_>>>> = snippets
        .iter()
        .map(snippet_body_shape_when_signature_only)
        .collect();
    let Some(shapes) = shapes else { return false };
    let Some(first) = shapes.first() else {
        return false;
    };
    shapes.iter().any(|shape| shape != first)
}

/// Returns the enclosing function body's normalised kind stream
/// ([`body_shape::body_kind_stream`]) when `snippet.range` lies
/// entirely inside that function's signature (before the body) — the
/// signature-only match condition for [CLONE-NOISE-SIGNATURE-ONLY].
/// Two bodies that share AST shape (and differ only by literals,
/// identifiers, or comments) compare equal — i.e. a legitimate
/// near-miss cluster keeps clustering. Returns `None` when the snippet
/// is not contained in a function, when the function has no `body`
/// field, or when the range intersects the body in any way.
fn snippet_body_shape_when_signature_only<'src>(
    snippet: &Snippet<'src>,
) -> Option<Vec<body_shape::ShapeToken<'src>>> {
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
    Some(body_shape::body_kind_stream(body, snippet.source))
}

/// Returns the set of tree-sitter node kinds that count as function
/// declarations for the purpose of polymorphism detection.
pub(super) const fn function_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["function_definition"],
        b"csharp" => &["method_declaration", "local_function_statement"],
        b"rust" => &["function_item"],
        // Dart `function_declaration`/`method_declaration` both expose a
        // `body` field, so the signature-only filter can compare
        // bodies after a signature-only structural match.
        b"dart" => &["function_declaration", "method_declaration"],
        // JS/TS/TSX function forms that expose a `body` field, so the
        // signature-only and polymorphic-signature filters can
        // compare bodies after a signature-only structural match — parity
        // with the other languages.
        b"javascript" | b"typescript" | b"tsx" => &[
            "function_declaration",
            "generator_function_declaration",
            "method_definition",
            "function_expression",
            "arrow_function",
        ],
        // PHP functions and class methods that expose a `body` field so the
        // signature-only and polymorphic-signature filters can
        // compare bodies after a signature-only structural match.
        b"php" => &["function_definition", "method_declaration"],
        // F# `let`/`member` bindings both expose a `body` field so the
        // signature-only and polymorphic-signature filters can
        // compare bodies after a signature-only structural match. Only
        // `method_or_prop_defn` carries a `name` field (members), so #69 is
        // limited to members; top-level `let` bindings still get #154.
        b"fsharp" => &["function_or_value_defn", "method_or_prop_defn"],
        // Go functions, methods, and closures all expose a `body` field so
        // the signature-only and polymorphic-signature filters
        // can compare bodies after a signature-only structural match.
        b"go" => &["function_declaration", "method_declaration", "func_literal"],
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
pub(crate) fn enclosing_kind<'tree>(
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
