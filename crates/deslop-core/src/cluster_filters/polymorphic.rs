//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE] — the abstract/interface
//! implementation pattern (gh #69). Every cluster member is *about* one
//! function definition whose declared name is the same identifier
//! across at least two files while the bodies differ: the contract
//! forces the signatures to agree, and nothing that differs can share a
//! refactor. Split out of `mod.rs` to keep every file under the 500-LOC
//! budget and to house the widened member resolution.

use std::{collections::HashMap, hash::BuildHasher};

use tree_sitter::Node;

use super::{
    body_shape::{body_kind_stream, ShapeToken},
    contract_index::{declared_bases, enclosing_container, function_name_node},
    enclosing_kind, function_kinds, parse_for, spans_multiple_files, ParseCache, Snippet,
};
use crate::{ast::ByteRange, state::FileId};

/// What one cluster member contributes to the polymorphic decision.
struct Subject<'src> {
    /// The subject function's declared name.
    name: &'src [u8],
    /// Simple names of the bases the subject's enclosing type declares,
    /// empty for a free function.
    bases: Vec<Vec<u8>>,
    /// The subject body's shape, carrying collaborator identity.
    shape: Vec<ShapeToken<'src>>,
}

/// Detects the polymorphic-signature pattern: every cluster member
/// resolves to a function definition ([`polymorphic_subject`]) with one
/// shared declared name, the members span at least two distinct files,
/// the bodies differ ([`body_kind_stream`]), and every subject method is
/// *declared by a contract* its enclosing type derives from
/// ([`ContractIndex::declares`], [CLONE-NOISE-POLYMORPHIC-CONTRACT]).
///
/// The contract requirement is the positive evidence the pattern is
/// named for. Without it every same-named cross-file function was
/// treated as polymorphic on the strength of a body difference, so a
/// copy-pasted helper renamed past the shared collaborators would be
/// deleted from the report the moment it shared its name (gh #373,
/// `polymorphic_gate_hides_rename_clone.rs`). Reading the requirement as
/// "the enclosing type names *some* base" was the same false negative one
/// step further out: two ordinary subclasses of one shared base that
/// happen to copy a method are not implementing anything
/// (`python_inherited_contract_boundary.rs`). The base must declare the
/// method. A free function is never an interface implementation; nothing
/// forces its signature.
///
/// The contract index is corpus-wide and therefore built lazily, after
/// the cheap per-cluster checks have already agreed the cluster looks
/// polymorphic.
pub(super) fn is_polymorphic_signature_cluster<S: BuildHasher>(
    snippets: &[Snippet<'_>],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> bool {
    let subjects: Option<Vec<Subject<'_>>> = snippets.iter().map(subject_of).collect();
    let Some(subjects) = subjects else {
        return false;
    };
    let Some(first) = subjects.first() else {
        return false;
    };
    if !subjects.iter().all(|subject| subject.name == first.name) {
        return false;
    }
    if !spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id)) {
        return false;
    }
    if !subjects.iter().any(|subject| subject.shape != first.shape) {
        return false;
    }
    let Some(language) = snippets.first().map(|snippet| snippet.language) else {
        return false;
    };
    let contracts = cache.contracts(sources, file_languages, language);
    subjects
        .iter()
        .all(|subject| contracts.declares(&subject.bases, subject.name))
}

/// Resolves one member's subject function and everything the decision
/// reads from it, in a single parse.
fn subject_of<'src>(snippet: &Snippet<'src>) -> Option<Subject<'src>> {
    let tree = parse_for(snippet)?;
    let function = polymorphic_subject(tree.root_node(), snippet)?;
    let name_node = function_name_node(function)?;
    Some(Subject {
        name: snippet.source.get(name_node.byte_range())?,
        bases: enclosing_container(function)
            .map(|container| declared_bases(container, snippet.source))
            .unwrap_or_default(),
        shape: body_kind_stream(function.child_by_field_name("body")?, snippet.source),
    })
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
