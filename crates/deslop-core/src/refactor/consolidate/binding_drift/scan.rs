//! AST scanning for the binding-drift gate: parse artefacts, name
//! collection, and the tree walks the proof engine in [`super`] builds
//! on. Pure queries — every refusal decision stays in the parent
//! module. ([AUTOFIX-CONSOLIDATE-GATE], issue #279)

use std::{collections::BTreeSet, path::PathBuf};

use tree_sitter::{Node, Tree};

use crate::{
    ast::ByteRange,
    lang::LanguageParser,
    refactor::{
        consolidate::DefinitionSite,
        preconditions::{named_children, node_text},
        tables::BindingKind,
    },
};

/// One occurrence file's parse artefacts, shared across the checks.
pub(super) struct ParsedFile<'a> {
    /// Occurrence path as reported.
    pub(super) path: &'a PathBuf,
    /// Parsed raw tree.
    pub(super) tree: Tree,
    /// File bytes.
    pub(super) source: &'a [u8],
}

/// Value/type names and method names referenced inside one definition.
pub(super) struct NameSets {
    /// Free value names plus non-generic type names.
    pub(super) values_and_types: BTreeSet<String>,
    /// Method-call names (`receiver.name(...)`).
    pub(super) methods: BTreeSet<String>,
}

/// Free value names, non-generic type names, and method-position names
/// inside one definition span. Raw collection, deliberately broader
/// than the extract free-var walk: call targets, path roots, and type
/// names all matter for binding stability.
pub(super) fn names_in_definition(
    file: &ParsedFile<'_>,
    span: ByteRange,
    parser: &dyn LanguageParser,
) -> Result<NameSets, String> {
    let Some(definition) = file
        .tree
        .root_node()
        .named_descendant_for_byte_range(span.start, span.end)
    else {
        return Err("definition node unavailable".to_owned());
    };
    let bound = locally_bound(definition, file.source, parser.binding_node_kinds());
    let mut values_and_types = BTreeSet::new();
    let mut methods = kind_texts(definition, file.source, "field_identifier");
    collect_value_identifiers(definition, file.source, &mut values_and_types, &mut methods);
    values_and_types.retain(|name| !bound.contains(name));
    values_and_types.extend(type_names(definition, file.source));
    Ok(NameSets {
        values_and_types,
        methods,
    })
}

/// Splits raw `identifier` leaves: the `name` segment of a scoped path
/// (`Light::next`, `Vec::with_capacity`) resolves through its qualifier
/// and joins the method channel; everything else is a value name.
fn collect_value_identifiers(
    root: Node<'_>,
    source: &[u8],
    values: &mut BTreeSet<String>,
    methods: &mut BTreeSet<String>,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "identifier" {
            if let Some(text) = node_text(node, source) {
                if scoped_name_segment(node) {
                    let _new = methods.insert(text);
                } else {
                    let _new = values.insert(text);
                }
            }
        }
        stack.extend(named_children(node));
    }
}

/// True when `node` is the `name` field of a `scoped_identifier`.
fn scoped_name_segment(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "scoped_identifier"
            && parent
                .child_by_field_name("name")
                .is_some_and(|name| name.id() == node.id())
    })
}

/// Identifiers bound inside the definition — parameters, `let`s, loop
/// and match patterns, closure parameters. In-span references to these
/// bind locally and stay stable wherever the definition moves.
fn locally_bound(
    definition: Node<'_>,
    source: &[u8],
    bindings: &'static [BindingKind],
) -> BTreeSet<String> {
    let mut bound = BTreeSet::new();
    let mut stack = vec![definition];
    while let Some(node) = stack.pop() {
        let binding_field = bindings
            .iter()
            .find(|binding| binding.node_kind == node.kind())
            .and_then(|binding| binding.name_field);
        let closure_params = (node.kind() == "closure_expression").then_some("parameters");
        if let Some(field) = binding_field.or(closure_params) {
            if let Some(target) = node.child_by_field_name(field) {
                bound.extend(kind_texts(target, source, "identifier"));
            }
        }
        stack.extend(named_children(node));
    }
    bound
}

/// Type names referenced in the definition, minus its generic
/// parameters.
fn type_names(definition: Node<'_>, source: &[u8]) -> BTreeSet<String> {
    let mut names = kind_texts(definition, source, "type_identifier");
    if let Some(generics) = definition.child_by_field_name("type_parameters") {
        for generic in kind_texts(generics, source, "type_identifier") {
            let _removed = names.remove(&generic);
        }
    }
    names
}

/// Texts of every descendant of `root` with node kind `kind`.
fn kind_texts(root: Node<'_>, source: &[u8], kind: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == kind {
            if let Some(text) = node_text(node, source) {
                let _new = names.insert(text);
            }
        }
        stack.extend(named_children(node));
    }
    names
}

/// True when any *non-top-level* definition node's `name` field equals
/// `name` — associated items, mod-nested items, enum variants, struct
/// fields. Path expressions and `use` trees also carry `name` fields
/// and are excluded by [`is_definition_kind`].
pub(super) fn nested_definition_exists(file: &ParsedFile<'_>, name: &str) -> bool {
    let root = file.tree.root_node();
    let mut stack: Vec<Node<'_>> = Vec::new();
    for top in named_children(root) {
        stack.extend(named_children(top));
    }
    while let Some(node) = stack.pop() {
        let named = is_definition_kind(node.kind())
            && node
                .child_by_field_name("name")
                .and_then(|child| node_text(child, file.source))
                .as_deref()
                == Some(name);
        if named {
            return true;
        }
        stack.extend(named_children(node));
    }
    false
}

/// Node kinds that *define* the name in their `name` field — items,
/// enum variants, struct fields, macro definitions.
fn is_definition_kind(kind: &str) -> bool {
    kind.ends_with("_item")
        || matches!(
            kind,
            "enum_variant" | "field_declaration" | "macro_definition"
        )
}

/// True when any occurrence file's `impl` (or `trait`) blocks define an
/// associated item named `name`.
pub(super) fn impl_defines(name: &str, files: &[ParsedFile<'_>]) -> bool {
    files.iter().any(|file| {
        named_children(file.tree.root_node())
            .into_iter()
            .filter(|node| matches!(node.kind(), "impl_item" | "trait_item"))
            .any(|block| {
                kind_texts(block, file.source, "identifier").contains(name)
                    || block
                        .child_by_field_name("body")
                        .is_some_and(|body| associated_item_named(body, file.source, name))
            })
    })
}

/// True when `body` has a direct child whose `name` field equals
/// `name`.
fn associated_item_named(body: Node<'_>, source: &[u8], name: &str) -> bool {
    named_children(body).into_iter().any(|item| {
        item.child_by_field_name("name")
            .and_then(|child| node_text(child, source))
            .as_deref()
            == Some(name)
    })
}

/// True when the file has any top-level glob `use …::*;`.
pub(super) fn glob_import_exists(file: &ParsedFile<'_>) -> bool {
    named_children(file.tree.root_node())
        .into_iter()
        .filter(|node| node.kind() == "use_declaration")
        .any(|node| contains_kind(node, "use_wildcard"))
}

/// True when any descendant of `node` has kind `kind`.
fn contains_kind(node: Node<'_>, kind: &str) -> bool {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == kind {
            return true;
        }
        stack.extend(named_children(current));
    }
    false
}

/// Distinct occurrence files, canonical first.
pub(super) fn occurrence_files(groups: &[Vec<DefinitionSite>]) -> Vec<&PathBuf> {
    let mut files = Vec::new();
    for site in groups.iter().flatten() {
        if !files.contains(&&site.path) {
            files.push(&site.path);
        }
    }
    files
}

/// Spans of top-level items named `name` (any item kind with a `name`
/// field — fn, struct, enum, const, trait, mod, …).
pub(super) fn top_level_definitions(file: &ParsedFile<'_>, name: &str) -> Vec<ByteRange> {
    named_children(file.tree.root_node())
        .into_iter()
        .filter(|node| {
            node.child_by_field_name("name")
                .and_then(|child| node_text(child, file.source))
                .as_deref()
                == Some(name)
        })
        .map(|node| ByteRange {
            start: node.start_byte(),
            end: node.end_byte(),
        })
        .collect()
}

/// Texts of top-level `use` declarations whose subtree mentions `name`.
pub(super) fn use_texts(file: &ParsedFile<'_>, name: &str) -> BTreeSet<String> {
    named_children(file.tree.root_node())
        .into_iter()
        .filter(|node| node.kind() == "use_declaration" && mentions(*node, file.source, name))
        .filter_map(|node| node_text(node, file.source))
        .collect()
}

/// True when any leaf descendant's text equals `name`.
fn mentions(node: Node<'_>, source: &[u8], name: &str) -> bool {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.named_child_count() == 0 && node_text(current, source).as_deref() == Some(name) {
            return true;
        }
        stack.extend(named_children(current));
    }
    false
}
