//! Single-file sibling-declaration family filter
//! ([RANK-STRUCTURAL-ONLY]).
//!
//! Suppresses a single-file `structural_only` family of sibling
//! declarations: the in-class REST/CRUD, settings, builder or visitor
//! idiom, where each method shares a skeleton but targets a different
//! endpoint literal and return type, so the family fuses at
//! `structural = 1.00` with no token or embedding support and dominates
//! `top-offenders` purely on size.
//!
//! # The question this filter must answer
//!
//! Both a real clone and a declaration family are shape-identical in one
//! file — that is what put them in this bucket. The only thing that
//! separates them is what normalisation erased, so that is what gets
//! measured: **do the members preserve their literals and map their
//! identifiers through one consistent substitution?**
//!
//! - `csharp-merge-rename`'s `TotalWithTax`/`SubtotalWithTax` keep every
//!   literal (`0`, `100`) and rename only bound locals (`total`→`sum`,
//!   `taxed`→`levy`) — one bijection explains the pair. Real duplication,
//!   liftable by the merge planner, **kept**.
//! - The vendored meilisearch settings family changes the endpoint string
//!   at every method. No substitution explains a literal, so the members
//!   share a skeleton and nothing else. Scaffolding, **hidden**.
//!
//! # Two answers that are not allowed
//!
//! A **member count** cannot decide it. A three-member floor let a
//! two-window settings family walk straight through and top the report;
//! it is the same defect class as the literal-variation shortcut in
//! `calls.rs`, a size threshold standing in for a structural question.
//! A body-difference threshold expressed as a member count is that same
//! defect wearing a different constant.
//!
//! A **covering node kind** cannot decide it either.
//! `descendant_for_byte_range` returning `method_declaration` says only
//! *where* a member sits; asking it erased `csharp-merge-rename`
//! entirely, because a genuine two-method clone also sits in a
//! `method_declaration`.
//!
//! Both errors stay pinned:
//! `single_file_structural_only_method_families_do_not_top_the_report`
//! fails if the family surfaces, and
//! `refactor_merge::consistent_renames_lift_without_parameters` fails if
//! the clone is erased. Neither passes alone — only the pair states the
//! contract.

use crate::{clone_category::CloneCategory, cluster::Cluster};

use super::spans_multiple_files;

/// Returns true when `cluster` is a single-file family of sibling
/// declarations rather than real duplication ([RANK-STRUCTURAL-ONLY]).
///
/// Suppression requires positive proof of scaffolding on every count:
/// the members live in one file, they are not a data table, and their
/// collapsed leaves prove they differ in substance. A cluster whose
/// content could not be measured is not proven to be anything and stays
/// visible, demoted by the `structural_only` policy.
///
/// A **data-category** cluster is never a declaration family
/// ([RANK-CATEGORY]). A table of constructor rows varies its literals by
/// construction — that is what a table *is* — so the substance test
/// convicts every one of them. But a table's payload is its substance,
/// repeating it is a real finding, and the user already chooses its fate
/// through the three-way `data_clones` policy: demote, drop, or keep.
/// Hiding it here would erase both the finding and the choice.
pub(crate) fn is_single_file_declaration_family(
    cluster: &Cluster,
    category: CloneCategory,
) -> bool {
    category == CloneCategory::Logic
        && !cluster.members.is_empty()
        && !spans_multiple_files(cluster.members.iter().map(|member| member.file_id))
        && cluster.content.substance_varies
}
