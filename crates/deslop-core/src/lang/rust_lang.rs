//! Rust language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for Rust using the
//! `tree-sitter-rust` grammar. Normalisation rules follow the same
//! Type-2-invariance principle as the C# plug-in ([CLONE-TYPE-TAXONOMY]):
//! every identifier flavour collapses to `__ident__`, every literal
//! flavour collapses to `__literal__`, line and block comments are
//! dropped. Shared walking / interning plumbing lives in
//! [`super::shared`].

use tree_sitter::Node;

use crate::{
    ast::{named_children, NormalizedNode},
    error::CoreError,
    lang::{
        shared::{build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND},
        LanguageParser,
    },
    refactor::{
        emit::{
            cluster_id_prefix, line_indent_at, line_start_at, run_text, EmitOutcome, EmitRequest,
        },
        merge::{plain_call_text, MergeEmitOutcome, MergeEmitRequest},
        preconditions::node_text,
        tables::{
            BindingKind, BoundaryKind, FrameKind, MergeTables, ReferenceTable, ScopeKinds,
            WriteKind,
        },
    },
    state::FileId,
};

/// Stable language identifier reported by [`RustParser::id`].
const LANGUAGE_ID: &str = "rust";

/// Rust implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct RustParser;

impl RustParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for RustParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
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

/// Rust merge tables ([AUTOFIX-MERGE-SAFETY] B and D). Rust has no
/// default parameter values ([AUTOFIX-MERGE-DEFAULTS]).
const MERGE_TABLES: MergeTables = MergeTables {
    boundary_kinds: &[
        BoundaryKind {
            node_kind: "return_expression",
            allowed_containers: &[],
        },
        BoundaryKind {
            node_kind: "try_expression",
            allowed_containers: &[],
        },
        BoundaryKind {
            node_kind: "await_expression",
            allowed_containers: &[],
        },
        BoundaryKind {
            node_kind: "yield_expression",
            allowed_containers: &[],
        },
        BoundaryKind {
            node_kind: "break_expression",
            allowed_containers: &["for_expression", "while_expression", "loop_expression"],
        },
        BoundaryKind {
            node_kind: "continue_expression",
            allowed_containers: &["for_expression", "while_expression", "loop_expression"],
        },
    ],
    literal_types: &[
        ("integer_literal", "i64"),
        ("float_literal", "f64"),
        ("string_literal", "&'static str"),
        ("boolean_literal", "bool"),
        ("char_literal", "char"),
    ],
    supports_default_parameters: false,
};

/// Syntactic declared-type lookup ([AUTOFIX-MERGE-SAFETY] D): the
/// explicit type text of `name`'s parameter or `let` declaration
/// inside `function`. Inferred `let`s yield `None` (which refuses).
fn declared_type_of(function: tree_sitter::Node<'_>, name: &str, source: &[u8]) -> Option<String> {
    let mut stack = vec![function];
    while let Some(node) = stack.pop() {
        let is_typed_binding = matches!(node.kind(), "parameter" | "let_declaration");
        if is_typed_binding && pattern_text(node, source).as_deref() == Some(name) {
            return node
                .child_by_field_name("type")
                .and_then(|child| node_text(child, source));
        }
        stack.extend(named_children(node));
    }
    None
}

/// Text of a binding's `pattern` field when it is a plain identifier.
fn pattern_text(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let pattern = node.child_by_field_name("pattern")?;
    (pattern.kind() == "identifier").then(|| node_text(pattern, source))?
}

/// Builds the Rust merged-helper emission: a `snake_case` free function
/// with real declared types at module scope above the first
/// occurrence's function ([AUTOFIX-MERGE-NAMES]). Runs that end in a
/// tail expression are refused — the call would change the produced
/// value.
fn emit_merge(request: &MergeEmitRequest<'_, '_>) -> Option<MergeEmitOutcome> {
    let first = request.scopes.first()?;
    if !run_ends_with_semicolon(request.source, first.span().end) {
        return None;
    }
    let anchor = attribute_chain_start(first.function?);
    let insertion_offset = line_start_at(request.source, anchor.start_byte());
    let indent = line_indent_at(request.source, anchor.start_byte());
    let helper_name = format!(
        "merged_from_cluster_{}",
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
        .map(|parameter| format!("{}: {}", parameter.name, parameter.type_name))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{indent}fn {helper_name}({parameters}) {{
{statement_indent}{}
{indent}}}

",
        request.helper_body
    )
}

/// Binding-introducing Rust nodes for [AUTOFIX-EXTRACT-FREE-VARS]:
/// `let` patterns, parameters, loop/`if let`/match patterns, and
/// block-local `const` items. A match arm's `value` runs with its
/// pattern already in scope, so it walks late.
const BINDING_KINDS: &[BindingKind] = &[
    BindingKind {
        node_kind: "let_declaration",
        name_field: Some("pattern"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "parameter",
        name_field: Some("pattern"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "for_expression",
        name_field: Some("pattern"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "let_condition",
        name_field: Some("pattern"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "match_arm",
        name_field: Some("pattern"),
        late_fields: &["value"],
    },
    BindingKind {
        node_kind: "const_item",
        name_field: Some("name"),
        late_fields: &[],
    },
];

/// Rust identifier-reference recognition for
/// [AUTOFIX-EXTRACT-FREE-VARS]. Paths (`limits::MAX`, `String::new`),
/// `use` items, macro names, and direct call targets are not variable
/// references — module-scope items resolve identically from the
/// helper's module-scope destination ([AUTOFIX-EXTRACT-DESTINATION]).
/// `field_identifier` / `type_identifier` are distinct kinds and never
/// enter the walk.
const REFERENCE_TABLE: ReferenceTable = ReferenceTable {
    reference_kinds: &["identifier"],
    bindable_kinds: &[],
    skip_parent_kinds: &[
        "scoped_identifier",
        "use_declaration",
        "scoped_use_list",
        "use_list",
        "use_as_clause",
    ],
    skip_parent_fields: &[
        ("call_expression", "function"),
        ("generic_function", "function"),
        ("macro_invocation", "macro"),
    ],
    skip_fields: &["type", "return_type"],
};

/// Nested Rust scopes that open a frame during the free-variable walk.
const FRAME_KINDS: &[FrameKind] = &[
    FrameKind {
        node_kind: "closure_expression",
        bind_inside_field: Some("parameters"),
        bind_outside_field: None,
        bind_first_kinds: &[],
    },
    FrameKind {
        node_kind: "function_item",
        bind_inside_field: None,
        bind_outside_field: Some("name"),
        bind_first_kinds: &[],
    },
];

/// Rust container kinds for [AUTOFIX-EXTRACT-PRECONDITIONS] rules 4–5:
/// statement runs live in blocks, the enclosing scope is a free or
/// `impl` `fn` (same `function_item` kind), and the shared parent is
/// the `impl` block, module, or crate root.
const SCOPE_KINDS: ScopeKinds = ScopeKinds {
    statement_container_kinds: &["block"],
    function_kinds: &["function_item"],
    shared_parent_kinds: &["impl_item", "mod_item", "source_file"],
    frame_kinds: FRAME_KINDS,
    allow_module_top_level: false,
    hoist_rules: &[],
    deferred_frame_kinds: &[],
    scope_escape_kinds: &[],
    // Non-identifier targets (`*p`, `s.f`, `v[i]`) mutate shared
    // state a parameter still reaches; the borrow checker backstops
    // the rest, so no marker or destructuring entries are needed.
    write_kinds: &[
        WriteKind {
            node_kind: "assignment_expression",
            target_field: Some("left"),
            marker_tokens: &[],
            destructuring_kinds: &[],
        },
        WriteKind {
            node_kind: "compound_assignment_expr",
            target_field: Some("left"),
            marker_tokens: &[],
            destructuring_kinds: &[],
        },
    ],
    relocation_unsafe_kinds: &[],
};

/// One indentation step for the emitted helper body, matching rustfmt.
const INDENT_STEP: &str = "    ";

/// Builds the Rust extract emission ([AUTOFIX-EXTRACT-EMITTER-RUST]):
/// the `DeslopTodo` alias plus a free function at module scope,
/// immediately above the function containing the first occurrence
/// (and above its attributes, so `#[...]` chains stay attached).
fn emit_extract(request: &EmitRequest<'_, '_>) -> Option<EmitOutcome> {
    let first = request.scopes.first()?;
    let anchor = attribute_chain_start(first.function?);
    let insertion_offset = line_start_at(request.source, anchor.start_byte());
    let indent = line_indent_at(request.source, anchor.start_byte());
    let method_name = format!(
        "extracted_from_cluster_{}",
        cluster_id_prefix(request.cluster_id)
    );
    let call_suffix = if run_ends_with_semicolon(request.source, first.span().end) {
        ";"
    } else {
        ""
    };
    Some(EmitOutcome {
        insertion_text: function_text(request, &indent, &method_name),
        call_text: format!(
            "{method_name}({}){call_suffix}",
            request.free_variables.join(", ")
        ),
        method_name,
        insertion_offset,
    })
}

/// Walks back over the contiguous `attribute_item` siblings above
/// `function` so the helper is inserted above the whole attributed
/// declaration.
fn attribute_chain_start(function: Node<'_>) -> Node<'_> {
    let mut anchor = function;
    while let Some(previous) = anchor.prev_named_sibling() {
        if previous.kind() != "attribute_item" {
            break;
        }
        anchor = previous;
    }
    anchor
}

/// True when the occurrence's final statement carries a trailing
/// semicolon — the call site keeps it ([AUTOFIX-EXTRACT-EMITTER-RUST]).
fn run_ends_with_semicolon(source: &[u8], span_end: usize) -> bool {
    span_end
        .checked_sub(1)
        .and_then(|index| source.get(index))
        .copied()
        == Some(b';')
}

/// Renders the `DeslopTodo` alias plus the helper function text per
/// [AUTOFIX-EXTRACT-EMITTER-RUST].
fn function_text(request: &EmitRequest<'_, '_>, indent: &str, method_name: &str) -> String {
    let statement_indent = format!("{indent}{INDENT_STEP}");
    let parameters = request
        .free_variables
        .iter()
        .map(|name| format!("{name}: DeslopTodo"))
        .collect::<Vec<_>>()
        .join(", ");
    let body = request
        .scopes
        .first()
        .map(|scope| run_text(request.source, scope))
        .unwrap_or_default();
    format!(
        "{indent}// TODO: deslop — replace `DeslopTodo` with real types.\n\
         {indent}type DeslopTodo = ();\n\
         \n\
         {indent}fn {method_name}({parameters}) -> DeslopTodo {{\n\
         {statement_indent}{body}\n\
         {indent}}}\n\
         \n"
    )
}

/// Maps a tree-sitter Rust node kind to its normalised form. Covers the
/// identifier / literal / trivia families emitted by `tree-sitter-rust`
/// 0.24.x. Every other named kind passes through interned so the hash
/// stays stable across runs.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "line_comment" | "block_comment" => None,
        "identifier"
        | "type_identifier"
        | "field_identifier"
        | "shorthand_field_identifier"
        | "primitive_type"
        | "scoped_identifier"
        | "scoped_type_identifier"
        | "metavariable" => Some(IDENTIFIER_KIND),
        "string_literal" | "raw_string_literal" | "char_literal" | "integer_literal"
        | "float_literal" | "boolean_literal" => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}
