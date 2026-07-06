//! C# language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for C# using the `tree-sitter-c-sharp`
//! grammar. Normalisation rules (mapping to [PIPELINE-NORMALIZE-AST]):
//!
//! - `identifier`, `predefined_type`, `type_parameter` → collapsed to
//!   `"__ident__"` so renamed variables / type names hash identically
//!   (Type-2 invariance per [CLONE-TYPE-TAXONOMY]).
//! - String / verbatim-string / interpolated-string / integer / real /
//!   character / boolean / null literals → collapsed to `"__literal__"`
//!   so changed constants do not perturb the fingerprint.
//! - `comment` nodes are dropped.
//! - All other named node kinds pass through with their grammar name
//!   preserved.
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
        emit::{
            cluster_id_prefix, line_indent_at, run_text, runs_produce_value, EmitOutcome,
            EmitRequest,
        },
        merge::{MergeEmitOutcome, MergeEmitRequest},
        tables::{BindingKind, FrameKind, MergeTables, ReferenceTable, ScopeKinds},
    },
    state::FileId,
};

/// Stable language identifier reported by [`CSharpParser::id`].
const LANGUAGE_ID: &str = "csharp";

/// C# implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct CSharpParser;

impl CSharpParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for CSharpParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["cs"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_c_sharp::LANGUAGE.into()
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

    fn emit_extract_method(&self, request: &EmitRequest<'_, '_>) -> Option<EmitOutcome> {
        emit_extract(request)
    }

    fn merge_tables(&self) -> Option<&'static MergeTables> {
        Some(&super::csharp_merge::MERGE_TABLES)
    }

    fn declared_type_of(
        &self,
        function: tree_sitter::Node<'_>,
        name: &str,
        source: &[u8],
    ) -> Option<String> {
        super::csharp_merge::declared_type_of(function, name, source)
    }

    fn emit_merge_method(&self, request: &MergeEmitRequest<'_, '_>) -> Option<MergeEmitOutcome> {
        super::csharp_merge::emit_merge(request)
    }
}

/// Binding-introducing C# nodes for [AUTOFIX-EXTRACT-FREE-VARS]:
/// declarators, parameters, `foreach`/`catch`/out-var declarations.
const BINDING_KINDS: &[BindingKind] = &[
    BindingKind {
        node_kind: "variable_declarator",
        name_field: Some("name"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "parameter",
        name_field: Some("name"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "foreach_statement",
        name_field: Some("left"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "catch_declaration",
        name_field: Some("name"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "declaration_expression",
        name_field: Some("name"),
        late_fields: &[],
    },
];

/// C# identifier-reference recognition for [AUTOFIX-EXTRACT-FREE-VARS].
/// Member names, type positions, attribute names, and direct invocation
/// targets are not variable references — an invocation target that is a
/// bare identifier resolves as a method of the same class, which the
/// extraction never leaves ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 4).
const REFERENCE_TABLE: ReferenceTable = ReferenceTable {
    reference_kinds: &["identifier"],
    bindable_kinds: &["implicit_parameter"],
    skip_parent_kinds: &[
        "generic_name",
        "type_argument_list",
        "qualified_name",
        "using_directive",
        "namespace_declaration",
        "attribute_list",
    ],
    skip_parent_fields: &[
        ("member_access_expression", "name"),
        ("member_binding_expression", "name"),
        ("invocation_expression", "function"),
        ("attribute", "name"),
    ],
    skip_fields: &["type"],
};

/// Nested C# scopes that open a frame during the free-variable walk.
const FRAME_KINDS: &[FrameKind] = &[
    FrameKind {
        node_kind: "lambda_expression",
        bind_inside_field: Some("parameters"),
        bind_outside_field: None,
        bind_first_kinds: &[],
    },
    FrameKind {
        node_kind: "anonymous_method_expression",
        bind_inside_field: None,
        bind_outside_field: None,
        bind_first_kinds: &[],
    },
    FrameKind {
        node_kind: "local_function_statement",
        bind_inside_field: None,
        bind_outside_field: Some("name"),
        bind_first_kinds: &[],
    },
];

/// C# container kinds for [AUTOFIX-EXTRACT-PRECONDITIONS] rules 4–5:
/// statement runs live in blocks / switch sections, scopes are methods,
/// accessors, or local functions, and the shared parent is the
/// containing type declaration.
const SCOPE_KINDS: ScopeKinds = ScopeKinds {
    statement_container_kinds: &["block", "switch_section"],
    function_kinds: &[
        "method_declaration",
        "accessor_declaration",
        "local_function_statement",
    ],
    shared_parent_kinds: &[
        "class_declaration",
        "struct_declaration",
        "record_declaration",
    ],
    frame_kinds: FRAME_KINDS,
    allow_module_top_level: false,
};

/// C# node kinds that exit with a value when they carry an expression
/// child ([AUTOFIX-EXTRACT-EMITTER-CSHARP] return-type rule).
const VALUE_RETURN_KINDS: &[&str] = &["return_statement", "yield_statement"];

/// One indentation step used for emitted members, matching the
/// dominant C# convention. Deterministic per
/// [AUTOFIX-EXTRACT-EMITTER].
const INDENT_STEP: &str = "    ";

/// Builds the C# extract-method emission ([AUTOFIX-EXTRACT-EMITTER-CSHARP]):
/// a `private static` helper at the top of the enclosing class body plus
/// the single-statement call form.
fn emit_extract(request: &EmitRequest<'_, '_>) -> Option<EmitOutcome> {
    let first = request.scopes.first()?;
    let class_body = first.shared_parent.child_by_field_name("body")?;
    let insertion_offset = class_body.start_byte().checked_add(1)?;
    let class_indent = line_indent_at(request.source, first.shared_parent.start_byte());
    let indent = format!("{class_indent}{INDENT_STEP}");
    let method_name = format!(
        "ExtractedFromCluster_{}",
        cluster_id_prefix(request.cluster_id)
    );
    Some(EmitOutcome {
        insertion_text: method_text(request, &indent, &method_name),
        call_text: format!("{method_name}({});", request.free_variables.join(", ")),
        method_name,
        insertion_offset,
    })
}

/// Renders the helper declaration text: header, brace pair, and the
/// verbatim occurrence body ([AUTOFIX-EXTRACT-EMITTER-CSHARP]).
fn method_text(request: &EmitRequest<'_, '_>, indent: &str, method_name: &str) -> String {
    let statement_indent = format!("{indent}{INDENT_STEP}");
    let signature = method_signature(request, method_name);
    let body = request
        .scopes
        .first()
        .map(|scope| run_text(request.source, scope))
        .unwrap_or_default();
    format!("\n{indent}{signature}\n{indent}{{\n{statement_indent}{body}\n{indent}}}\n")
}

/// Renders the `private static` signature with placeholder types per
/// [AUTOFIX-EXTRACT-EMITTER-CSHARP].
fn method_signature(request: &EmitRequest<'_, '_>, method_name: &str) -> String {
    let frame_kind_names: Vec<&str> = FRAME_KINDS.iter().map(|frame| frame.node_kind).collect();
    let returns_value = runs_produce_value(request.scopes, VALUE_RETURN_KINDS, &frame_kind_names);
    let (return_type, return_todo) = if returns_value {
        ("object", " // TODO: deslop — fix return type")
    } else {
        ("void", "")
    };
    let parameters = request
        .free_variables
        .iter()
        .map(|name| format!("object {name} /* TODO: deslop — fix type */"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("private static {return_type} {method_name}({parameters}){return_todo}")
}

/// Maps a tree-sitter C# node kind to its normalised form. Returns `None`
/// when the node should be dropped entirely (pure trivia). The returned
/// `&'static str` comes from a fixed placeholder set or is interned on
/// first sight so downstream hashing is cheap and stable.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "comment" => None,
        "identifier" | "predefined_type" | "type_parameter" => Some(IDENTIFIER_KIND),
        raw if is_literal_kind(raw) => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}

/// Returns true when `raw` is a C# literal node collapsed by normalisation.
#[must_use]
pub(crate) fn is_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "string_literal"
            | "verbatim_string_literal"
            | "interpolated_string_text"
            | "integer_literal"
            | "real_literal"
            | "character_literal"
            | "boolean_literal"
            | "null_literal"
    )
}
