//! Single-file sibling-declaration family filter
//! ([RANK-STRUCTURAL-ONLY]).
//!
//! Suppresses a single-file `structural_only` *family* of sibling
//! declarations: the in-class REST/CRUD, settings, builder or visitor
//! idiom, where each window covers a run of members that share a
//! skeleton but target a different endpoint literal and return type, so
//! the family fuses at `structural = 1.00` with no token or embedding
//! support and dominates `top-offenders` purely on size.
//!
//! # Three questions, and why each one is needed
//!
//! **Is it a family at all?** A family is *plural* — a window covering
//! one declaration is a unit of logic, however much it resembles its
//! neighbour. This is the question the filter is named for and the one
//! the deleted implementation never asked. It asked instead where a
//! member *sat* (`descendant_for_byte_range` returning
//! `method_declaration`), which is true of a genuine two-method clone as
//! well, and erased `csharp-merge-rename` outright.
//!
//! **Do the members differ in substance?** `ContentEvidence::
//! substance_varies` — a literal disagrees, or the identifier
//! substitution needs more than one consistent mapping. Two windows that
//! duplicate their substance are a clone, not scaffolding.
//!
//! **Is it a data table?** A table varies its literals by construction,
//! so the substance test convicts every one of them. Its payload *is*
//! its substance, and the `data_clones` policy already lets the user
//! demote, drop or keep it ([RANK-CATEGORY]).
//!
//! # What the member count may never do
//!
//! `if members.len() < 3 { return false }` — a *cluster*-size threshold
//! standing in for a structural question, the same defect class as the
//! literal-variation shortcut in `calls.rs`. It let a two-window
//! settings family top the report. The plurality test below is not that
//! threshold wearing a new constant: it does not count cluster members,
//! it asks what a single member *is*, and "two or more siblings" is the
//! definition of a family rather than a tuning knob.
//!
//! # Why the substance test cannot decide alone
//!
//! Differing literals are exactly what a parameterised merge *fixes*.
//! `csharp-merge-drift`'s `ApplyStandard`/`ApplyPremium` differ only in
//! `"standard"`/`100` versus `"premium"`/`250`; suppressing on literal
//! variation alone erased them and took the LSP merge offer and its
//! refusal reason with it. The plurality test is what keeps a
//! parameterisable pair visible while still suppressing the REST family
//! whose windows each span three methods.
//!
//! Every one of those errors stays pinned:
//! `single_file_structural_only_method_families_do_not_top_the_report`,
//! `refactor_merge::consistent_renames_lift_without_parameters`,
//! `issue_190_data_table_demote`, and the `csharp-merge-drift` LSP
//! `code_action` / `code_action_refusal` pair. No one of them passes
//! alone — only together do they state the contract.

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
};

use tree_sitter::Node;

use crate::{clone_category::CloneCategory, cluster::Cluster, state::FileId};

use super::{
    collect_snippets, enclosing_kind, forwarding::is_forwarding_declaration,
    node_intersects_range, parse_for, snippet_range_text, spans_multiple_files, uniform_language,
    ParseCache, Snippet,
};

/// Returns true when `cluster` is a single-file family of sibling
/// declarations rather than real duplication ([RANK-STRUCTURAL-ONLY]).
///
/// Suppression requires positive proof on every count: the members live
/// in one file, they are not a data table, their collapsed leaves prove
/// they differ in substance, and each window covers two or more sibling
/// declarations. A cluster that fails any of them is not proven to be
/// scaffolding and stays visible, demoted by the `structural_only`
/// policy.
pub(crate) fn is_single_file_declaration_family<S: BuildHasher>(
    cluster: &Cluster,
    category: CloneCategory,
    sources: &HashMap<FileId, Vec<u8>>,
    file_languages: &HashMap<FileId, &'static str, S>,
    cache: &ParseCache,
) -> bool {
    if category != CloneCategory::Logic
        || cluster.members.is_empty()
        || spans_multiple_files(cluster.members.iter().map(|member| member.file_id))
        || !cluster.content.substance_varies
    {
        return false;
    }
    let Some(language) = uniform_language(&cluster.members, file_languages) else {
        return false;
    };
    collect_snippets(&cluster.members, sources, language, cache).is_some_and(|snippets| {
        snippets_pairwise_distinct(&snippets)
            && snippets.iter().all(covers_declaration_family_window)
    })
}

/// Returns true when no two members' reported bytes are identical. A
/// byte-identical pair is real duplication whatever the members are —
/// even two forwarding wrappers, copied verbatim, are a copy to report —
/// so it disqualifies the whole suppression. An unreadable member also
/// fails open here.
fn snippets_pairwise_distinct(snippets: &[Snippet<'_>]) -> bool {
    let mut seen = HashSet::new();
    snippets.iter().all(|snippet| {
        snippet_range_text(snippet).is_some_and(|text| seen.insert(text))
    })
}

/// Returns true when the snippet's window is a declaration-family view:
/// either it covers two or more members of one declaration container —
/// the plural REST/settings run — or it covers exactly one member whose
/// body proves the forwarding shape
/// ([RANK-STRUCTURAL-ONLY-FORWARDING]). A window over statements inside
/// a logic-bearing member matches neither and stays visible.
///
/// Counts container *members* rather than matching per-language
/// declaration node kinds. tree-sitter-dart has no `method_declaration`
/// at all — a class member is a generic node identified by the
/// `function_body` it carries — so a kind list is both grammar-specific
/// and wrong on the very language this filter exists for. The children
/// of one class body are siblings by construction.
fn covers_declaration_family_window(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let Some(container) = enclosing_kind(
        tree.root_node(),
        snippet.range,
        container_kinds(snippet.language),
    ) else {
        return false;
    };
    if !is_declaration_container(container) {
        return false;
    }
    let mut cursor = container.walk();
    let members: Vec<Node<'_>> = container
        .named_children(&mut cursor)
        .filter(|member| node_intersects_range(*member, snippet.range))
        .collect();
    match members.as_slice() {
        [] => false,
        [member] => is_forwarding_declaration(*member, snippet.language, snippet.source),
        _ => true,
    }
}

/// Python reuses `block` for class *and* function bodies, so the parent
/// decides: statements inside a function are not sibling declarations.
/// Every other container kind below is class-like by construction.
fn is_declaration_container(container: Node<'_>) -> bool {
    container.kind() != "block"
        || container
            .parent()
            .is_some_and(|parent| parent.kind() == "class_definition")
}

/// Tree-sitter node kinds that hold a run of sibling member
/// declarations. Deliberately class-like only: a statement block's
/// children are statements, and counting those would suppress exactly
/// the multi-statement window clones this filter must keep.
const fn container_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"csharp" | b"rust" => &["declaration_list"],
        b"dart" => &["class_body", "extension_body", "mixin_body", "enum_body"],
        b"python" => &["block"],
        b"javascript" | b"typescript" | b"tsx" => &["class_body", "interface_body", "object_type"],
        _ => &[],
    }
}
