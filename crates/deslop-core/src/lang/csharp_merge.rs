//! C# mechanical-merge support ([AUTOFIX-MERGE]): safety tables, the
//! syntactic declared-type lookup, and the typed helper emitter. Split
//! out of `csharp.rs` to respect the 500-line file budget.

use tree_sitter::Node;

use crate::ast::named_children;
use crate::lang::merge_emit::{emit_merge_helper, HelperDialect, HelperPlacement};
use crate::refactor::{
    emit::line_indent_at,
    merge::{site_arguments, MergeEmitOutcome, MergeEmitRequest},
    preconditions::{field_text, node_text},
    tables::{BoundaryKind, MergeTables},
};
use crate::wire_generated::MergeParameter;

/// C# merge tables ([AUTOFIX-MERGE-SAFETY] B and D,
/// [AUTOFIX-MERGE-DEFAULTS]).
pub(super) const MERGE_TABLES: MergeTables = MergeTables {
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
            node_kind: "goto_statement",
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
                "foreach_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
            ],
        },
        BoundaryKind {
            node_kind: "continue_statement",
            allowed_containers: &[
                "for_statement",
                "foreach_statement",
                "while_statement",
                "do_statement",
            ],
        },
        BoundaryKind {
            node_kind: "throw_statement",
            allowed_containers: &["try_statement"],
        },
        BoundaryKind {
            node_kind: "throw_expression",
            allowed_containers: &["try_statement"],
        },
    ],
    literal_types: &[
        ("integer_literal", "int"),
        ("real_literal", "double"),
        ("string_literal", "string"),
        ("verbatim_string_literal", "string"),
        ("boolean_literal", "bool"),
        ("character_literal", "char"),
    ],
    supports_default_parameters: true,
};

/// One indentation step for emitted members.
const INDENT_STEP: &str = "    ";

/// Syntactic declared-type lookup ([AUTOFIX-MERGE-SAFETY] D): the
/// explicit type text of `name`'s parameter or local declaration
/// inside `function`. `var` declarations carry no explicit type and
/// yield `None` (which refuses the merge).
pub(super) fn declared_type_of(function: Node<'_>, name: &str, source: &[u8]) -> Option<String> {
    let mut stack = vec![function];
    while let Some(node) = stack.pop() {
        if node.kind() == "parameter" && field_text(node, "name", source).as_deref() == Some(name) {
            return field_text(node, "type", source);
        }
        if node.kind() == "variable_declaration" {
            if let Some(found) = declarator_type(node, name, source) {
                return Some(found);
            }
        }
        stack.extend(named_children(node));
    }
    None
}

/// The declared type of `name` when this `variable_declaration` binds
/// it with an explicit (non-`var`) type.
fn declarator_type(declaration: Node<'_>, name: &str, source: &[u8]) -> Option<String> {
    let type_node = declaration.child_by_field_name("type")?;
    if type_node.kind() == "implicit_type" {
        return None;
    }
    named_children(declaration)
        .into_iter()
        .filter(|child| child.kind() == "variable_declarator")
        .any(|declarator| field_text(declarator, "name", source).as_deref() == Some(name))
        .then(|| node_text(type_node, source))?
}

/// Builds the C# merged-helper emission: a `private static void`
/// helper with real declared types at the top of the enclosing class,
/// plus per-site calls with trailing defaults elided
/// ([AUTOFIX-MERGE-NAMES], [AUTOFIX-MERGE-DEFAULTS]).
pub(super) fn emit_merge(request: &MergeEmitRequest<'_, '_>) -> Option<MergeEmitOutcome> {
    let first = request.scopes.first()?;
    let class_body = first.shared_parent.child_by_field_name("body")?;
    let class_indent = line_indent_at(request.source, first.shared_parent.start_byte());
    let placement = HelperPlacement {
        insertion_offset: class_body.start_byte().checked_add(1)?,
        indent: format!("{class_indent}{INDENT_STEP}"),
    };
    Some(emit_merge_helper(request, &placement, &MERGE_DIALECT))
}

/// How C# spells a merged helper: a `private static void` method whose
/// parameters are `Type name`, optionally carrying a default.
const MERGE_DIALECT: HelperDialect = HelperDialect {
    name_prefix: "MergedFromCluster_",
    indent_step: INDENT_STEP,
    parameter: merge_parameter_text,
    signature: merge_signature_text,
    call: call_text,
};

/// Renders one C# parameter as `Type name`, with ` = default` when the
/// parameter carries one ([AUTOFIX-MERGE-DEFAULTS]).
fn merge_parameter_text(parameter: &MergeParameter) -> String {
    match &parameter.default_value {
        Some(default) => format!("{} {} = {default}", parameter.type_name, parameter.name),
        None => format!("{} {}", parameter.type_name, parameter.name),
    }
}

/// Renders the C# helper declaration line.
fn merge_signature_text(helper_name: &str, parameters: &str) -> String {
    format!("private static void {helper_name}({parameters})")
}

/// One site's call, eliding trailing arguments equal to their default.
fn call_text(request: &MergeEmitRequest<'_, '_>, helper_name: &str, site: usize) -> String {
    let mut arguments = site_arguments(request.parameters, site);
    for parameter in request.parameters.iter().rev() {
        let elidable = parameter
            .default_value
            .as_deref()
            .zip(arguments.last().copied())
            .is_some_and(|(default, last)| default == last);
        if !elidable {
            break;
        }
        let _elided = arguments.pop();
    }
    format!("{helper_name}({});", arguments.join(", "))
}
