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
//! # What every suppression must prove
//!
//! **Is this window a family view?** Two shapes qualify, and both are
//! AST facts about what the window *covers*, never about how many
//! members the cluster has:
//!
//! - a run of **two or more sibling declarations** — the plural
//!   settings/CRUD window; or
//! - **one declaration that forwards** — a body with a single client
//!   call, optionally bound and returned, and nothing computed on the
//!   way through ([RANK-STRUCTURAL-ONLY-FORWARDING], `forwarding.rs`).
//!
//! The second shape is what the meilisearch surface actually is. Its
//! `resetSettings` / `resetStopWords` wrappers are one statement each,
//! so every fingerprint window covers exactly one declaration and a
//! plurality-only test could never reach them. Counting declarations
//! alone therefore left the real #197 family on the report; asking what
//! the declaration *does* hides it without touching anything liftable.
//!
//! **Do the members differ in substance?** `ContentEvidence::
//! substance_varies` — a literal disagrees, or the identifier
//! substitution needs more than one consistent mapping. Two windows that
//! duplicate their substance are a clone, not scaffolding.
//!
//! **Do any two proven wrappers share a body?** Two siblings forwarding
//! to the same route are a copy-paste bug — one of those calls is dead
//! or misaimed — so one shared body disqualifies the suppression for the
//! whole family. The comparison is on bodies, not reported windows:
//! sibling declarations differ in their method names, so their windows
//! never compare equal even when the duplication is exact.
//!
//! **Is it a data table?** A table varies its literals by construction,
//! so the substance test convicts every one of them. Its payload *is*
//! its substance, and the `data_clones` policy already lets the user
//! demote, drop or keep it ([RANK-CATEGORY]).
//!
//! # What a count may never do
//!
//! `if members.len() < 3 { return false }` — a *cluster*-size threshold
//! standing in for a structural question, the same defect class as the
//! literal-variation shortcut in `calls.rs`. It let a two-window
//! settings family top the report. Neither test below is that threshold
//! wearing a new constant: they do not count cluster members, they ask
//! what a member *is*.
//!
//! A statement count is the same mistake one level down. "One or two
//! statements means scaffolding" convicts `csharp-merge-rename`'s
//! `TotalWithTax` — a loop, arithmetic and an accumulator in a short
//! body — and acquits a `getX()` wrapper that spends two statements on
//! a temporary. The forwarding proof matches the data-flow shape
//! instead, which is why it separates the meilisearch wrappers from
//! `csharp-merge-drift`'s `ApplyStandard` / `ApplyPremium`: those bind
//! two locals, call five collaborators and branch on a ceiling.
//!
//! # Why the substance test cannot decide alone
//!
//! Differing literals are exactly what a parameterised merge *fixes*.
//! `ApplyStandard`/`ApplyPremium` differ only in `"standard"`/`100`
//! versus `"premium"`/`250`; suppressing on literal variation alone
//! erased them and took the LSP merge offer and its refusal reason with
//! it. Content evidence is also *cluster-wide* — one divergent sibling
//! sets it for every member — so it can say "something here varies" and
//! never "all of this is scaffolding". Only the window tests can.
//!
//! Every one of those errors stays pinned:
//! `single_file_structural_only_method_families_do_not_top_the_report`,
//! `declaration_family_plurality`,
//! `declaration_family_mixed_component`,
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
    collect_snippets, enclosing_kind, forwarding::forwarding_body, node_intersects_range,
    parse_for, spans_multiple_files, uniform_language, ParseCache, Snippet,
};

/// Returns true when `cluster` is a single-file family of sibling
/// declarations rather than real duplication ([RANK-STRUCTURAL-ONLY]).
///
/// Suppression requires positive proof on every count: the members live
/// in one file, they are not a data table, their collapsed leaves prove
/// they differ in substance, and every window is a family view — two or
/// more sibling declarations, or one declaration proven to forward, no
/// two wrappers sharing a body. A cluster that fails any of them
/// is not proven to be scaffolding and stays visible, demoted by the
/// `structural_only` policy.
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
    collect_snippets(&cluster.members, sources, language, cache)
        .is_some_and(|snippets| every_window_is_family_noise(&snippets))
}

/// Returns true when every window is a family view and no two proven
/// wrappers share a body.
///
/// The distinctness check is what keeps a copy-paste bug visible. Two
/// sibling wrappers that forward to the *same* route are real
/// duplication — one of the two calls is dead — but their declarations
/// differ in the method name, so their reported windows never compare
/// equal. Comparing the proven bodies is the only view that sees it.
fn every_window_is_family_noise(snippets: &[Snippet<'_>]) -> bool {
    let mut wrapper_bodies = HashSet::new();
    snippets.iter().all(|snippet| match family_window(snippet) {
        None => false,
        Some(FamilyWindow::SiblingRun) => true,
        Some(FamilyWindow::Wrapper(body)) => wrapper_bodies.insert(body),
    })
}

/// What a fingerprint window covers, when it covers family noise at all.
enum FamilyWindow<'a> {
    /// Two or more sibling declarations of one container.
    SiblingRun,
    /// Exactly one declaration proven to forward, with its body bytes.
    Wrapper(&'a [u8]),
}

/// Classifies the snippet's window: two or more members of one
/// declaration container — the plural REST/settings run — or exactly one
/// member whose body proves the forwarding shape
/// ([RANK-STRUCTURAL-ONLY-FORWARDING]). A window over statements inside
/// a logic-bearing member is neither and returns `None`.
///
/// Counts container *members* rather than matching per-language
/// declaration node kinds. tree-sitter-dart has no `method_declaration`
/// at all — a class member is a generic node identified by the
/// `function_body` it carries — so a kind list is both grammar-specific
/// and wrong on the very language this filter exists for. The children
/// of one class body are siblings by construction.
fn family_window<'a>(snippet: &Snippet<'a>) -> Option<FamilyWindow<'a>> {
    let tree = parse_for(snippet)?;
    let container = enclosing_kind(
        tree.root_node(),
        snippet.range,
        container_kinds(snippet.language),
    )
    .filter(|container| is_declaration_container(*container))?;
    match covered_members(container, snippet).as_slice() {
        [] => None,
        [member] => {
            forwarding_body(*member, snippet.language, snippet.source).map(FamilyWindow::Wrapper)
        }
        _ => Some(FamilyWindow::SiblingRun),
    }
}

/// The container's named children that the snippet's window touches.
fn covered_members<'tree>(container: Node<'tree>, snippet: &Snippet<'_>) -> Vec<Node<'tree>> {
    let mut cursor = container.walk();
    let members = container
        .named_children(&mut cursor)
        .filter(|member| node_intersects_range(*member, snippet.range))
        .collect();
    members
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
