//! Single-file sibling-declaration family filter — **QUARANTINED**
//! ([RANK-STRUCTURAL-ONLY]).
//!
//! The filter suppressed a single-file `structural_only` family of
//! sibling declarations: the in-class REST/CRUD, settings, builder or
//! visitor idiom, where each method shares a skeleton but targets a
//! different endpoint literal and return type, so the family fuses at
//! `structural = 1.00` with no token or embedding support and dominates
//! `top-offenders` purely on size.
//!
//! **It could not tell that family from real duplication, in either
//! configuration, and both errors are pinned by a red test.**
//!
//! It asked two questions. The first was a bare member count — suppress
//! only families of three or more — which is the same defect class as
//! the literal-variation shortcut in `calls.rs`: a size threshold
//! standing in for a structural question. With the floor in place a
//! *two*-window settings family walks straight through: two 1.5 KB spans
//! of `get`/`reset`/`update` methods in one Dart file, fused at
//! `structural = 1.00` with `token_jaccard = 0.00`, topping the report as
//! exactly the REST surface this filter exists to suppress. That is a
//! **false positive**, pinned by
//! `single_file_structural_only_method_families_do_not_top_the_report`.
//!
//! The second was the CST discriminator below — declaration context vs.
//! statement context — which was meant to answer the same question
//! honestly and let the floor go. It does not. With the floor removed,
//! the `csharp-merge-rename` fixture produces **zero clusters**: a
//! genuine two-method C# clone under consistent renames, which the merge
//! planner can lift, is classified a sibling-declaration family and
//! erased from the report. That is a **false negative**, pinned by
//! `refactor_merge::consistent_renames_lift_without_parameters`.
//!
//! Neither setting is correct and the choice between them is a choice of
//! which error to ship. `descendant_for_byte_range` returning
//! `method_declaration` says only *where* the member sits, not whether
//! its siblings differ solely in their literals — and that, not the
//! member count and not the covering node kind, is the question the
//! filter has to answer. The deleted code is reproduced in
//! `docs/plans/quarantine-repair-plan.md`; a replacement must compare the
//! members' normalized bodies and be green on **both** pinning tests
//! before it returns anything.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{fingerprint::Fingerprint, state::FileId};

use super::snippets::ParseCache;

/// Quarantined: see the module docs. Answering this question wrongly
/// either erases real duplication from the report or hands the user a
/// REST surface as its top offender, and the deleted implementation did
/// one or the other depending on a member-count constant.
///
/// # Panics
///
/// Always. The caller reaches here for every single-file
/// `structural_only` cluster, so any repository containing one aborts
/// the run until the filter is rebuilt.
#[allow(clippy::panic)]
pub(crate) fn is_single_file_declaration_family<S: BuildHasher>(
    _members: &[Fingerprint],
    _sources: &HashMap<FileId, Vec<u8>>,
    _file_languages: &HashMap<FileId, &'static str, S>,
    _cache: &ParseCache,
) -> bool {
    panic!(
        "[RANK-STRUCTURAL-ONLY] single-file declaration-family suppression is \
         quarantined: with a member-count floor it published a two-window Dart \
         settings family as the top offender \
         (single_file_structural_only_method_families_do_not_top_the_report), \
         and without one it erased the csharp-merge-rename clone entirely \
         (refactor_merge::consistent_renames_lift_without_parameters). The CST \
         declaration-vs-statement discriminator cannot separate the two; see \
         docs/plans/quarantine-repair-plan.md"
    )
}
