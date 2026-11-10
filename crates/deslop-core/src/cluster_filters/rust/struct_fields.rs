use tree_sitter::Node;

use super::super::{
    enclosing_kind, node_intersects_range, parse_for, trimmed_snippet_range, Snippet,
};
use crate::ast::{named_children, ByteRange};

/// Returns true when `snippet.range` covers only Rust struct field
/// declarations — a run of sibling fields inside one `field_declaration_list`,
/// or one whole `struct_item` whose body is nothing but field declarations.
/// Any method, impl, or other item body fails the check and keeps clustering.
pub(super) fn covers_only_struct_field_declarations(snippet: &Snippet<'_>) -> bool {
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
