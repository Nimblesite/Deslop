//! Rust-specific cluster filters.
//!
//! Suppresses false-positive clusters whose shape is fixed by the Rust
//! language itself rather than by the program under analysis.
//!
//! Issues addressed (see parent `mod.rs` header):
//! - [CLONE-NOISE-RUST-LANGPARSER] — every first-party Rust
//!   language plug-in implements the same `LanguageParser` trait surface.
//! - [CLONE-NOISE-RUST-DECL] — every member is a single `mod
//!   NAME;` or `use ...;` statement.
//! - [CLONE-NOISE-RUST-ITER-COLLECT] — every member is the
//!   `<expr>.iter().map(|x| x.<field>.<method>(...)).collect()` chain.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{
    enclosing_kind, is_multi_member_language_cluster, language_cluster_shapes,
    node_intersects_range, node_search::KindSearch, parse_for, raw_snippet_texts_differ,
    spans_multiple_files, trimmed_snippet_range, Snippet,
};
use crate::ast::{named_children, ByteRange};

/// Detects [CLONE-NOISE-RUST-LANGPARSER]: the Rust source files that
/// implement the first-party language plug-ins all carry the same
/// `LanguageParser` adapter surface. Each implementation has
/// language-specific constants and grammar functions, but the trait
/// contract forces the same method outline, so the cluster is not
/// actionable duplication.
pub(super) fn is_rust_language_parser_adapter_cluster(snippets: &[Snippet<'_>]) -> bool {
    spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id))
        && language_cluster_shapes(snippets, "rust", rust_language_parser_impl_shape)
            .is_some_and(|shapes| impl_shapes_form_adapter_family(&shapes))
}

/// True when every shape implements `LanguageParser` with the core
/// methods and at least two impl bodies differ, so a verbatim copy of
/// one adapter still surfaces as duplication.
fn impl_shapes_form_adapter_family(shapes: &[RustImplShape]) -> bool {
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
struct RustImplShape {
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
fn rust_language_parser_impl_shape(snippet: &Snippet<'_>) -> Option<RustImplShape> {
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

/// Detects ****: clusters whose every member is a single Rust
/// top-level declaration that has no body (`mod NAME;`, `use ...;`,
/// `pub use ...;`). Module declarations cannot be macro-generated in
/// Rust, so the cluster is not actionable.
pub(super) fn is_rust_top_level_decl_cluster(snippets: &[Snippet<'_>]) -> bool {
    language_cluster_shapes(snippets, "rust", decl_signature)
        .is_some_and(|signatures| decl_identifiers_differ(&signatures))
}

/// Detects ****: every cluster member contains the
/// `.iter().map(|x| x.field.method()).collect()` idiom and the cluster
/// spans at least two distinct source files.
pub(super) fn is_rust_iter_collect_idiom_cluster(snippets: &[Snippet<'_>]) -> bool {
    if !is_multi_member_language_cluster(snippets, "rust") {
        return false;
    }
    spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id))
        && snippets.iter().all(snippet_contains_iter_collect_idiom)
}

/// Per-member signature for the Rust top-level declaration filter.
struct DeclSignature {
    /// Identifier text (module name or use-path leaf token bytes) used
    /// to confirm that members really do differ.
    identifier: Vec<u8>,
}

/// Extracts the declaration signature from one cluster member or
/// returns `None` when the member is not a single bodiless mod / use.
fn decl_signature(snippet: &Snippet<'_>) -> Option<DeclSignature> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet)?;
    let node = top_level_decl_node(tree.root_node(), range)?;
    let identifier = decl_identifier_bytes(node, snippet.source)?;
    Some(DeclSignature { identifier })
}

/// Walks the parse tree for the smallest `mod_item` (bodiless) or
/// `use_declaration` whose byte range matches `range` after trimming.
fn top_level_decl_node(root: Node<'_>, range: ByteRange) -> Option<Node<'_>> {
    let node = enclosing_kind(root, range, &["mod_item", "use_declaration"])?;
    if !decl_range_matches(node, range) {
        return None;
    }
    if node.kind() == "mod_item" && node.child_by_field_name("body").is_some() {
        return None;
    }
    Some(node)
}

/// Returns true when the reported byte range hugs the declaration node.
/// We accept a small leading window so a `pub` / `pub(crate)` visibility
/// modifier sitting just before `mod` / `use` still counts.
fn decl_range_matches(node: Node<'_>, range: ByteRange) -> bool {
    let node_start = node.start_byte();
    let node_end = node.end_byte();
    let leading_slack = node_start.saturating_sub(range.start);
    node_end == range.end && leading_slack <= 16 && range.start <= node_start
}

/// Returns identifier bytes that distinguish two declarations: the
/// module name for `mod_item`, the full argument text for
/// `use_declaration`.
fn decl_identifier_bytes(node: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    let field = if node.kind() == "mod_item" {
        "name"
    } else {
        "argument"
    };
    let inner = node.child_by_field_name(field)?;
    source
        .get(inner.start_byte()..inner.end_byte())
        .map(<[u8]>::to_vec)
}

/// Returns true when at least two declarations differ in their
/// identifier text — the signal that distinguishes scaffolding from
/// genuine copy-paste of one identical declaration.
fn decl_identifiers_differ(signatures: &[DeclSignature]) -> bool {
    let Some(first) = signatures.first() else {
        return false;
    };
    signatures
        .iter()
        .any(|signature| signature.identifier != first.identifier)
}

/// Returns true when `snippet` contains the
/// `.iter().map(|x| x.field.method(...)).collect()` chain at any depth.
fn snippet_contains_iter_collect_idiom(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = match trimmed_snippet_range(snippet) {
        Some(range) => range,
        None => snippet.range,
    };
    find_iter_collect_idiom(tree.root_node(), range, snippet.source)
}

/// Recursively scans `node` (restricted to `range`) for the idiom.
fn find_iter_collect_idiom(node: Node<'_>, range: ByteRange, source: &[u8]) -> bool {
    if !node_intersects_range(node, range) {
        return false;
    }
    if node.kind() == "call_expression" && call_is_iter_collect_idiom(node, source) {
        return true;
    }
    named_children(node)
        .into_iter()
        .any(|child| find_iter_collect_idiom(child, range, source))
}

/// Returns true when `call` is a `.collect()` / `.collect::<...>()` call
/// whose receiver is `.iter().map(<single-method-closure>)`.
fn call_is_iter_collect_idiom(call: Node<'_>, source: &[u8]) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let Some(receiver) = receiver_for_method(function, source, "collect") else {
        return false;
    };
    map_call_has_field_method_closure(receiver, source)
}

/// Returns the receiver expression when `function` is a `field_expression`
/// or `generic_function` selecting the named method (e.g. `collect` or
/// `collect::<Vec<_>>`).
fn receiver_for_method<'tree>(
    function: Node<'tree>,
    source: &[u8],
    name: &str,
) -> Option<Node<'tree>> {
    let field_expression = if function.kind() == "generic_function" {
        function.child_by_field_name("function")?
    } else {
        function
    };
    if field_expression.kind() != "field_expression" {
        return None;
    }
    let field = field_expression.child_by_field_name("field")?;
    if field.kind() != "field_identifier" {
        return None;
    }
    let field_bytes = source.get(field.start_byte()..field.end_byte())?;
    if field_bytes != name.as_bytes() {
        return None;
    }
    field_expression.child_by_field_name("value")
}

/// Returns true when `receiver` is a `<expr>.iter().map(|x| body)` call
/// whose closure body is a single `x.field.method(...)` access.
fn map_call_has_field_method_closure(receiver: Node<'_>, source: &[u8]) -> bool {
    if receiver.kind() != "call_expression" {
        return false;
    }
    let Some(map_callee) = receiver.child_by_field_name("function") else {
        return false;
    };
    let Some(iter_receiver) = receiver_for_method(map_callee, source, "map") else {
        return false;
    };
    if !receiver_chain_has_iter_call(iter_receiver, source) {
        return false;
    }
    let Some(map_args) = receiver.child_by_field_name("arguments") else {
        return false;
    };
    closure_body_is_field_method_call(map_args, source)
}

/// Returns true when `expr` is itself a `.iter()` call expression with
/// no arguments — that's the start of the idiom chain.
fn receiver_chain_has_iter_call(expr: Node<'_>, source: &[u8]) -> bool {
    if expr.kind() != "call_expression" {
        return false;
    }
    let Some(function) = expr.child_by_field_name("function") else {
        return false;
    };
    receiver_for_method(function, source, "iter").is_some()
}

/// Returns true when the call's `arguments` is exactly one closure whose
/// body is a `closure_arg.field.method(...)` call expression.
fn closure_body_is_field_method_call(arguments: Node<'_>, source: &[u8]) -> bool {
    let closures: Vec<Node<'_>> = named_children(arguments)
        .into_iter()
        .filter(|child| child.kind() == "closure_expression")
        .collect();
    let [closure] = closures.as_slice() else {
        return false;
    };
    let Some(closure_arg) = sole_closure_parameter_bytes(*closure, source) else {
        return false;
    };
    let Some(body) = closure.child_by_field_name("body") else {
        return false;
    };
    closure_body_matches_field_method(body, source, closure_arg)
}

/// Returns the byte range of the single identifier parameter declared
/// by the closure, or `None` when the closure has zero / multiple / non
/// trivial parameters (we only suppress single-arg field projections).
fn sole_closure_parameter_bytes<'a>(closure: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    let parameters = closure.child_by_field_name("parameters")?;
    let named = named_children(parameters);
    let [first] = named.as_slice() else {
        return None;
    };
    if first.kind() != "identifier" {
        return None;
    }
    source.get(first.start_byte()..first.end_byte())
}

/// Returns true when `body` is exactly `<closure_arg>.<field>.<method>(...)`.
fn closure_body_matches_field_method(body: Node<'_>, source: &[u8], closure_arg: &[u8]) -> bool {
    if body.kind() != "call_expression" {
        return false;
    }
    let Some(function) = body.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "field_expression" {
        return false;
    }
    let Some(method_field) = function.child_by_field_name("field") else {
        return false;
    };
    if method_field.kind() != "field_identifier" {
        return false;
    }
    let Some(field_value) = function.child_by_field_name("value") else {
        return false;
    };
    field_expression_projects_closure_arg(field_value, source, closure_arg)
}

/// Detects **** [CLONE-NOISE-RUST-MATCH-DISPATCH]: clusters
/// whose every member is a run of `match` arms inside one dispatch
/// `match`. Each arm routes a distinct pattern (command key) to a
/// distinct handler, so after identifier and literal normalisation every
/// arm collapses to the same `<path>::<ident> => Ok(<call>(...))` shape
/// and the sibling-window pass matches one window of arms against another
/// within the same `match`. A routing table is not extractable
/// duplication.
///
/// Suppressed only when the matched arm patterns across the cluster are
/// pairwise distinct (distinct dispatch keys ⇒ a routing table) and at
/// least two members differ in raw bytes, so a verbatim copy-pasted run
/// of arms still surfaces as a genuine clone.
pub(super) fn is_rust_match_dispatch_cluster(snippets: &[Snippet<'_>]) -> bool {
    language_cluster_shapes(snippets, "rust", match_dispatch_arm_patterns).is_some_and(|patterns| {
        let all: Vec<&[u8]> = patterns.iter().flatten().map(Vec::as_slice).collect();
        arm_patterns_pairwise_distinct(&all) && raw_snippet_texts_differ(snippets)
    })
}

/// Returns the pattern bytes of every `match_arm` covered by one cluster
/// member's range, or `None` when the member is not a contiguous run of
/// arm siblings under a single `match_block`.
fn match_dispatch_arm_patterns(snippet: &Snippet<'_>) -> Option<Vec<Vec<u8>>> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet)?;
    let arms = KindSearch::intersecting(range, |kind| kind == "match_arm").nodes(tree.root_node());
    let first = arms.first()?;
    let parent = first.parent()?;
    if parent.kind() != "match_block" || arms.iter().any(|arm| arm.parent() != Some(parent)) {
        return None;
    }
    arms.iter()
        .map(|arm| arm_pattern_bytes(*arm, snippet.source))
        .collect()
}

/// Returns the `pattern` field bytes of one `match_arm`.
fn arm_pattern_bytes(arm: Node<'_>, source: &[u8]) -> Option<Vec<u8>> {
    let pattern = arm.child_by_field_name("pattern")?;
    source
        .get(pattern.start_byte()..pattern.end_byte())
        .map(<[u8]>::to_vec)
}

/// Returns true when the cluster carries at least two arms and every arm
/// pattern is distinct — the signal that the `match` is a dispatch table
/// rather than a run of byte-identical copy-pasted arms.
fn arm_patterns_pairwise_distinct(patterns: &[&[u8]]) -> bool {
    if patterns.len() < 2 {
        return false;
    }
    patterns.iter().copied().collect::<BTreeSet<&[u8]>>().len() == patterns.len()
}

/// Returns true when `node` is `<closure_arg>.<field>` (one field hop).
fn field_expression_projects_closure_arg(
    node: Node<'_>,
    source: &[u8],
    closure_arg: &[u8],
) -> bool {
    if node.kind() != "field_expression" {
        return false;
    }
    let Some(base) = node.child_by_field_name("value") else {
        return false;
    };
    if base.kind() != "identifier" {
        return false;
    }
    let base_bytes = source.get(base.start_byte()..base.end_byte());
    if base_bytes != Some(closure_arg) {
        return false;
    }
    node.child_by_field_name("field")
        .is_some_and(|field| field.kind() == "field_identifier")
}

/// Detects **** [CLONE-NOISE-RUST-STRUCT-FIELDS]: clusters whose
/// every member covers only Rust struct field declarations. A field list
/// encodes a data model's *shape*, not extractable duplicate logic — after
/// identifier, type, and literal normalisation `pub a: Option<String>`
/// collapses to the same subtree as `pub b: Option<String>`, so unrelated
/// field runs (and whole structs that are nothing but such fields) cluster as
/// `structural_only`. No refactor removes them, yet they dominate the
/// duplication metric on serde-heavy repos. This is the Rust counterpart of
/// the Dart class-field filter.
///
/// Guarded by `raw_snippet_texts_differ` exactly like the Dart filter: a
/// byte-identical copy-pasted struct still surfaces as genuine duplication
/// rather than being mistaken for a run of distinct fields.
pub(super) fn is_rust_struct_field_declaration_cluster(snippets: &[Snippet<'_>]) -> bool {
    is_multi_member_language_cluster(snippets, "rust")
        && snippets.iter().all(covers_only_struct_field_declarations)
        && raw_snippet_texts_differ(snippets)
}

/// Returns true when `snippet.range` covers only Rust struct field
/// declarations — a run of sibling fields inside one `field_declaration_list`,
/// or one whole `struct_item` whose body is nothing but field declarations.
/// Any method, impl, or other item body fails the check and keeps clustering.
fn covers_only_struct_field_declarations(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let root = tree.root_node();
    // A run of sibling fields inside one struct's field list.
    if let Some(list) = enclosing_kind(root, range, &["field_declaration_list"]) {
        return field_list_range_is_all_fields(list, range);
    }
    // Otherwise the range is a whole-struct window: the smallest node spanning
    // it is the struct itself (range hugs one `struct_item`), its field list,
    // or a container holding one or more whole structs plus their leading
    // `#[derive]` / `#[serde]` attribute siblings.
    let Some(container) = root.descendant_for_byte_range(range.start, range.end) else {
        return false;
    };
    match container.kind() {
        "field_declaration_list" => field_list_range_is_all_fields(container, range),
        "struct_item" => struct_body_in_range_is_all_fields(container, range),
        _ => range_covers_only_field_structs(container, range),
    }
}

/// Returns true when one `struct_item`'s body covers only field declarations
/// within `range`.
fn struct_body_in_range_is_all_fields(struct_item: Node<'_>, range: ByteRange) -> bool {
    field_declaration_list_of(struct_item)
        .is_some_and(|list| field_list_range_is_all_fields(list, range))
}

/// Returns true when every named child of `container` that `range` covers is a
/// whole struct whose in-range body is only field declarations, or a leading
/// attribute, import, or comment sibling — and at least one field is covered.
/// Any function, impl, or other item keeps the cluster.
///
/// `use_declaration` is on that list because a sibling window over a data
/// model starts where the file starts. `host.rs` and `manifest.rs` in the
/// #224 fixture hold nothing but distinct serde structs, and the selected
/// window opened on their shared `use serde::{Deserialize, Serialize};` —
/// one non-struct sibling, and the whole 125-node view escaped the filter
/// and published `structural_only` across both files. An import is already
/// boilerplate in its own right ([CLONE-NOISE-RUST-DECL]), so a run of
/// distinct field structs does not stop being a data model because one sits
/// above it. The `saw_field` requirement is what keeps this from
/// swallowing an import-only window, and `raw_snippet_texts_differ` still
/// lets a byte-identical copy-pasted struct through.
fn range_covers_only_field_structs(container: Node<'_>, range: ByteRange) -> bool {
    let mut saw_field = false;
    for child in named_children(container) {
        if !node_intersects_range(child, range) {
            continue;
        }
        match child.kind() {
            "struct_item" => {
                let Some(list) = field_declaration_list_of(child) else {
                    return false;
                };
                if node_intersects_range(list, range) {
                    if !field_list_range_is_all_fields(list, range) {
                        return false;
                    }
                    saw_field = true;
                }
            }
            "attribute_item" | "use_declaration" => {}
            kind if kind.ends_with("comment") => {}
            _ => return false,
        }
    }
    saw_field
}

/// Returns the direct `field_declaration_list` child of a `struct_item`, or
/// `None` for a tuple/unit struct that has none.
fn field_declaration_list_of(struct_item: Node<'_>) -> Option<Node<'_>> {
    named_children(struct_item)
        .into_iter()
        .find(|child| child.kind() == "field_declaration_list")
}

/// Returns true when every named child of `list` that intersects `range` is a
/// `field_declaration` or its `attribute_item`, with at least one field
/// present.
fn field_list_range_is_all_fields(list: Node<'_>, range: ByteRange) -> bool {
    let mut fields = 0_usize;
    for member in named_children(list) {
        if !node_intersects_range(member, range) {
            continue;
        }
        match member.kind() {
            "field_declaration" => fields = fields.saturating_add(1),
            "attribute_item" => {}
            kind if kind.ends_with("comment") => {}
            _ => return false,
        }
    }
    fields >= 1
}
