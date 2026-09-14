//! Dart language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] and [LANG-CAND-DART] using the
//! `tree-sitter-dart` grammar (nielsenko fork, Dart 3.x: records,
//! patterns, class modifiers, extension types, null-aware elements).
//! Normalisation follows the same Type-2-invariance principle as the
//! other plug-ins ([CLONE-TYPE-TAXONOMY]):
//!
//! - `identifier`, `identifier_dollar_escaped`, `type_identifier` →
//!   `"__ident__"` so renamed variables / type names hash identically.
//!   Structural wrappers (`type`, `extension_type_name`, `typed_identifier`,
//!   `type_parameter`) pass through so generic / annotation shape survives.
//! - Every numeric / boolean / `null` / symbol literal and every string
//!   form (single / double / multiline / raw quote variants and their
//!   `template_chars_*` text chunks) → `"__literal__"`. Collapsing the
//!   outer string node makes `'x'` and `"x"` fingerprint identically while
//!   `template_substitution` interpolation expressions stay structural.
//! - `comment`, `block_comment`, `documentation_block_comment` are dropped.
//! - All other named node kinds pass through with their grammar name.
//!
//! Shared walking / interning plumbing lives in [`super::shared`].

use crate::{
    ast::{named_children, NormalizedNode},
    error::CoreError,
    lang::{
        merge_emit::{
            emit_merge_helper, plain_call, BraceStyle, HelperDialect, HelperPlacement,
            InsertionPoint,
        },
        shared::{build_normalised_root, normalise_kind_with, parse_source},
        LanguageParser,
    },
    refactor::{
        emit::{line_indent_at, line_start_at},
        merge::{MergeEmitOutcome, MergeEmitRequest},
        preconditions::{field_text, node_text},
        tables::{
            BindingKind, BoundaryKind, FrameKind, MergeTables, ReferenceTable, ScopeKinds,
            WriteKind,
        },
    },
    state::FileId,
    wire_generated::MergeParameter,
};

/// Stable language identifier reported by [`DartParser::id`].
const LANGUAGE_ID: &str = "dart";

/// Dart implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct DartParser;

impl DartParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for DartParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["dart"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_dart::LANGUAGE.into()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        let tree = parse_source(LANGUAGE_ID, &self.grammar(), source)?;
        build_normalised_root(&tree, file_id, normalise_kind, LANGUAGE_ID)
    }

    fn binding_node_kinds(&self) -> &'static [BindingKind] {
        BINDING_KINDS
    }

    fn identifier_reference_kinds(&self) -> &'static ReferenceTable {
        &REFERENCE_TABLE
    }

    fn extract_scope_kinds(&self) -> Option<&'static ScopeKinds> {
        Some(&SCOPE_KINDS)
    }

    fn merge_tables(&self) -> Option<&'static MergeTables> {
        Some(&MERGE_TABLES)
    }

    fn declared_type_of(
        &self,
        function: tree_sitter::Node<'_>,
        name: &str,
        source: &[u8],
    ) -> Option<String> {
        declared_type_of(function, name, source)
    }

    fn emit_merge_method(&self, request: &MergeEmitRequest<'_, '_>) -> Option<MergeEmitOutcome> {
        emit_merge(request)
    }
}

/// Binding-introducing Dart nodes for [AUTOFIX-EXTRACT-FREE-VARS] /
/// [AUTOFIX-MERGE-SAFETY]: variable definitions, formal parameters,
/// and `for`-in bindings.
const BINDING_KINDS: &[BindingKind] = &[
    BindingKind::new("initialized_variable_definition", Some("name"), &[]),
    BindingKind::new("initialized_identifier", None, &[]),
    BindingKind::new("formal_parameter", Some("name"), &[]),
    BindingKind::new("for_statement", Some("name"), &[]),
];

/// Dart identifier-reference recognition. Member names (`.add`),
/// type positions, and direct call targets (library-scope functions)
/// are not variable references.
const REFERENCE_TABLE: ReferenceTable = ReferenceTable {
    reference_kinds: &["identifier", "identifier_dollar_escaped"],
    bindable_kinds: &[],
    skip_parent_kinds: &["type", "type_identifier", "type_arguments"],
    skip_parent_fields: &[
        ("member_expression", "property"),
        ("call_expression", "function"),
        ("unconditional_assignable_selector", "identifier"),
    ],
    skip_fields: &["type"],
};

/// Nested Dart scopes that open a frame during walks.
const FRAME_KINDS: &[FrameKind] = &[FrameKind::new("function_expression", None, None, &[])];

/// Dart container kinds: statement runs live in blocks, scopes are
/// function or method declarations, shared parents are classes or the
/// library root. Dart has no Tier-1 verbatim emitter yet
/// ([AUTOFIX-EXTRACT-EMITTER] covers C#/Rust/Python), so these tables
/// serve the mechanical merge ([AUTOFIX-MERGE]).
const SCOPE_KINDS: ScopeKinds = ScopeKinds {
    statement_container_kinds: &["block"],
    function_kinds: &["function_declaration", "method_declaration"],
    shared_parent_kinds: &["class_definition", "source_file"],
    frame_kinds: FRAME_KINDS,
    allow_module_top_level: false,
    hoist_rules: &[],
    deferred_frame_kinds: &[],
    scope_escape_kinds: &[],
    // `pattern_assignment` (Dart 3 destructuring) has no fields, so it
    // conservatively matches any named leaf under it — including the
    // right-hand side, an accepted over-refusal.
    write_kinds: &[
        WriteKind::new("assignment_expression", Some("left"), &[], &[]),
        WriteKind::new("postfix_expression", Some("argument"), &["++", "--"], &[]),
        WriteKind::new(
            "unary_expression",
            None,
            &["++", "--"],
            &["unary_expression"],
        ),
        WriteKind::new("pattern_assignment", None, &[], &["pattern_assignment"]),
    ],
    relocation_unsafe_kinds: &[],
};

/// Dart merge tables ([AUTOFIX-MERGE-SAFETY] B and D). Defaults are
/// not emitted in v1 — check F rewrites every site, making them a
/// readability nicety only ([AUTOFIX-MERGE-DEFAULTS]).
const MERGE_TABLES: MergeTables = MergeTables {
    boundary_kinds: &[
        BoundaryKind::new("return_statement", &[]),
        BoundaryKind::new("yield_statement", &[]),
        BoundaryKind::new("await_expression", &[]),
        BoundaryKind::new(
            "break_statement",
            &[
                "for_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
            ],
        ),
        BoundaryKind::new(
            "continue_statement",
            &["for_statement", "while_statement", "do_statement"],
        ),
        BoundaryKind::new("throw_expression", &["try_statement"]),
    ],
    literal_types: &[
        ("decimal_integer_literal", "int"),
        ("hex_integer_literal", "int"),
        ("decimal_floating_point_literal", "double"),
        ("string_literal", "String"),
        ("true", "bool"),
        ("false", "bool"),
    ],
    supports_default_parameters: false,
};

/// One indentation step matching `dart format` (two spaces).
const INDENT_STEP: &str = "  ";

/// Syntactic declared-type lookup ([AUTOFIX-MERGE-SAFETY] D): the
/// explicit type of `name`'s formal parameter inside `function`.
/// `final`/`var` locals carry no explicit type and yield `None`.
fn declared_type_of(function: tree_sitter::Node<'_>, name: &str, source: &[u8]) -> Option<String> {
    let mut stack = vec![function];
    while let Some(node) = stack.pop() {
        if node.kind() == "formal_parameter"
            && field_text(node, "name", source).as_deref() == Some(name)
        {
            return named_children(node)
                .into_iter()
                .find(|child| child.kind() == "type")
                .and_then(|child| node_text(child, source));
        }
        stack.extend(named_children(node));
    }
    None
}

/// Builds the Dart merged-helper emission: a lowerCamel top-level
/// function with real declared types above the first occurrence's
/// function ([AUTOFIX-MERGE-NAMES]).
fn emit_merge(request: &MergeEmitRequest<'_, '_>) -> Option<MergeEmitOutcome> {
    let anchor = request.scopes.first()?.function?.start_byte();
    let placement = HelperPlacement {
        insertion_offset: line_start_at(request.source, anchor),
        indent: line_indent_at(request.source, anchor),
        point: InsertionPoint::LineStart,
    };
    Some(emit_merge_helper(request, &placement, &MERGE_DIALECT))
}

/// How Dart spells a merged helper: a lowerCamel top-level function
/// whose parameters are `Type name`.
const MERGE_DIALECT: HelperDialect = HelperDialect {
    name_prefix: "mergedFromCluster_",
    indent_step: INDENT_STEP,
    brace: BraceStyle::SameLine,
    parameter: merge_parameter_text,
    signature: merge_signature_text,
    call: plain_call,
};

/// Renders one Dart parameter as `Type name`.
fn merge_parameter_text(parameter: &MergeParameter) -> String {
    format!("{} {}", parameter.type_name, parameter.name)
}

/// Renders the Dart helper declaration line.
fn merge_signature_text(helper_name: &str, parameters: &str) -> String {
    format!("void {helper_name}({parameters})")
}

/// Maps a tree-sitter Dart node kind to its normalised form. Returns
/// `None` when the node should be dropped entirely (pure trivia). The
/// returned `&'static str` comes from a fixed placeholder set or is
/// interned on first sight so downstream hashing is cheap and stable.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    normalise_kind_with(raw, is_comment_kind, is_identifier_kind, is_literal_kind)
}

/// Dart trivia.
fn is_comment_kind(raw: &str) -> bool {
    matches!(
        raw,
        "comment" | "block_comment" | "documentation_block_comment"
    )
}

/// Dart identifier leaves, collapsed for Type-2 renamed-clone detection.
fn is_identifier_kind(raw: &str) -> bool {
    matches!(
        raw,
        "identifier" | "identifier_dollar_escaped" | "type_identifier"
    )
}

/// Returns true when `raw` is a Dart literal node collapsed by
/// normalisation — every numeric / boolean / `null` / symbol scalar and
/// every string form (including the `template_chars_*` text chunks).
/// Collapsing the outer string node makes `'x'` and `"x"` fingerprint
/// identically regardless of quote style, escaping, or value, while
/// `template_substitution` interpolation expressions stay structural.
#[must_use]
pub(crate) fn is_literal_kind(raw: &str) -> bool {
    is_scalar_literal_kind(raw) || is_string_literal_kind(raw)
}

/// Returns true for Dart numeric, boolean, `null`, and symbol literal
/// node kinds.
fn is_scalar_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "decimal_integer_literal"
            | "hex_integer_literal"
            | "decimal_floating_point_literal"
            | "true"
            | "false"
            | "null_literal"
            | "symbol_literal"
    )
}

/// Returns true for every Dart string node kind — all quote/raw/multiline
/// variants and their `template_chars_*` text chunks.
fn is_string_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "string_literal"
            | "string_literal_single_quotes"
            | "string_literal_single_quotes_multiple"
            | "string_literal_double_quotes"
            | "string_literal_double_quotes_multiple"
            | "raw_string_literal_single_quotes"
            | "raw_string_literal_single_quotes_multiple"
            | "raw_string_literal_double_quotes"
            | "raw_string_literal_double_quotes_multiple"
            | "template_chars_single"
            | "template_chars_single_single"
            | "template_chars_double"
            | "template_chars_double_single"
            | "template_chars_raw_slash"
    )
}
