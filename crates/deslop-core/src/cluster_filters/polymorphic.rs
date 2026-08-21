//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE] — the abstract/interface
//! implementation pattern (gh #69). Every cluster member is *about* one
//! function definition whose declared name is the same identifier
//! across at least two files while the bodies differ: the contract
//! forces the signatures to agree, and nothing that differs can share a
//! refactor. Split out of `mod.rs` to keep every file under the 500-LOC
//! budget and to house the widened member resolution.

use tree_sitter::Node;

use super::{
    body_shape::body_kind_stream, enclosing_kind, function_kinds, parse_for, spans_multiple_files,
    Snippet,
};
use crate::ast::ByteRange;

/// Detects the polymorphic-signature pattern: every cluster member
/// resolves to a function definition ([`polymorphic_subject`]) with one
/// shared declared name, the members span at least two distinct files,
/// and the bodies differ as normalised trees — so a genuine copy-pasted
/// helper that happens to share a name (e.g. a private `_helper` reused
/// in two modules), byte-identical or consistently renamed, still fires
/// as a cluster (gh #373).
pub(super) fn is_polymorphic_signature_cluster(snippets: &[Snippet<'_>]) -> bool {
    let names: Option<Vec<&[u8]>> = snippets.iter().map(subject_name).collect();
    let Some(names) = names else { return false };
    let Some(first_name) = names.first() else {
        return false;
    };
    if !names.iter().all(|name| name == first_name) {
        return false;
    }
    if !spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id)) {
        return false;
    }
    subject_bodies_differ(snippets)
}

/// Returns true when at least two members' subject-function bodies
/// differ as normalised trees — the shared
/// [`body_kind_stream`] the signature-only filter also compares with —
/// distinguishing polymorphism (different implementations of one
/// signature) from genuinely duplicated helper functions that share a
/// name. Identifier and literal text never enters the stream, so a
/// consistently renamed copy is the *same* implementation, not a
/// different one: deciding this on raw source bytes classified every
/// Type-2 rename as polymorphism and deleted the finding (gh #373,
/// `polymorphic_gate_hides_rename_clone.rs`).
fn subject_bodies_differ(snippets: &[Snippet<'_>]) -> bool {
    let streams: Option<Vec<Vec<i32>>> = snippets
        .iter()
        .map(|snippet| {
            let tree = parse_for(snippet)?;
            let function = polymorphic_subject(tree.root_node(), snippet)?;
            let body = function.child_by_field_name("body")?;
            Some(body_kind_stream(body))
        })
        .collect();
    let Some(streams) = streams else { return false };
    let Some(first) = streams.first() else {
        return false;
    };
    streams.iter().any(|stream| stream != first)
}

/// Returns the declared name of the member's subject function, when one
/// resolves.
fn subject_name<'a>(snippet: &'a Snippet<'_>) -> Option<&'a [u8]> {
    let tree = parse_for(snippet)?;
    let function = polymorphic_subject(tree.root_node(), snippet)?;
    let name_node = function_name_node(function)?;
    snippet
        .source
        .get(name_node.start_byte()..name_node.end_byte())
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
