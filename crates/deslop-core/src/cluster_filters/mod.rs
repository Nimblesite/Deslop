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
//!   declaration": Python `NAME = <literal>` (#133), Rust `const` /
//!   `static` items (#362), ECMAScript `const`/`let`/`var`, and F#
//!   module-level `let NAME = <literal>` bindings. Suppressed only when
//!   the members differ in raw bytes (a verbatim copy survives).
//!
//! - [CLONE-NOISE-DATA-ROWS] — a window of table rows normalises to the
//!   same subtree as any other window of rows with the same shape. Rows
//!   holding no logic are data, not extractable duplication.
//! - [CLONE-NOISE-PHP-ACCESSORS] — properties, constructors storing their
//!   parameters and getters returning them share one shape in every PHP
//!   value class.
//! - [CLONE-NOISE-GO-FIXTURE-TEST] — Go tests whose scenario is one string
//!   share only the harness calls around it.
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
mod data_rows;
mod declaration_family;
mod ecmascript;
mod family;
mod forwarding;
mod fsharp_scaffolding;
mod go_fixture_test;
mod helpers;
mod node_search;
mod override_marker;
mod php_accessors;
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
mod signature_only;
mod snippets;
mod structural_families;
mod verbatim_subgroup;

use std::{collections::HashMap, hash::BuildHasher};

pub(crate) use declaration_family::is_single_file_declaration_family;
pub(crate) use helpers::enclosing_kind;
use helpers::{
    contains_bytes, is_multi_member_language_cluster, language_cluster_shapes,
    node_contains_identifier, node_contains_kind, node_intersects_range, raw_snippet_texts_differ,
    snippet_range_text, source_head, spans_multiple_files, trim_ascii_start, trimmed_snippet_range,
};
pub(crate) use signature_only::function_kinds;
use signature_only::{declaration_shell_kinds, is_signature_only_cluster};
pub(crate) use snippets::locked;
pub use snippets::ParseCache;
use snippets::{collect_snippets, parse_for, uniform_language, Snippet};
pub(crate) use structural_families::split_structural_families;
pub(crate) use verbatim_subgroup::{
    escapes_as_copy, noise_workers, split_noise_verbatim_families, NOISE_CHUNK_CLUSTERS,
};

use crate::{fingerprint::Fingerprint, state::FileId};

/// Decides whether `cluster` is a known noise pattern that must not be
/// surfaced as duplication. Returns **which** filter recognised it, and
/// `None` when none did.
///
/// Callers that only need "hide this" use `.is_some()`. The identity is
/// retained for observability; the verbatim escape hatch applies every
/// qualifying exact-byte family independently of the filter.
pub(crate) fn is_noise_pattern<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Option<NoiseFilter> {
    let language = pre_gate(
        uniform_language(members, file_languages),
        NoiseFilter::UniformLanguage,
        members.len(),
        cache,
    )?;
    let snippets = pre_gate(
        collect_snippets(members, sources, language, cache),
        NoiseFilter::CollectSnippets,
        members.len(),
        cache,
    )?;
    run_noise_checks(language, &snippets, sources, file_languages, cache)
}

/// Records a pre-gate that could not even reach the filters, and passes
/// its value through.
fn pre_gate<T>(
    value: Option<T>,
    filter: NoiseFilter,
    members: usize,
    cache: &ParseCache,
) -> Option<T> {
    if value.is_none() {
        cache.record_noise(filter, members, false, std::time::Duration::ZERO);
    }
    value
}

/// Runs every noise filter over `snippets` in short-circuit order,
/// returning the first that fires.
///
/// Generic, language-agnostic checks run for every language (they key
/// off per-language kind maps). The language-specific idiom filters only
/// fire for their own language, so gate them by `language` rather than
/// walking every Dart/C# cluster's CST through Python/Rust matchers that
/// can never match — that wasted walk dominated analysis time on large
/// codegen-heavy repos ([CLONE-NOISE-REPARSE-CACHE]). Each check runs
/// only until one fires, and the counters record only what actually ran
/// ([PERF-FLUTTER-TODO-OBSERVABILITY]).
fn run_noise_checks<S: BuildHasher>(
    language: &str,
    snippets: &[Snippet<'_>],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> Option<NoiseFilter> {
    let polymorphic =
        || polymorphic::is_polymorphic_signature_cluster(snippets, sources, file_languages, cache);
    let signature_only = || is_signature_only_cluster(snippets, cache);
    let literal_calls = || calls::is_literal_variation_call_cluster(snippets, cache);
    let constant_table = || constant_table::is_constant_table_cluster(snippets);
    let data_rows = || data_rows::is_data_row_cluster(snippets);
    let checks: [(NoiseFilter, &dyn Fn() -> bool); 5] = [
        (NoiseFilter::Polymorphic, &polymorphic),
        (NoiseFilter::SignatureOnly, &signature_only),
        (NoiseFilter::LiteralCalls, &literal_calls),
        (NoiseFilter::ConstantTable, &constant_table),
        (NoiseFilter::DataRows, &data_rows),
    ];
    for (filter, check) in checks {
        if let Some(fired) = timed(filter, check, snippets.len(), cache) {
            return Some(fired);
        }
    }
    timed_language_specific(language, snippets, cache)
}

/// Times one generic check, records its counter row, and reports the
/// filter when it fired.
fn timed(
    filter: NoiseFilter,
    check: &dyn Fn() -> bool,
    snippets: usize,
    cache: &ParseCache,
) -> Option<NoiseFilter> {
    let started = std::time::Instant::now();
    let result = check();
    cache.record_noise(filter, snippets, result, started.elapsed());
    result.then_some(filter)
}

/// Times the language-specific bank. Its counter row stays under the one
/// `language_specific` label so the per-run log is unchanged, while the
/// returned identity is the individual filter that fired.
fn timed_language_specific(
    language: &str,
    snippets: &[Snippet<'_>],
    cache: &ParseCache,
) -> Option<NoiseFilter> {
    let started = std::time::Instant::now();
    let fired = language_specific_noise(language, snippets, cache);
    cache.record_noise(
        NoiseFilter::LanguageSpecific,
        snippets.len(),
        fired.is_some(),
        started.elapsed(),
    );
    fired
}

/// One cluster-noise sub-check, for [`ParseCache`]'s aggregate counters
/// ([PERF-FLUTTER-TODO-OBSERVABILITY]): which filter the corpus-scale
/// time actually goes to, and which of them ever fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoiseFilter {
    /// The uniform-language pre-gate.
    UniformLanguage,
    /// The snippet-collection pre-gate.
    CollectSnippets,
    /// The polymorphic-signature contract filter.
    Polymorphic,
    /// The signature-only filter.
    SignatureOnly,
    /// The literal-variation call filter.
    LiteralCalls,
    /// The constant-table filter.
    ConstantTable,
    /// The data-row filter ([CLONE-NOISE-DATA-ROWS]).
    DataRows,
    /// The language-specific idiom filters.
    LanguageSpecific,
    /// The Python sibling-cell filter, named apart from the rest of the
    /// language-specific bank for per-filter observability.
    PyCollectionSiblingCells,
}

impl NoiseFilter {
    /// Stable label for the aggregate record.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::UniformLanguage => "uniform_language",
            Self::CollectSnippets => "collect_snippets",
            Self::Polymorphic => "polymorphic",
            Self::SignatureOnly => "signature_only",
            Self::LiteralCalls => "literal_calls",
            Self::ConstantTable => "constant_table",
            Self::DataRows => "data_rows",
            Self::LanguageSpecific => "language_specific",
            Self::PyCollectionSiblingCells => "py_collection_sibling_cells",
        }
    }

    /// Whether a byte-identical family must span two files before it
    /// escapes this filter ([CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE]).
    ///
    /// Every filter here recognises a family that *may* be spread over
    /// many files, so byte-identity confined to one file is better
    /// explained by the idiom than by a paste — except the sibling-cell
    /// filter, whose members must already share one file and one literal
    /// node. Asking a family that is single-file by construction to span
    /// two files is a question with one answer, and it closed the escape
    /// hatch on that route permanently
    /// ([CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL]). A data-row
    /// window never repeats by accident, so a same-file repeat is a paste too.
    pub(crate) const fn demands_cross_file_copy(self) -> bool {
        !matches!(self, Self::PyCollectionSiblingCells | Self::DataRows)
    }
}

/// Language-specific idiom filters, dispatched by language so a cluster is
/// only walked by matchers that can fire for it. C# has no idiom filter
/// today; Dart suppresses const-data-registry field clusters. Both
/// also rely on the generic checks plus the fusion and report-hide gates.
fn language_specific_noise(
    language: &str,
    snippets: &[Snippet<'_>],
    cache: &ParseCache,
) -> Option<NoiseFilter> {
    let generic = NoiseFilter::LanguageSpecific;
    match language {
        "dart" => (dart::is_dart_class_field_declaration_cluster(snippets, cache)
            || dart::is_dart_widget_scaffold_cluster(snippets))
        .then_some(generic),
        "fsharp" => fsharp_scaffolding::is_import_scaffolding_cluster(snippets).then_some(generic),
        "go" => go_fixture_test::is_fixture_test_cluster(snippets).then_some(generic),
        "php" => php_accessors::is_accessor_scaffold_cluster(snippets).then_some(generic),
        "python" => python_noise(snippets),
        "rust" => rust_noise(snippets).then_some(generic),
        "javascript" | "typescript" | "tsx" => {
            ecmascript::is_ecmascript_data_shape_cluster(snippets).then_some(generic)
        }
        _ => None,
    }
}

/// All Python idiom noise filters (/
/// #112/#114/#115/#121/#126/#133 and monkeypatch scaffolding), in
/// short-circuit order, reporting which one fired.
///
/// The sibling-cell filter is named apart from the rest because its
/// members must already share one file *and* one literal node, so a
/// byte-identical family it recognises can never span two files
/// ([CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL]). Its
/// position in the chain is unchanged: a cluster an earlier filter
/// claims is still credited to that filter.
fn python_noise(snippets: &[Snippet<'_>]) -> Option<NoiseFilter> {
    if python_noise_before_cells(snippets) {
        return Some(NoiseFilter::LanguageSpecific);
    }
    if python_collection_cells::is_collection_sibling_cell_cluster(snippets) {
        return Some(NoiseFilter::PyCollectionSiblingCells);
    }
    python_noise_after_cells(snippets).then_some(NoiseFilter::LanguageSpecific)
}

/// The Python idiom filters that run before the sibling-cell check.
fn python_noise_before_cells(snippets: &[Snippet<'_>]) -> bool {
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
}

/// The Python idiom filters that run after the sibling-cell check.
fn python_noise_after_cells(snippets: &[Snippet<'_>]) -> bool {
    python_class_shapes::is_strenum_class_shape_cluster(snippets)
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

/// Decides whether two exact pair endpoints hold incompatible top-level
/// roles — a class/type definition and a function/method
/// ([CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]). Unresolved roles do not
/// reject the candidate pair.
pub(crate) fn is_embedding_role_mismatch<S: BuildHasher>(
    left: &Fingerprint,
    right: &Fingerprint,
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> bool {
    let members = [left.clone(), right.clone()];
    let Some(language) = uniform_language(&members, file_languages) else {
        return false;
    };
    let Some(snippets) = collect_snippets(&members, sources, language, cache) else {
        return false;
    };
    role_compat::is_role_incompatible_embedding_match(&snippets)
}
