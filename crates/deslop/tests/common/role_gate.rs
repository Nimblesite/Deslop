//! The GH #119 role-gate contract, asserted once for every language
//! ([CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]).
//!
//! The gate requires an embedding-dominant pair to be role/context
//! compatible — all classes, or all functions — before it enters
//! closure, because a class definition and a function have no safe shared
//! extraction. Proving that takes two assertions per language and
//! they pull in opposite directions: the offending cross-role pair must
//! be suppressed, and a same-role behaviour-equivalent pair must not be.
//!
//! Both are written here rather than per language. A per-suite copy is
//! duplication this repository's own gate counts against it, and worse,
//! it lets one language's copy drift into asserting less than another's
//! — which is exactly how a role gate comes to be "covered" in four
//! places and enforced in none.
//!
//! Imported explicitly with `use crate::common::role_gate::*;`, for the
//! same reason as `signals`.

use std::path::Path;

use serde_json::Value;

use super::{
    clusters, clusters_hidden, embeddings::scan_fixture_copy_with_mock, fixture, occurrence_texts,
    signals::assert_structural_only_contract, Result,
};

/// Subtree-size floor every #119 fixture is scanned at.
pub(crate) const ROLE_GATE_MIN_NODES: &str = "5";
const NO_HIDDEN_COMPONENTS: u64 = 0;

/// [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] acceptance: an embedding-dominant
/// pair whose members have different top-level roles — a `class`
/// definition and a top-level function — must be rejected before closure.
///
/// `class_marker` and `function_marker` name source text unique to each
/// role, so a surviving cluster covering both is the role-mismatch
/// signature; matching on the rendered occurrence text is what makes this
/// a black-box assertion about the report rather than about internals.
pub(crate) fn assert_role_mismatch_is_suppressed(
    fixture_name: &str,
    language: &str,
    endpoint: &str,
    class_marker: &str,
    function_marker: &str,
) -> Result<()> {
    let scan_root = fixture(fixture_name);
    let report = scan_fixture_copy_with_mock(&scan_root, ROLE_GATE_MIN_NODES, endpoint)?;
    let offenders = cross_role_pairs(&report, &scan_root, class_marker, function_marker)?;
    assert!(
        offenders.is_empty(),
        "a {language} class paired with a top-level function by the embedding \
         pass must not surface as duplication — there is no safe cross-role \
         extraction: {offenders:#?}"
    );
    assert!(
        clusters(&report).is_empty(),
        "the role-incompatible {language} pair must not form any component: {report:#}"
    );
    assert_eq!(
        clusters_hidden(&report),
        NO_HIDDEN_COMPONENTS,
        "role incompatibility rejects the pair before suppression: {report:#}"
    );
    Ok(())
}

/// Over-suppression guard: two genuinely behaviour-equivalent functions
/// share one top-level role, so the gate must not hide them. They surface
/// pairing both named functions; the same-role admission is proven from
/// the rendered text (black-box), not from a bucket label.
pub(crate) fn assert_same_role_pair_surfaces(
    fixture_name: &str,
    language: &str,
    endpoint: &str,
    recursive_marker: &str,
    iterative_marker: &str,
) -> Result<()> {
    let scan_root = fixture(fixture_name);
    let report = scan_fixture_copy_with_mock(&scan_root, ROLE_GATE_MIN_NODES, endpoint)?;
    let surviving: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| {
            occurrence_texts(&scan_root, cluster).is_ok_and(|texts| {
                texts.iter().any(|text| text.contains(recursive_marker))
                    && texts.iter().any(|text| text.contains(iterative_marker))
            })
        })
        .collect();
    assert!(
        !surviving.is_empty(),
        "two same-role behaviour-equivalent {language} functions must surface \
         — the role gate must not over-suppress. Visible clusters: {:#?}",
        clusters(&report)
    );
    assert_pairs_both_members(
        &scan_root,
        &surviving,
        recursive_marker,
        iterative_marker,
        language,
    )?;
    for cluster in &surviving {
        assert_structural_only_contract(cluster, language);
    }
    Ok(())
}

/// Visible clusters whose occurrence text covers both roles at once.
fn cross_role_pairs(
    report: &Value,
    scan_root: &Path,
    class_marker: &str,
    function_marker: &str,
) -> Result<Vec<Vec<String>>> {
    let mut offenders = Vec::new();
    for cluster in clusters(report) {
        let texts = occurrence_texts(scan_root, cluster)?;
        let touches_class = texts.iter().any(|text| text.contains(class_marker));
        let touches_function = texts.iter().any(|text| text.contains(function_marker));
        if touches_class && touches_function {
            offenders.push(texts);
        }
    }
    Ok(offenders)
}

/// Asserts one surviving cluster covers both named members.
fn assert_pairs_both_members(
    scan_root: &Path,
    surviving: &[&Value],
    left: &str,
    right: &str,
    language: &str,
) -> Result<()> {
    let paired = surviving.iter().try_fold(false, |found, cluster| {
        let texts = occurrence_texts(scan_root, cluster)?;
        let touches_left = texts.iter().any(|text| text.contains(left));
        let touches_right = texts.iter().any(|text| text.contains(right));
        Ok::<bool, anyhow::Error>(found || (touches_left && touches_right))
    })?;
    assert!(
        paired,
        "the surviving {language} same-role cluster must pair `{left}` with \
         `{right}`: {surviving:#?}"
    );
    Ok(())
}
