//! Rust-specific cluster filters.
//!
//! Suppresses false-positive clusters whose shape is fixed by the Rust
//! language itself rather than by the program under analysis.
//!
//! Issues addressed (see parent `mod.rs` header):
//! - **#150** [CLONE-NOISE-RUST-DECL] — every member is a single `mod
//!   NAME;` or `use ...;` statement.
//! - **#147** [CLONE-NOISE-RUST-ITER-COLLECT] — every member is the
//!   `<expr>.iter().map(|x| x.<field>.<method>(...)).collect()` chain.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{enclosing_kind, parse_for, Snippet};
use crate::ast::ByteRange;

/// Detects **issue #150**: clusters whose every member is a single Rust
/// top-level declaration that has no body (`mod NAME;`, `use ...;`,
/// `pub use ...;`). Module declarations cannot be macro-generated in
/// Rust, so the cluster is not actionable.
pub(super) fn is_rust_top_level_decl_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "rust") {
        return false;
    }
    let signatures: Option<Vec<DeclSignature>> = snippets.iter().map(decl_signature).collect();
    let Some(signatures) = signatures else {
        return false;
    };
    decl_identifiers_differ(&signatures)
}

/// Detects **issue #147**: every cluster member contains the
/// `.iter().map(|x| x.field.method()).collect()` idiom and the cluster
/// spans at least two distinct source files.
pub(super) fn is_rust_iter_collect_idiom_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 || !snippets.iter().all(|snippet| snippet.language == "rust") {
        return false;
    }
    let mut files = BTreeSet::new();
    for snippet in snippets {
        let _inserted = files.insert(snippet.file_id);
    }
    files.len() >= 2 && snippets.iter().all(snippet_contains_iter_collect_idiom)
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
    let range = trimmed_range(snippet)?;
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
    let range = match trimmed_range(snippet) {
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
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| find_iter_collect_idiom(child, range, source));
    found
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
    let mut cursor = arguments.walk();
    let closures: Vec<Node<'_>> = arguments
        .named_children(&mut cursor)
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
    let mut cursor = parameters.walk();
    let mut named = parameters.named_children(&mut cursor);
    let first = named.next()?;
    if named.next().is_some() {
        return None;
    }
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

/// Returns true when `node` overlaps `range`.
fn node_intersects_range(node: Node<'_>, range: ByteRange) -> bool {
    node.start_byte() < range.end && node.end_byte() > range.start
}

/// Trims ASCII whitespace off both ends of `snippet.range` so a trailing
/// newline does not push the matched node outside the reported range.
fn trimmed_range(snippet: &Snippet<'_>) -> Option<ByteRange> {
    let bytes = snippet.source.get(snippet.range.start..snippet.range.end)?;
    let leading = bytes.iter().position(|byte| !byte.is_ascii_whitespace())?;
    let trailing = bytes.iter().rposition(|byte| !byte.is_ascii_whitespace())?;
    let start = snippet.range.start.checked_add(leading)?;
    let end = snippet.range.start.checked_add(trailing)?.checked_add(1)?;
    Some(ByteRange { start, end })
}
