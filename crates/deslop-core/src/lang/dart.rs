//! Dart language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for Dart using the
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
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND},
        LanguageParser,
    },
    refactor::{
        emit::{cluster_id_prefix, line_indent_at, line_start_at},
        merge::{plain_call_text, MergeEmitOutcome, MergeEmitRequest},
        preconditions::{field_text, named_children, node_text},
        tables::{BindingKind, BoundaryKind, FrameKind, MergeTables, ReferenceTable, ScopeKinds},
    },
    state::FileId,
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
    BindingKind {
        node_kind: "initialized_variable_definition",
        name_field: Some("name"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "initialized_identifier",
        name_field: None,
        late_fields: &[],
    },
    BindingKind {
        node_kind: "formal_parameter",
        name_field: Some("name"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "for_statement",
        name_field: Some("name"),
        late_fields: &[],
    },
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
const FRAME_KINDS: &[FrameKind] = &[FrameKind {
    node_kind: "function_expression",
    bind_inside_field: None,
    bind_outside_field: None,
    bind_first_kinds: &[],
}];

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
    write_kinds: &[("assignment_expression", "left")],
};

/// Dart merge tables ([AUTOFIX-MERGE-SAFETY] B and D). Defaults are
/// not emitted in v1 — check F rewrites every site, making them a
/// readability nicety only ([AUTOFIX-MERGE-DEFAULTS]).
const MERGE_TABLES: MergeTables = MergeTables {
    boundary_kinds: &[
        BoundaryKind {
            node_kind: "return_statement",
            allowed_containers: &[],
        },
        BoundaryKind {
            node_kind: "yield_statement",
            allowed_containers: &[],
        },
        BoundaryKind {
            node_kind: "await_expression",
            allowed_containers: &[],
        },
        BoundaryKind {
            node_kind: "break_statement",
            allowed_containers: &[
                "for_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
            ],
        },
        BoundaryKind {
            node_kind: "continue_statement",
            allowed_containers: &["for_statement", "while_statement", "do_statement"],
        },
        BoundaryKind {
            node_kind: "throw_expression",
            allowed_containers: &["try_statement"],
        },
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
    let first = request.scopes.first()?;
    let function = first.function?;
    let insertion_offset = line_start_at(request.source, function.start_byte());
    let indent = line_indent_at(request.source, function.start_byte());
    let helper_name = format!(
        "mergedFromCluster_{}",
        cluster_id_prefix(request.cluster_id)
    );
    let call_texts = (0..request.scopes.len())
        .map(|site| plain_call_text(request.parameters, &helper_name, site))
        .collect();
    Some(MergeEmitOutcome {
        insertion_text: merge_helper_text(request, &indent, &helper_name),
        insertion_offset,
        helper_name,
        call_texts,
    })
}

/// Renders the merged helper with typed parameters.
fn merge_helper_text(
    request: &MergeEmitRequest<'_, '_>,
    indent: &str,
    helper_name: &str,
) -> String {
    let statement_indent = format!("{indent}{INDENT_STEP}");
    let parameters = request
        .parameters
        .iter()
        .map(|parameter| format!("{} {}", parameter.type_name, parameter.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{indent}void {helper_name}({parameters}) {{
{statement_indent}{}
{indent}}}

",
        request.helper_body
    )
}

/// Maps a tree-sitter Dart node kind to its normalised form. Returns
/// `None` when the node should be dropped entirely (pure trivia). The
/// returned `&'static str` comes from a fixed placeholder set or is
/// interned on first sight so downstream hashing is cheap and stable.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "comment" | "block_comment" | "documentation_block_comment" => None,
        "identifier" | "identifier_dollar_escaped" | "type_identifier" => Some(IDENTIFIER_KIND),
        raw if is_literal_kind(raw) => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
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
