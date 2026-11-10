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
    node_search::KindSearch, parse_for, raw_snippet_texts_differ, spans_multiple_files,
    trimmed_snippet_range, Snippet,
};
use crate::ast::ByteRange;

/// The `.iter().map(|x| x.field.method()).collect()` idiom scan
/// backing [CLONE-NOISE-RUST-ITER-COLLECT].
mod iter_collect;
/// The `LanguageParser` adapter-shape scan backing
/// [CLONE-NOISE-RUST-LANGPARSER].
mod lang_parser;
/// The struct field-declaration scan backing
/// [CLONE-NOISE-RUST-STRUCT-FIELDS].
mod struct_fields;

/// Detects [CLONE-NOISE-RUST-LANGPARSER]: the Rust source files that
/// implement the first-party language plug-ins all carry the same
/// `LanguageParser` adapter surface. Each implementation has
/// language-specific constants and grammar functions, but the trait
/// contract forces the same method outline, so the cluster is not
/// actionable duplication.
pub(super) fn is_rust_language_parser_adapter_cluster(snippets: &[Snippet<'_>]) -> bool {
    spans_multiple_files(snippets.iter().map(|snippet| snippet.file_id))
        && language_cluster_shapes(
            snippets,
            "rust",
            lang_parser::rust_language_parser_impl_shape,
        )
        .is_some_and(|shapes| lang_parser::impl_shapes_form_adapter_family(&shapes))
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
        && snippets
            .iter()
            .all(iter_collect::snippet_contains_iter_collect_idiom)
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
        && snippets
            .iter()
            .all(struct_fields::covers_only_struct_field_declarations)
        && raw_snippet_texts_differ(snippets)
}
