use std::collections::BTreeSet;

use tree_sitter::Node;

use super::super::{enclosing_kind, parse_for, trimmed_snippet_range, Snippet};
use crate::ast::{named_children, ByteRange};

/// True when every shape implements `LanguageParser` with the core
/// methods and at least two impl bodies differ, so a verbatim copy of
/// one adapter still surfaces as duplication.
pub(super) fn impl_shapes_form_adapter_family(shapes: &[RustImplShape]) -> bool {
    let Some(first) = shapes.first() else {
        return false;
    };
    let required_methods = language_parser_core_methods();
    shapes.iter().all(|shape| {
        shape.trait_name == b"LanguageParser" && required_methods.is_subset(&shape.methods)
    }) && shapes
        .iter()
        .any(|shape| shape.impl_source != first.impl_source)
}

/// Parsed shape of one Rust `impl Trait for Type` block.
pub(super) struct RustImplShape {
    /// Trait name from the `impl Trait for Type` header.
    trait_name: Vec<u8>,
    /// Method names declared directly inside the impl block.
    methods: BTreeSet<Vec<u8>>,
    /// Raw impl bytes used to avoid suppressing exact copies.
    impl_source: Vec<u8>,
}

/// Returns the `LanguageParser` impl contained in — or enclosing —
/// `snippet.range`. A sibling-window member (issue #339) spans a run
/// of trait methods *inside* the impl's declaration list rather than
/// the impl item itself, so when no impl lies within the range the
/// impl that owns the window carries the contract shape.
pub(super) fn rust_language_parser_impl_shape(snippet: &Snippet<'_>) -> Option<RustImplShape> {
    let tree = parse_for(snippet)?;
    let mut shapes = Vec::new();
    collect_rust_impl_shapes(tree.root_node(), snippet.range, snippet.source, &mut shapes);
    shapes
        .into_iter()
        .find(|shape| shape.trait_name == b"LanguageParser")
        .or_else(|| enclosing_rust_impl_shape(tree.root_node(), snippet))
}

/// Resolves the innermost `impl` block enclosing the member's trimmed
/// range and returns its shape when it is a `LanguageParser` impl.
fn enclosing_rust_impl_shape(root: Node<'_>, snippet: &Snippet<'_>) -> Option<RustImplShape> {
    let range = trimmed_snippet_range(snippet)?;
    let node = enclosing_kind(root, range, &["impl_item"])?;
    rust_impl_shape_from_node(node, snippet.source)
        .filter(|shape| shape.trait_name == b"LanguageParser")
}

/// Recursively collects Rust impl blocks fully contained in `range`.
fn collect_rust_impl_shapes(
    node: Node<'_>,
    range: ByteRange,
    source: &[u8],
    out: &mut Vec<RustImplShape>,
) {
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return;
    }
    if node.kind() == "impl_item"
        && node.start_byte() >= range.start
        && node.end_byte() <= range.end
    {
        if let Some(shape) = rust_impl_shape_from_node(node, source) {
            out.push(shape);
        }
    }
    for child in named_children(node) {
        collect_rust_impl_shapes(child, range, source, out);
    }
}

/// Extracts trait header and direct method names from one Rust impl
/// node. The trait name comes from the `trait` field of the CST — a
/// text scan for `impl … for …` misreads generic parameters, `where`
/// clauses, and paths such as `lang::LanguageParser`.
fn rust_impl_shape_from_node(node: Node<'_>, source: &[u8]) -> Option<RustImplShape> {
    let impl_source = source.get(node.start_byte()..node.end_byte())?;
    let trait_node = node.child_by_field_name("trait")?;
    let trait_name = source.get(trait_node.start_byte()..trait_node.end_byte())?;
    Some(RustImplShape {
        trait_name: trait_name.to_vec(),
        methods: rust_impl_method_names(node, source),
        impl_source: impl_source.to_vec(),
    })
}

/// Returns method names declared directly under a Rust impl block.
fn rust_impl_method_names(node: Node<'_>, source: &[u8]) -> BTreeSet<Vec<u8>> {
    let mut methods = BTreeSet::new();
    collect_rust_function_names(node, source, &mut methods);
    methods
}

/// Walks Rust function items inside an impl block.
fn collect_rust_function_names(node: Node<'_>, source: &[u8], out: &mut BTreeSet<Vec<u8>>) {
    if node.kind() == "function_item" {
        if let Some(name) = node
            .child_by_field_name("name")
            .and_then(|name| source.get(name.start_byte()..name.end_byte()))
        {
            let _inserted = out.insert(name.to_vec());
        }
    }
    for child in named_children(node) {
        collect_rust_function_names(child, source, out);
    }
}

/// Core `LanguageParser` methods every first-party plug-in must
/// override. The trait also carries optional refactoring hooks that
/// each plug-in overrides differently, so the adapter test is a
/// *subset* relation: requiring set equality pinned the filter to one
/// historical trait revision and silently stopped matching the moment
/// the trait grew (issue #391, [CLONE-NOISE-RUST-LANGPARSER]).
fn language_parser_core_methods() -> BTreeSet<Vec<u8>> {
    BTreeSet::from([
        b"id".to_vec(),
        b"file_extensions".to_vec(),
        b"grammar".to_vec(),
        b"parse_and_normalize".to_vec(),
    ])
}
