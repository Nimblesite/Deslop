//! [CLONE-NOISE-POLYMORPHIC-CONTRACT] — corpus-wide proof that a named
//! method is *declared by a contract* the subject's enclosing type names.
//!
//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE] may only suppress a same-named
//! cross-file cluster when a contract forces the signature the cluster
//! matched on. Reading "the enclosing type names some base" as that proof
//! makes every ordinary subclass a contract implementation, so two
//! copy-pasted methods in unrelated subclasses of one shared base are
//! deleted from the report the moment the copies rename their
//! collaborators — a false negative. The base must actually declare the
//! method.
//!
//! Establishing that is corpus-wide: the base is normally declared in
//! another file. This index is built at most once per report per
//! language, from the same source map the render already holds, and is
//! keyed by declared type name. Two same-named types in one language
//! merge, which can only add a declared member, so the index answers
//! "some type by this name declares this member".
//!
//! Languages whose contracts are implicit — Go, where methods are
//! declared outside the receiver type and interface satisfaction is
//! never written down — have no lexical enclosing type to resolve, so
//! they fail open: no proof, no suppression, no false negative.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::BuildHasher,
};

use tree_sitter::Node;

use super::{function_kinds, snippets::ParseCache};
use crate::state::FileId;

/// Node kinds that declare a type whose members can be forced to a
/// shared signature.
pub(super) const CONTAINER_KINDS: &[&str] = &[
    "class_definition",
    "class_declaration",
    "struct_declaration",
    "record_declaration",
    "interface_declaration",
    "impl_item",
];

/// Kinds that declare a contract without ever lexically containing an
/// implementation: a Rust `trait` and a Dart `mixin` hold the signatures
/// an implementer must match, but the implementation lives elsewhere.
const DECLARER_ONLY_KINDS: &[&str] = &["trait_item", "mixin_declaration"];

/// Fields of a declarer naming what it derives from: Python's
/// `superclasses`, Dart's `superclass`/`interfaces`, C#'s `bases`,
/// Rust's `trait`.
const CONTRACT_FIELDS: &[&str] = &["superclasses", "superclass", "bases", "interfaces", "trait"];

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

/// Kinds that only group base references; the names sit deeper.
const BASE_WRAPPER_KINDS: &[&str] = &[
    "base_list",
    "class_heritage",
    "extends_clause",
    "implements_clause",
    "argument_list",
    "interfaces",
    "superclass",
    "mixins",
    "type_list",
];

/// Fields leading from a type reference to the identifier naming it: a
/// generic instantiation's `value`/`type`, a qualified path's trailing
/// `attribute`, a keyword argument's `value` (`metaclass=ABCMeta`).
const BASE_NAME_FIELDS: &[&str] = &["value", "attribute", "type", "name"];

/// Member kinds that declare a signature with no body: a Rust trait
/// method, a Dart abstract method, a TypeScript interface member, a Go
/// interface method. Bodied members are covered by [`function_kinds`].
const SIGNATURE_KINDS: &[&str] = &[
    "function_signature_item",
    "method_signature",
    "abstract_method_signature",
    "method_spec",
    "method_elem",
];

/// What one declared type contributes to the index.
#[derive(Default)]
struct TypeFacts {
    /// Simple names of the types this one derives from.
    bases: Vec<Vec<u8>>,
    /// Names of the members this type declares itself.
    members: HashSet<Vec<u8>>,
}

/// Corpus-wide map from declared type name to [`TypeFacts`].
pub(super) struct ContractIndex {
    /// Facts keyed by the declared type's simple name.
    types: HashMap<Vec<u8>, TypeFacts>,
}

impl ContractIndex {
    /// Indexes every file in the report written in `language`. Files in
    /// other languages are skipped: a Python `Sink` and a C# `Sink` are
    /// unrelated contracts, and parsing either with the other's grammar
    /// would poison the shared per-file tree cache.
    pub(super) fn build<S: BuildHasher>(
        sources: &HashMap<FileId, Vec<u8>>,
        file_languages: &HashMap<FileId, &'static str, S>,
        language: &'static str,
        cache: &ParseCache,
    ) -> Self {
        let mut index = Self {
            types: HashMap::new(),
        };
        for (file_id, source) in sources {
            if file_languages.get(file_id) != Some(&language) {
                continue;
            }
            if let Some(tree) = cache.tree_for(*file_id, language, source) {
                index.index_tree(tree.root_node(), source, language);
            }
        }
        index
    }

    /// True when `method` is declared by any transitive base of `bases`.
    /// An unresolved base — declared outside the scanned corpus — simply
    /// contributes no proof.
    pub(super) fn declares<'index>(&'index self, bases: &'index [Vec<u8>], method: &[u8]) -> bool {
        let mut seen: HashSet<&'index [u8]> = HashSet::new();
        let mut queue: VecDeque<&'index [u8]> = bases.iter().map(Vec::as_slice).collect();
        while let Some(name) = queue.pop_front() {
            if !seen.insert(name) {
                continue;
            }
            let Some(facts) = self.types.get(name) else {
                continue;
            };
            if facts.members.contains(method) {
                return true;
            }
            queue.extend(facts.bases.iter().map(Vec::as_slice));
        }
        false
    }

    /// Records every declarer in one file's CST.
    fn index_tree(&mut self, root: Node<'_>, source: &[u8], language: &'static str) {
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if is_declarer(node.kind()) {
                self.record(node, source, language);
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
    }

    /// Merges one declarer's bases and member names into the index.
    fn record(&mut self, declarer: Node<'_>, source: &[u8], language: &'static str) {
        let Some(name) = declared_type_name(declarer, source) else {
            return;
        };
        let facts = self.types.entry(name.to_vec()).or_default();
        facts.bases.extend(declared_bases(declarer, source));
        for member in member_names(declarer, source, language) {
            let _existing = facts.members.insert(member);
        }
    }
}

/// True for a node kind that declares a type or a contract.
fn is_declarer(kind: &str) -> bool {
    CONTAINER_KINDS.contains(&kind) || DECLARER_ONLY_KINDS.contains(&kind)
}

/// The nearest type declaration `function` is declared inside, or `None`
/// for a free function — which nothing forces a signature on.
pub(super) fn enclosing_container(function: Node<'_>) -> Option<Node<'_>> {
    let mut current = function.parent();
    while let Some(node) = current {
        if CONTAINER_KINDS.contains(&node.kind()) {
            return Some(node);
        }
        current = node.parent();
    }
    None
}

/// The simple name a declarer is known by. Rust's `impl_item` names the
/// implementing type under `type` rather than `name`.
fn declared_type_name<'src>(declarer: Node<'_>, source: &'src [u8]) -> Option<&'src [u8]> {
    let named = declarer
        .child_by_field_name("name")
        .or_else(|| declarer.child_by_field_name("type"))?;
    base_name(named, source)
}

/// Resolves the identifier naming a type reference: descends a generic
/// instantiation to its base, a qualified path to its trailing segment,
/// and a keyword argument (`metaclass=ABCMeta`) to its value. Each step
/// moves to a child, so the descent always terminates.
fn base_name<'src>(node: Node<'_>, source: &'src [u8]) -> Option<&'src [u8]> {
    let mut current = node;
    loop {
        let field_child = BASE_NAME_FIELDS
            .iter()
            .find_map(|field| current.child_by_field_name(field));
        match field_child {
            Some(child) => current = child,
            None if current.named_child_count() == 0 => return source.get(current.byte_range()),
            None => current = current.named_child(0)?,
        }
    }
}

/// The simple names of every base a declarer names, across the field and
/// child-kind spellings the grammars use.
pub(super) fn declared_bases(declarer: Node<'_>, source: &[u8]) -> Vec<Vec<u8>> {
    let mut names = Vec::new();
    let mut cursor = declarer.walk();
    let fielded = CONTRACT_FIELDS
        .iter()
        .filter_map(|field| declarer.child_by_field_name(field));
    let childed = declarer
        .named_children(&mut cursor)
        .filter(|child| CONTRACT_KINDS.contains(&child.kind()));
    for reference in fielded.chain(childed) {
        collect_base_names(reference, source, &mut names);
    }
    names
}

/// Appends the base names inside `node`, descending through the wrapper
/// nodes (`base_list`, `class_heritage`, …) that only group references.
fn collect_base_names(node: Node<'_>, source: &[u8], names: &mut Vec<Vec<u8>>) {
    if BASE_WRAPPER_KINDS.contains(&node.kind()) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            collect_base_names(child, source, names);
        }
        return;
    }
    names.extend(base_name(node, source).map(<[u8]>::to_vec));
}

/// The names of every member `declarer` declares itself — bodied
/// functions and body-less signatures alike — without descending into a
/// nested type, whose members belong to that type instead.
fn member_names(declarer: Node<'_>, source: &[u8], language: &'static str) -> Vec<Vec<u8>> {
    let kinds = function_kinds(language);
    let mut names = Vec::new();
    let mut stack = vec![declarer];
    while let Some(node) = stack.pop() {
        if kinds.contains(&node.kind()) || SIGNATURE_KINDS.contains(&node.kind()) {
            names.extend(
                function_name_node(node)
                    .and_then(|name| source.get(name.byte_range()))
                    .map(<[u8]>::to_vec),
            );
            continue;
        }
        if node.id() != declarer.id() && is_declarer(node.kind()) {
            continue;
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    names
}

/// Resolves the identifier node that names `function`. Python, C#, and
/// Rust expose a direct `name` field on the function node. Dart instead
/// nests it under `signature` — `function_signature.name` for a top-level
/// `function_declaration`, and `method_signature → function_signature.name`
/// for a `method_declaration`. Without this descent the polymorphic
/// subject has no name, so [CLONE-NOISE-POLYMORPHIC-SIGNATURE] could
/// never fire on Dart even though [`function_kinds`] lists its node
/// kinds.
pub(super) fn function_name_node(function: Node<'_>) -> Option<Node<'_>> {
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
