//! Binding-drift gate ([AUTOFIX-CONSOLIDATE-GATE] v1.1, issue #279).
//!
//! Byte-equivalence of the moved definitions is not sufficient: a free
//! name inside a definition may resolve to a module-local item that
//! differs across the duplicate files — the traffic-light shape,
//! identical `run` bodies each calling their own `next`. Consolidating
//! would re-bind such references, violating Schäfer's
//! `lookup(ref)_after == lookup(ref)_before`.
//!
//! The gate **proves stability or refuses** — it never assumes it
//! ([AUTOFIX-ZERO-RISK], hardened after the issue #279 review):
//!
//! - free value names and type names must be top-level items defined
//!   byte-equivalently in every occurrence file (checked
//!   **transitively** through their own free names), bound by
//!   textually identical non-glob `use` declarations, or std-prelude
//!   names;
//! - names matching any *nested* definition (associated items in
//!   `impl` blocks, items inside `mod` blocks, enum variants) refuse —
//!   resolution through containers is not mechanically decidable here;
//! - a glob `use …::*` in any occurrence file makes every otherwise
//!   unproven name refuse;
//! - method-call names refuse when any occurrence file's `impl` blocks
//!   define an associated item of that name (receiver types are not
//!   resolved).

use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

use tree_sitter::{Node, Tree};

use crate::{
    ast::ByteRange,
    lang::{shared::parse_source, LanguageParser},
    refactor::{
        consolidate::DefinitionSite,
        preconditions::{named_children, node_text, raw_slices_equivalent},
        tables::BindingKind,
        RefactorError,
    },
};

/// Rust std-prelude names (types, traits, variants) plus ubiquitous
/// std macros — all resolve identically from any sibling module.
/// Primitive types parse as `primitive_type` nodes and never reach the
/// gate.
const RUST_PRELUDE: &[&str] = &[
    "AsMut", "AsRef", "Box", "Clone", "Copy", "Debug", "Default", "DoubleEndedIterator", "Drop",
    "Eq", "Err", "ExactSizeIterator", "Extend", "Fn", "FnMut", "FnOnce", "From", "FromIterator",
    "Hash", "Into", "IntoIterator", "Iterator", "None", "Ok", "Option", "Ord", "PartialEq",
    "PartialOrd", "Result", "Send", "Sized", "Some", "String", "Sync", "ToOwned", "ToString",
    "TryFrom", "TryInto", "Unpin", "Vec", "assert", "assert_eq", "assert_ne", "cfg", "concat",
    "dbg", "env", "eprint", "eprintln", "file", "format", "include_str", "line", "matches",
    "option_env", "print", "println", "stringify", "vec", "write", "writeln",
];

/// One occurrence file's parse artefacts, shared across the checks.
struct ParsedFile<'a> {
    /// Occurrence path as reported.
    path: &'a PathBuf,
    /// Parsed raw tree.
    tree: Tree,
    /// File bytes.
    source: &'a [u8],
}

/// Value/type names and method names referenced inside one definition.
struct NameSets {
    /// Free value names plus non-generic type names.
    values_and_types: BTreeSet<String>,
    /// Method-call names (`receiver.name(...)`).
    methods: BTreeSet<String>,
}

/// How one referenced name proved stable.
enum Stability {
    /// Proven without further work (use-bound, prelude).
    Proven,
    /// A top-level definition — its own references must recurse.
    TopLevel(ByteRange),
}

/// Runs the gate over every symbol group. The inner `Err` carries the
/// refusal reason naming the drifting or unprovable symbol.
///
/// # Errors
///
/// Returns [`RefactorError::Core`] when an occurrence file fails to
/// parse.
pub(super) fn gate<S: ::std::hash::BuildHasher>(
    groups: &[Vec<DefinitionSite>],
    sources: &HashMap<PathBuf, Vec<u8>, S>,
    parser: &dyn LanguageParser,
) -> Result<Result<(), String>, RefactorError> {
    let mut files = Vec::new();
    for path in occurrence_files(groups) {
        let Some(source) = sources.get(path) else {
            return Ok(Err(format!("no source for {}", path.display())));
        };
        let tree = parse_source(parser.id(), &parser.grammar(), source)?;
        files.push(ParsedFile { path, tree, source });
    }
    let consolidated: BTreeSet<String> = groups
        .iter()
        .filter_map(|group| group.first())
        .map(|site| site.name.clone())
        .collect();
    Ok(check_groups(groups, &files, parser, &consolidated))
}

/// Seeds the worklist from every canonical definition and drains it.
fn check_groups(
    groups: &[Vec<DefinitionSite>],
    files: &[ParsedFile<'_>],
    parser: &dyn LanguageParser,
    consolidated: &BTreeSet<String>,
) -> Result<(), String> {
    let mut pending: Vec<String> = Vec::new();
    let mut methods: BTreeSet<String> = BTreeSet::new();
    for canonical in groups.iter().filter_map(|group| group.first()) {
        let Some(file) = files.iter().find(|file| *file.path == canonical.path) else {
            continue;
        };
        let sets = names_in_definition(file, canonical.item_span, parser)?;
        pending.extend(sets.values_and_types);
        methods.extend(sets.methods);
    }
    drain_worklist(pending, &mut methods, files, parser, consolidated)?;
    for method in &methods {
        if impl_defines(method, files) {
            return Err(format!(
                "`{method}` may resolve to an impl-defined method the move would re-bind (v1 gate, issue #279)"
            ));
        }
    }
    Ok(())
}

/// Proves every pending name stable, recursing through top-level
/// definitions with a visited set.
fn drain_worklist(
    mut pending: Vec<String>,
    methods: &mut BTreeSet<String>,
    files: &[ParsedFile<'_>],
    parser: &dyn LanguageParser,
    consolidated: &BTreeSet<String>,
) -> Result<(), String> {
    let mut visited: BTreeSet<String> = BTreeSet::new();
    pending.sort_unstable_by(|left, right| right.cmp(left));
    while let Some(name) = pending.pop() {
        if consolidated.contains(&name) || !visited.insert(name.clone()) {
            continue;
        }
        if let Stability::TopLevel(span) = prove_stable(&name, files)? {
            let Some(canonical) = files.first() else {
                continue;
            };
            let sets = names_in_definition(canonical, span, parser)?;
            pending.extend(sets.values_and_types);
            methods.extend(sets.methods);
        }
    }
    Ok(())
}

/// Proves one name stable or refuses with the reason.
fn prove_stable(name: &str, files: &[ParsedFile<'_>]) -> Result<Stability, String> {
    if files.iter().any(|file| nested_definition_exists(file, name)) {
        return Err(format!(
            "`{name}` matches a definition nested inside another item (impl/mod/enum) — resolution is not mechanically decidable (v1 gate, issue #279)"
        ));
    }
    let definitions: Vec<Vec<ByteRange>> = files
        .iter()
        .map(|file| top_level_definitions(file, name))
        .collect();
    if definitions.iter().any(|spans| !spans.is_empty()) {
        return definitions_equivalent(name, files, &definitions);
    }
    if files.iter().any(|file| !use_texts(file, name).is_empty()) {
        return use_declarations_identical(name, files);
    }
    if files.iter().any(glob_import_exists) {
        return Err(format!(
            "`{name}` may be bound by a glob `use …::*` — not mechanically decidable (v1 gate, issue #279)"
        ));
    }
    if RUST_PRELUDE.contains(&name) {
        return Ok(Stability::Proven);
    }
    Err(format!(
        "`{name}` cannot be proven binding-stable across the duplicate files (v1 gate, issue #279)"
    ))
}

/// Free value names, non-generic type names, and method-position names
/// inside one definition span. Raw collection, deliberately broader
/// than the extract free-var walk: call targets, path roots, and type
/// names all matter for binding stability.
fn names_in_definition(
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
        let closure_params =
            (node.kind() == "closure_expression").then_some("parameters");
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
fn nested_definition_exists(file: &ParsedFile<'_>, name: &str) -> bool {
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
        || matches!(kind, "enum_variant" | "field_declaration" | "macro_definition")
}

/// True when any occurrence file's `impl` (or `trait`) blocks define an
/// associated item named `name`.
fn impl_defines(name: &str, files: &[ParsedFile<'_>]) -> bool {
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
fn glob_import_exists(file: &ParsedFile<'_>) -> bool {
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
fn occurrence_files(groups: &[Vec<DefinitionSite>]) -> Vec<&PathBuf> {
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
fn top_level_definitions(file: &ParsedFile<'_>, name: &str) -> Vec<ByteRange> {
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

/// Module-local definitions of `name` must be exactly one per file and
/// byte-equivalent across all of them; the canonical span recurses.
fn definitions_equivalent(
    name: &str,
    files: &[ParsedFile<'_>],
    definitions: &[Vec<ByteRange>],
) -> Result<Stability, String> {
    if definitions.iter().any(|spans| spans.len() != 1) {
        return Err(format!(
            "`{name}` is not defined exactly once in every duplicate file — the moved reference would re-bind (issue #279)"
        ));
    }
    let slices: Option<Vec<&[u8]>> = files
        .iter()
        .zip(definitions)
        .map(|(file, spans)| {
            spans
                .first()
                .and_then(|span| file.source.get(span.start..span.end))
        })
        .collect();
    if !slices.is_some_and(|slices| raw_slices_equivalent(&slices)) {
        return Err(format!(
            "`{name}` is defined differently across the duplicate files — the moved reference would re-bind (issue #279)"
        ));
    }
    let span = definitions
        .first()
        .and_then(|spans| spans.first().copied())
        .ok_or_else(|| format!("`{name}` has no canonical definition span"))?;
    Ok(Stability::TopLevel(span))
}

/// `use` declarations mentioning `name` must be textually identical
/// across every occurrence file.
fn use_declarations_identical(
    name: &str,
    files: &[ParsedFile<'_>],
) -> Result<Stability, String> {
    let per_file: Vec<BTreeSet<String>> =
        files.iter().map(|file| use_texts(file, name)).collect();
    let all_equal = per_file
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left == right));
    if all_equal {
        Ok(Stability::Proven)
    } else {
        Err(format!(
            "`use` declarations binding `{name}` differ across the duplicate files (issue #279)"
        ))
    }
}

/// Texts of top-level `use` declarations whose subtree mentions `name`.
fn use_texts(file: &ParsedFile<'_>, name: &str) -> BTreeSet<String> {
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
        if current.named_child_count() == 0 && node_text(current, source).as_deref() == Some(name)
        {
            return true;
        }
        stack.extend(named_children(current));
    }
    false
}
