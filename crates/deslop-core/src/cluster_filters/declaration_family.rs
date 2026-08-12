//! Single-file sibling-declaration family filter.
//!
//! suppressed *cross-file* `structural_only` scaffolding
//! (test boilerplate replicated across many files). is the
//! single-file twin: an in-class sibling-method family — the REST/CRUD,
//! settings, builder, or visitor idiom — where each method shares a skeleton
//! but targets a different endpoint literal and return type, so they fuse at
//! `structural=1.00` with no token or embedding support and dominate
//! `top-offenders` purely on size.
//!
//! The discriminator is *declaration vs. statement*: a sibling-method family
//! lives directly under a type/module body, while a repeated block inside one
//! method body is a statement-window clone that may still be worth extracting.
//! We only suppress when every member's covering CST node is a top-level
//! declaration (or a window of sibling declarations), so statement-window
//! clones keep their full ranking weight.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{fingerprint::Fingerprint, state::FileId};

use super::snippets::{collect_snippets, parse_for, uniform_language, ParseCache, Snippet};

/// Returns true when the cluster is a single-file family of three or more
/// sibling declarations. The `structural_only` signal gate is applied by the
/// caller ([`crate::report`]); this function only answers the structural
/// question "are these members sibling declarations in one file?".
pub(crate) fn is_single_file_declaration_family<S: BuildHasher>(
    members: &[Fingerprint],
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> bool {
    if members.len() < 3 {
        return false;
    }
    let Some(first) = members.first() else {
        return false;
    };
    if !members.iter().all(|member| member.file_id == first.file_id) {
        return false;
    }
    let Some(language) = uniform_language(members, file_languages) else {
        return false;
    };
    let Some(snippets) = collect_snippets(members, sources, language, cache) else {
        return false;
    };
    snippets.iter().all(member_is_declaration_context)
}

/// True when the smallest CST node spanning the member's byte range is a
/// declaration (one method/class/…) or a body that holds sibling
/// declarations — as opposed to a statement block inside a method body.
fn member_is_declaration_context(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let start = snippet.range.start;
    let end_inclusive = snippet.range.end.saturating_sub(1).max(start);
    let Some(node) = tree
        .root_node()
        .descendant_for_byte_range(start, end_inclusive)
    else {
        return false;
    };
    is_declaration_kind(node.kind()) || is_declaration_body_kind(node.kind())
}

/// CST node kinds that are themselves a single top-level declaration across
/// the supported grammars. (`class_declaration` / `interface_declaration` /
/// `method_signature` are shared with the JS/TS family below.)
fn is_declaration_kind(kind: &str) -> bool {
    matches!(
        kind,
        // C#
        "method_declaration"
            | "constructor_declaration"
            | "property_declaration"
            | "field_declaration"
            | "class_declaration"
            | "struct_declaration"
            | "interface_declaration"
            | "record_declaration"
            // Rust
            | "function_item"
            | "struct_item"
            | "impl_item"
            | "trait_item"
            | "mod_item"
            | "enum_item"
            // Python
            | "function_definition"
            | "class_definition"
            | "decorated_definition"
            // Dart (`class_definition` is shared with Python above)
            | "method_signature"
            | "function_signature"
            | "declaration"
            | "extension_declaration"
            // JavaScript / TypeScript / TSX
            | "method_definition"
            | "public_field_definition"
            | "function_declaration"
            | "generator_function_declaration"
            | "abstract_class_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "property_signature"
    )
}

/// CST node kinds that hold a run of sibling declarations — the covering node
/// when a member spans more than one adjacent declaration.
fn is_declaration_body_kind(kind: &str) -> bool {
    matches!(
        kind,
        // C# / Rust type-and-module bodies
        "declaration_list"
            | "compilation_unit"
            | "source_file"
            // Dart (`class_body` / `enum_body` / `program` shared with JS/TS)
            | "class_body"
            | "extension_body"
            | "mixin_body"
            | "enum_body"
            | "program"
            // JavaScript / TypeScript / TSX
            | "interface_body"
            | "object_type"
            | "statement_block"
    )
}
