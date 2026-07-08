//! Python language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for Python using the
//! `tree-sitter-python` grammar. Normalisation rules match the
//! Type-2-invariance principle from the other plug-ins
//! ([CLONE-TYPE-TAXONOMY]): identifiers collapse to `__ident__`,
//! literals (including the `true` / `false` / `none` keywords that
//! tree-sitter exposes as named leaf nodes) collapse to `__literal__`,
//! and `comment` nodes are dropped. Shared walking / interning plumbing
//! lives in [`super::shared`].

use tree_sitter::Node;

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND},
        LanguageParser,
    },
    refactor::{
        emit::{
            cluster_id_prefix, line_indent_at, line_start_at, run_text, EmitOutcome, EmitRequest,
        },
        tables::{BindingKind, FrameKind, HoistRule, ReferenceTable, ScopeKinds},
    },
    state::FileId,
};

/// Stable language identifier reported by [`PythonParser::id`].
const LANGUAGE_ID: &str = "python";

/// Python implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct PythonParser;

impl PythonParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for PythonParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["py"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
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
}

/// Binding-introducing Python nodes for [AUTOFIX-EXTRACT-FREE-VARS].
/// Augmented assignment (`x += 1`) is deliberately absent: its target
/// reads before it writes, so the walk records the outer name as free —
/// matching Python's own local-binding semantics for the extracted
/// scope.
const BINDING_KINDS: &[BindingKind] = &[
    BindingKind {
        node_kind: "assignment",
        name_field: Some("left"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "for_statement",
        name_field: Some("left"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "for_in_clause",
        name_field: Some("left"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "named_expression",
        name_field: Some("name"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "as_pattern",
        name_field: Some("alias"),
        late_fields: &[],
    },
    BindingKind {
        node_kind: "global_statement",
        name_field: None,
        late_fields: &[],
    },
    BindingKind {
        node_kind: "nonlocal_statement",
        name_field: None,
        late_fields: &[],
    },
    BindingKind {
        node_kind: "import_statement",
        name_field: None,
        late_fields: &[],
    },
    BindingKind {
        node_kind: "import_from_statement",
        name_field: None,
        late_fields: &[],
    },
    BindingKind {
        node_kind: "parameters",
        name_field: None,
        late_fields: &[],
    },
    BindingKind {
        node_kind: "lambda_parameters",
        name_field: None,
        late_fields: &[],
    },
];

/// Python identifier-reference recognition for
/// [AUTOFIX-EXTRACT-FREE-VARS]. Attribute names and keyword-argument
/// names are not references. Call targets ARE references: a callable
/// passed through as a parameter behaves identically at the call site,
/// so parameterising it is always behaviour-preserving — Python has no
/// compiler backstop to catch a skipped local callable
/// ([AUTOFIX-EXTRACT-CAVEATS]).
const REFERENCE_TABLE: ReferenceTable = ReferenceTable {
    reference_kinds: &["identifier"],
    bindable_kinds: &[],
    skip_parent_kinds: &["dotted_name"],
    skip_parent_fields: &[("attribute", "attribute"), ("keyword_argument", "name")],
    skip_fields: &["type"],
};

/// Nested Python scopes that open a frame during the free-variable
/// walk: functions, lambdas, classes, and comprehension scopes.
const FRAME_KINDS: &[FrameKind] = &[
    FrameKind {
        node_kind: "function_definition",
        bind_inside_field: None,
        bind_outside_field: Some("name"),
        bind_first_kinds: &[],
    },
    FrameKind {
        node_kind: "lambda",
        bind_inside_field: Some("parameters"),
        bind_outside_field: None,
        bind_first_kinds: &[],
    },
    FrameKind {
        node_kind: "class_definition",
        bind_inside_field: None,
        bind_outside_field: Some("name"),
        bind_first_kinds: &[],
    },
    FrameKind {
        node_kind: "list_comprehension",
        bind_inside_field: None,
        bind_outside_field: None,
        bind_first_kinds: &["for_in_clause"],
    },
    FrameKind {
        node_kind: "set_comprehension",
        bind_inside_field: None,
        bind_outside_field: None,
        bind_first_kinds: &["for_in_clause"],
    },
    FrameKind {
        node_kind: "dictionary_comprehension",
        bind_inside_field: None,
        bind_outside_field: None,
        bind_first_kinds: &["for_in_clause"],
    },
    FrameKind {
        node_kind: "generator_expression",
        bind_inside_field: None,
        bind_outside_field: None,
        bind_first_kinds: &["for_in_clause"],
    },
];

/// Python container kinds for [AUTOFIX-EXTRACT-PRECONDITIONS] rules
/// 4–5: statement runs live in blocks or at module top level (allowed
/// per rule 4), scopes are `def`/`async def` (one grammar kind), and
/// the shared parent is the containing class or the module.
const SCOPE_KINDS: ScopeKinds = ScopeKinds {
    statement_container_kinds: &["block", "module"],
    function_kinds: &["function_definition"],
    shared_parent_kinds: &["class_definition", "module"],
    frame_kinds: FRAME_KINDS,
    allow_module_top_level: true,
    hoist_rules: HOIST_RULES,
    deferred_frame_kinds: &["function_definition", "lambda"],
    scope_escape_kinds: &["global_statement", "nonlocal_statement"],
    // Plain assignment *binds* (rule 6 territory); only augmented
    // assignment reads-then-rebinds an outer name, so it alone is a
    // write of a free variable (rule 7, issue #280).
    write_kinds: &[("augmented_assignment", "left")],
};

/// PEP 572: a walrus target inside a comprehension binds in the
/// containing function or module scope, never the comprehension's own
/// frame ([AUTOFIX-EXTRACT-FREE-VARS] hoisting).
const HOIST_RULES: &[HoistRule] = &[HoistRule {
    binding_kind: "named_expression",
    transparent_frame_kinds: &[
        "list_comprehension",
        "set_comprehension",
        "dictionary_comprehension",
        "generator_expression",
    ],
}];

/// One indentation step for re-indented module-level bodies (PEP 8).
const INDENT_STEP: &str = "    ";

/// Builds the Python extract emission
/// ([AUTOFIX-EXTRACT-EMITTER-PYTHON]): a module-scope `def` immediately
/// above the top-level definition containing the first occurrence, two
/// blank lines around it per PEP 8.
fn emit_extract(request: &EmitRequest<'_, '_>) -> Option<EmitOutcome> {
    let first = request.scopes.first()?;
    let anchor_offset = top_level_anchor_offset(request, first.run.first().copied()?);
    let method_name = format!(
        "extracted_from_cluster_{}",
        cluster_id_prefix(request.cluster_id)
    );
    Some(EmitOutcome {
        insertion_text: function_text(request, &method_name),
        call_text: format!("{method_name}({})", request.free_variables.join(", ")),
        method_name,
        insertion_offset: anchor_offset,
    })
}

/// Line-start offset of the top-level definition (function, class, or
/// decorated definition) containing the first occurrence — the helper
/// is inserted immediately above it ([AUTOFIX-EXTRACT-DESTINATION]).
/// Module-top-level occurrences anchor at their own first statement.
fn top_level_anchor_offset(request: &EmitRequest<'_, '_>, first_node: Node<'_>) -> usize {
    let mut anchor = first_node;
    let mut current = first_node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "module" {
            anchor = current;
            break;
        }
        current = parent;
    }
    line_start_at(request.source, anchor.start_byte())
}

/// Renders the helper `def` text with PEP 8 spacing: the body is the
/// verbatim occurrence slice (its original indentation is a valid
/// consistent block indent), re-indented one step only when the
/// occurrence sat at module top level (indent zero is not a valid body
/// indent).
fn function_text(request: &EmitRequest<'_, '_>, method_name: &str) -> String {
    let parameters = request.free_variables.join(", ");
    let body = request
        .scopes
        .first()
        .map(|scope| indented_body(request.source, scope))
        .unwrap_or_default();
    format!("def {method_name}({parameters}):\n{body}\n\n\n")
}

/// The occurrence body with a valid `def`-relative indentation: the
/// first line gains the run's original indent so every line shares one
/// consistent block indent; zero-indent (module-level) bodies are
/// re-indented by one step instead.
fn indented_body(
    source: &[u8],
    scope: &crate::refactor::preconditions::OccurrenceScope<'_>,
) -> String {
    let text = run_text(source, scope);
    let original_indent = line_indent_at(source, scope.span().start);
    if original_indent.is_empty() {
        return text
            .lines()
            .map(|line| {
                if line.is_empty() {
                    String::new()
                } else {
                    format!("{INDENT_STEP}{line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    format!("{original_indent}{text}")
}

/// Maps a tree-sitter Python node kind to its normalised form. Covers
/// the identifier / literal / trivia families emitted by
/// `tree-sitter-python` 0.25.x.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "comment" => None,
        "identifier" | "type" => Some(IDENTIFIER_KIND),
        "string"
        | "concatenated_string"
        | "integer"
        | "float"
        | "true"
        | "false"
        | "none"
        | "ellipsis" => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}
