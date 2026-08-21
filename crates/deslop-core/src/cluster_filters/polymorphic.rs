//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE] — the abstract/interface
//! implementation pattern (gh #69). Every cluster member is *about* one
//! function definition whose declared name is the same identifier
//! across at least two files while the bodies differ: the contract
//! forces the signatures to agree, and nothing that differs can share a
//! refactor. Split out of `mod.rs` to keep every file under the 500-LOC
//! budget and to house the widened member resolution.

use tree_sitter::Node;

use super::{
    body_shape::{body_kind_stream, ShapeToken},
    enclosing_kind, function_kinds, parse_for, spans_multiple_files, Snippet,
};
use crate::ast::ByteRange;

/// Node kinds that declare a type whose members can be forced to a
/// shared signature.
const CONTAINER_KINDS: &[&str] = &[
    "class_definition",
    "class_declaration",
    "struct_declaration",
    "record_declaration",
    "interface_declaration",
    "impl_item",
];

/// Fields of a [`CONTAINER_KINDS`] node naming what it derives from:
/// Python's `superclasses`, Dart's `superclass`/`interfaces`, C#'s
/// `bases`, Rust's `trait`.
const CONTRACT_FIELDS: &[&str] = &[
    "superclasses",
    "superclass",
    "bases",
    "interfaces",
    "trait",
];

/// Named children that carry a declared base where the grammar exposes
/// no field for it.
const CONTRACT_KINDS: &[&str] = &[
    "base_list",
    "class_heritage",
    "superclass",
    "interfaces",
    "mixins",
    "extends_clause",
    "implements_clause",
];

/// What one cluster member contributes to the polymorphic decision.
struct Subject<'src> {
    /// The subject function's declared name.
    name: &'src [u8],
    /// Whether the subject is declared inside a type that names a
    /// contract — the positive evidence that the signature is forced.
    under_contract: bool,
    /// The subject body's shape, carrying collaborator identity.
    shape: Vec<ShapeToken<'src>>,
}

/// Detects the polymorphic-signature pattern: every cluster member
/// resolves to a function definition ([`polymorphic_subject`]) with one
/// shared declared name, every one of them is declared under a contract
/// its signature must match, the members span at least two distinct
/// files, and the bodies differ ([`body_kind_stream`]).
///
/// The contract requirement is the positive evidence the pattern is
/// named for. Without it every same-named cross-file function was
/// treated as polymorphic on the strength of a body difference, so a
/// copy-pasted helper renamed past the shared collaborators would be
/// deleted from the report the moment it shared its name (gh #373,
/// `polymorphic_gate_hides_rename_clone.rs`). A free function is never
/// an interface implementation; nothing forces its signature.
pub(super) fn is_polymorphic_signature_cluster(snippets: &[Snippet<'_>]) -> bool {
    let subjects: Option<Vec<Subject<'_>>> = snippets.iter().map(subject_of).collect();
    let Some(subjects) = subjects else {
        return false;
    };
    let Some(first) = subjects.first() else {
        return false;
    };
    if !subjects
        .iter()
        .all(|subject| subject.name == first.name && subject.under_contract)
    {
        return false;
    }
    if !spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id)) {
        return false;
    }
    subjects.iter().any(|subject| subject.shape != first.shape)
}

/// Resolves one member's subject function and everything the decision
/// reads from it, in a single parse.
fn subject_of<'src>(snippet: &Snippet<'src>) -> Option<Subject<'src>> {
    let tree = parse_for(snippet)?;
    let function = polymorphic_subject(tree.root_node(), snippet)?;
    let name_node = function_name_node(function)?;
    Some(Subject {
        name: snippet.source.get(name_node.byte_range())?,
        under_contract: under_declared_contract(function),
        shape: body_kind_stream(function.child_by_field_name("body")?, snippet.source),
    })
}

/// True when `function` is declared inside a type that names a base,
/// interface or trait — an `ABC` subclass's override, a C# interface
/// implementation, a Rust `impl Trait for T` method. That declaration
/// is what forces the signature the cluster matched on.
fn under_declared_contract(function: Node<'_>) -> bool {
    let mut container = function.parent();
    while let Some(node) = container {
        if CONTAINER_KINDS.contains(&node.kind()) && declares_contract(node) {
            return true;
        }
        container = node.parent();
    }
    false
}

/// True when a type declaration carries a non-empty base list, under
/// whichever field or child kind its grammar exposes it as. An empty
/// `class X():` names no contract.
fn declares_contract(container: Node<'_>) -> bool {
    let field_base = CONTRACT_FIELDS
        .iter()
        .filter_map(|field| container.child_by_field_name(field))
        .any(|base| base.named_child_count() > 0 || base.kind() != "argument_list");
    let mut cursor = container.walk();
    field_base
        || container
            .named_children(&mut cursor)
            .any(|child| CONTRACT_KINDS.contains(&child.kind()))
}

/// The one function a member view is *about*: the innermost function
/// containing the range or, failing that, the sole function the range
/// contains when everything else inside the range is declaration
/// scaffolding. The second direction is how a whole-file view of a
/// single-method class re-enters the pattern — [FUSION-SHARED-SUBTREE]
/// admits module-wide views, and one promoted `docker_host`/`fly_host`
/// to a whole-file near-identical pair on the strength of the bytes the
/// `tool_call` contract forces to agree
/// (`different_backend_implementations_never_pair_across_files`).
fn polymorphic_subject<'tree>(root: Node<'tree>, snippet: &Snippet<'_>) -> Option<Node<'tree>> {
    let kinds = function_kinds(snippet.language);
    if let Some(function) = enclosing_kind(root, snippet.range, kinds) {
        return Some(function);
    }
    let mut functions = Vec::new();
    if !scaffolding_besides_functions(root, snippet.range, kinds, &mut functions) {
        return None;
    }
    match functions.as_slice() {
        [sole] => Some(*sole),
        _ => None,
    }
}

/// Walks the nodes intersecting `range`, collecting contained function
/// definitions and vetting the residue. Returns false the moment any
/// executable residue appears — only import statements, docstrings, and
/// the declaration shell around a class body may surround the subject.
/// The container and scaffolding kind names are Python's, the one
/// language measured for the widened direction; members in other
/// languages keep the containing-function behaviour.
fn scaffolding_besides_functions<'tree>(
    node: Node<'tree>,
    range: ByteRange,
    kinds: &[&str],
    functions: &mut Vec<Node<'tree>>,
) -> bool {
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return true;
    }
    if kinds.contains(&node.kind())
        && node.start_byte() >= range.start
        && node.end_byte() <= range.end
    {
        functions.push(node);
        return true;
    }
    match node.kind() {
        "module" | "class_definition" | "block" | "decorated_definition" => {
            let mut cursor = node.walk();
            let residue_clean = node
                .named_children(&mut cursor)
                .all(|child| scaffolding_besides_functions(child, range, kinds, functions));
            residue_clean
        }
        "expression_statement" => is_docstring(node),
        kind => is_inert_declaration_kind(kind),
    }
}

/// Kinds that carry no behaviour of their own inside a declaration
/// shell: imports, the class name and superclass list, bare strings,
/// and comments.
fn is_inert_declaration_kind(kind: &str) -> bool {
    matches!(
        kind,
        "import_statement"
            | "import_from_statement"
            | "future_import_statement"
            | "identifier"
            | "argument_list"
            | "string"
            | "comment"
    )
}

/// True for an expression statement whose sole named child is a string
/// — a docstring. An executable expression statement disqualifies the
/// scaffolding walk.
fn is_docstring(node: Node<'_>) -> bool {
    node.named_child_count() == 1
        && node
            .named_child(0)
            .is_some_and(|child| child.kind() == "string")
}

/// Resolves the identifier node that names `function`. Python, C#, and
/// Rust expose a direct `name` field on the function node. Dart instead
/// nests it under `signature` — `function_signature.name` for a top-level
/// `function_declaration`, and `method_signature → function_signature.name`
/// for a `method_declaration`. Without this descent
/// [`subject_name`] returns `None` for every Dart member, so
/// the polymorphic-signature filter could never fire on Dart even
/// though `function_kinds` lists its node kinds.
fn function_name_node(function: Node<'_>) -> Option<Node<'_>> {
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
