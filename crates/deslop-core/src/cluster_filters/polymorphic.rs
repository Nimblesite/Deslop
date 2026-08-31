//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE] — the abstract/interface
//! implementation pattern (gh #69). Every cluster member is *about* one
//! function definition whose declared name is the same identifier
//! across at least two files while the bodies differ: the contract
//! forces the signatures to agree, and nothing that differs can share a
//! refactor. Split out of `mod.rs` to keep every file under the 500-LOC
//! budget and to house the widened member resolution.

use std::{collections::HashMap, hash::BuildHasher};

use tree_sitter::Node;

use std::sync::Arc;

use super::{
    body_shape::{body_kind_stream, ShapeToken},
    contract_index::{declared_bases, enclosing_container, function_name_node},
    enclosing_kind, function_kinds,
    override_marker::carries_override_marker,
    parse_for, spans_multiple_files, ParseCache, Snippet,
};
use crate::{
    ast::{named_children, ByteRange},
    state::FileId,
};

/// What one cluster member contributes to the polymorphic decision, in
/// owned form so [`ParseCache`] can memoise it by `(file, range)`
/// beyond the source borrow ([PERF-FLUTTER-TODO-CORPUS]). Field
/// comparison semantics are unchanged from the borrowed original.
pub(crate) struct OwnedSubject {
    /// The subject function's declared name.
    pub(super) name: Vec<u8>,
    /// Simple names of the bases the subject's enclosing type declares,
    /// empty for a free function.
    pub(super) bases: Vec<Vec<u8>>,
    /// blake3 digest of the subject body's shape stream. The decision
    /// only ever asks whether two streams are *unequal*; a digest keeps
    /// that exact for unequal streams (different bodies, different
    /// digests, with overwhelming probability) and can only ever err by
    /// calling two different bodies equal — the direction that keeps a
    /// cluster visible rather than hiding a real one. Storing the digest
    /// instead of the stream shrinks the memoised cell from megabytes
    /// (a giant generated function's full token stream) to ~32 bytes,
    /// which is what keeps the corpus-scale memo resident at all
    /// ([PERF-FLUTTER-TODO-CORPUS]).
    pub(super) shape_digest: [u8; 32],
    /// Whether the language's own override marker qualifies the subject,
    /// which proves a contract declares it even when that contract is
    /// outside the scan ([`carries_override_marker`]).
    pub(super) overrides: bool,
    /// Whether the subject is the abstract declaration of the contract
    /// itself rather than an implementation. The abstract declaration IS
    /// the contract, so a wider view that names it alongside the
    /// implementations qualifies for [CLONE-NOISE-POLYMORPHIC-SIGNATURE];
    /// without this arm the implementations were convicted at method
    /// scope while the whole-file view that also names the abstract base
    /// escaped untouched (`python-issue-69-abstract-method`).
    pub(super) abstract_declaration: bool,
}

/// Detects the polymorphic-signature pattern: every cluster member
/// resolves to a function definition ([`polymorphic_subject`]) with one
/// shared declared name, the members span at least two distinct files,
/// the bodies differ ([`body_kind_stream`]), and every subject method is
/// *declared by a contract* its enclosing type derives from
/// ([`ContractIndex::declares`], [CLONE-NOISE-POLYMORPHIC-CONTRACT]) —
/// or carries the language's own override marker, which proves the same
/// thing for a contract the scan never reached
/// ([`carries_override_marker`]).
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
    // The file-spread gate is pure and needs no subjects, so it runs
    // before the per-member resolution — same verdict, far fewer
    // resolutions ([PERF-FLUTTER-TODO-CORPUS]).
    if !spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id)) {
        return false;
    }
    let subjects: Option<Vec<Arc<OwnedSubject>>> = snippets
        .iter()
        .map(|snippet| cache.subject(snippet, || subject_of(snippet, cache)))
        .collect();
    let Some(subjects) = subjects else {
        return false;
    };
    let Some(first) = subjects.first() else {
        return false;
    };
    if !subjects.iter().all(|subject| subject.name == first.name) {
        return false;
    }
    if !subjects
        .iter()
        .any(|subject| subject.shape_digest != first.shape_digest)
    {
        return false;
    }
    let Some(language) = snippets.first().map(|snippet| snippet.language) else {
        return false;
    };
    let contracts = cache.contracts(sources, file_languages, language);
    subjects.iter().all(|subject| {
        subject.overrides
            || subject.abstract_declaration
            || contracts.declares(&subject.bases, &subject.name)
    })
}

/// Whether the subject is the abstract declaration of the contract it
/// names: a Python function carrying `@abstractmethod`, or a bare
/// Ellipsis body (`...`). The declaration is the contract, so it
/// satisfies the contract requirement on its own ([CLONE-NOISE-POLYMORPHIC-CONTRACT]).
/// Other languages reach the same place through
/// [`ContractIndex::declares`] on an implementation's bases.
fn is_abstract_declaration(function: Node<'_>, language: &str, source: &[u8]) -> bool {
    if language.as_bytes() == b"python" {
        return python_abstract_method(function, source)
            || function.child_by_field_name("body").is_some_and(|body| {
                body.named_child_count() == 1
                    && body
                        .named_child(0)
                        .is_some_and(|child| child.kind() == "ellipsis")
            });
    }
    false
}

/// True when the Python function carries an `abstractmethod` decorator
/// in the decorator block immediately above it — the language's own
/// abstract marker, read off the source lines like
/// [`python_function_has_fixture_decorator`].
fn python_abstract_method(function: Node<'_>, source: &[u8]) -> bool {
    let Some(prefix) = source
        .get(..function.start_byte())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    for line in prefix.trim_end().lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('@') {
            return false;
        }
        if trimmed
            .strip_prefix('@')
            .is_some_and(|callee| callee.contains("abstractmethod"))
        {
            return true;
        }
    }
    false
}

/// blake3 digest of a body-shape stream: `Kind` as two bytes, `Symbol`
/// bytes length-prefixed, `Close` as one byte — a canonical encoding of
/// the comparison the stream itself carried.
fn shape_digest(stream: &[ShapeToken<'_>]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for token in stream {
        match token {
            ShapeToken::Kind(kind) => {
                let bytes = kind.to_le_bytes();
                let _ = hasher.update(&bytes);
            }
            ShapeToken::Symbol(bytes) => {
                let length = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
                let _ = hasher.update(&length.to_le_bytes());
                let _ = hasher.update(bytes);
            }
            ShapeToken::Close => {
                let _ = hasher.update(&[0]);
            }
        }
    }
    *hasher.finalize().as_bytes()
}

/// Resolves one member's subject function and everything the decision
/// reads from it, in a single parse — memoised per `(file, range)` in
/// the cache by the caller ([PERF-FLUTTER-TODO-CORPUS]). The body
/// digest is memoised one level up, by the *function's* range, because
/// sibling members inside one function share the whole-body walk.
fn subject_of(snippet: &Snippet<'_>, cache: &ParseCache) -> Option<OwnedSubject> {
    let tree = parse_for(snippet)?;
    let function = polymorphic_subject(tree.root_node(), snippet)?;
    let name_node = function_name_node(function)?;
    let body = function.child_by_field_name("body")?;
    let digest = cache.body_digest(
        (snippet.file_id, function.start_byte(), function.end_byte()),
        || shape_digest(&body_kind_stream(body, snippet.source)),
    );
    Some(OwnedSubject {
        name: snippet.source.get(name_node.byte_range())?.to_vec(),
        bases: enclosing_container(function)
            .map(|container| declared_bases(container, snippet.source))
            .unwrap_or_default(),
        shape_digest: digest,
        overrides: carries_override_marker(function, snippet.language, snippet.source),
        abstract_declaration: is_abstract_declaration(function, snippet.language, snippet.source),
    })
}

/// The one function a member view is *about*: the innermost function
/// containing the range or, failing that, the sole function the range
/// contains when everything else inside the range is declaration
/// scaffolding. The second direction is how a whole-file view of a
/// single-method class re-enters the pattern — [FUSED-SHARED-SUBTREE]
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
    // A range that cuts into a function without reaching its end is a
    // view into that function, not residue around it: the whole-file
    // views [FUSED-SHARED-SUBTREE] admits can start in the class header
    // and end inside the method body, and the function is still what the
    // view is about (`python-issue-69-abstract-method`). A range cutting
    // through two functions collects both, so the sole-function
    // requirement still refuses it.
    if kinds.contains(&node.kind())
        && node.start_byte() >= range.start
        && node.start_byte() < range.end
    {
        functions.push(node);
        return true;
    }
    match node.kind() {
        "module" | "class_definition" | "block" | "decorated_definition" | "decorator" => {
            named_children(node)
                .into_iter()
                .all(|child| scaffolding_besides_functions(child, range, kinds, functions))
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
