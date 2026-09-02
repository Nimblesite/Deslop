//! Extension coverage and language-boundary E2E tests for the JavaScript
//! and TypeScript surfaces ([LANG-CAND-JAVASCRIPT], [LANG-CAND-TYPESCRIPT],
//! [PIPELINE-LANG-TRAIT]).
//!
//! `JavaScriptParser` claims `js`/`mjs`/`cjs`/`jsx` and feeds them through
//! one grammar, so all four must be discovered and must cluster together as
//! the single `javascript` language. `ts` and `tsx` are deliberately
//! separate languages (separate tree-sitter entry points), so the default
//! same-language clone filter must never merge a `.ts` clone with a `.tsx`
//! one. These black-box tests pin both halves of that contract.

use std::{collections::BTreeSet, ffi::OsStr, ops::RangeInclusive, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, distinct_texts,
    has_verbatim_pair, rank_of,
};
use crate::common::*;

/// The fixture whose three files carry one `reconcileInventory` routine
/// under the `.js`, `.mjs` and `.cjs` extensions.
const JS_FAMILY_FIXTURE: &str = "js-mjs-cjs-family";
/// Every file of the family, in report order.
const JS_FAMILY_FILES: [&str; 3] = ["inventory.js", "ledger.mjs", "stock.cjs"];
/// The 1-based lines of the `reconcileInventory` declaration in every
/// file of the family — the extent each published copy must have.
const JS_FAMILY_FUNCTION_LINES: RangeInclusive<u64> = 3..=17;
/// The `--min-nodes` the family is scanned at.
const JS_FAMILY_MIN_NODES: u32 = 8;
/// The two files that are one another's copy in full: same shape, one
/// import specifier apart.
const JS_FAMILY_WHOLE_FILE_COPIES: [&str; 2] = ["inventory.js", "ledger.mjs"];
/// The 1-based lines of those whole-file copies.
const JS_FAMILY_WHOLE_FILE_LINES: RangeInclusive<u64> = 1..=17;

/// Asserts discovery reached every file the fixture holds. Without it a
/// "these do not cluster" verdict could pass vacuously on a parser that
/// silently dropped one side.
fn assert_files_analysed(report: &Value, expected: u64, why: &str) {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(expected),
        "{why}: {report:#}"
    );
}

#[test]
fn javascript_family_clusters_across_js_mjs_and_cjs_extensions() -> Result<()> {
    let scan_root = fixture(JS_FAMILY_FIXTURE);
    let report = run_report(&scan_root, JS_FAMILY_MIN_NODES)?;
    assert_files_analysed(
        &report,
        u64::try_from(JS_FAMILY_FILES.len())?,
        "every .js/.mjs/.cjs file must be discovered and analysed",
    );
    // All three extensions carry the same reconciliation routine; because
    // they are one language they cluster together into a single family.
    let clone = expect_cluster_spanning(&report, &JS_FAMILY_FILES)?;
    // [PIPELINE-CLUSTER-CLOSURE] The wire facts that hold the acceptance:
    // the family is admitted, mass-honest and clean-surfaced.
    assert_structural_only_contract(clone, JS_FAMILY_FIXTURE);
    assert_no_pair_surface_on_cluster(clone, JS_FAMILY_FIXTURE);
    assert_eq!(
        distinct_texts(&scan_root, clone)?.len(),
        1,
        "the three copies must slice to one byte-identical text: {clone:#}"
    );
    assert!(
        has_verbatim_pair(&scan_root, clone)?,
        "an unedited copy under three extensions is a verbatim family: {clone:#}"
    );
    assert_whole_file_copy_survives_beside_the_family(&report, &scan_root, clone)?;
    // [FUSED-SHARED-SUBTREE-ECHO] The `.cjs` copy is the declaration and
    // nothing around it: its `require` and `module.exports` lines are
    // duplicated nowhere, and a whole-file view welded onto the family
    // through a function body in one file against the whole of another
    // used to count them as duplicated code.
    let declaration: BTreeSet<u64> = JS_FAMILY_FUNCTION_LINES.collect();
    let whole_file: BTreeSet<u64> = JS_FAMILY_WHOLE_FILE_LINES.collect();
    let published = visible_duplicated_lines(&report);
    for file in JS_FAMILY_FILES {
        let expected = if JS_FAMILY_WHOLE_FILE_COPIES.contains(&file) {
            &whole_file
        } else {
            &declaration
        };
        assert_eq!(
            published.get(file),
            Some(expected),
            "{file} must publish exactly the lines its copies cover: {report:#}"
        );
    }
    assert_eq!(
        visible_duplicated_loc(&report),
        line_count(&declaration)
            + line_count(&whole_file) * u64::try_from(JS_FAMILY_WHOLE_FILE_COPIES.len())?,
        "the duplicated line count is the .cjs declaration plus two whole files: {report:#}"
    );
    Ok(())
}

/// [PIPELINE-CLUSTER-SUBSUME] The `.js` and `.mjs` files are one another's
/// copy in full — same shape, one import specifier apart — so the report
/// carries that whole-file view beside the three-way declaration family.
/// The wide view names no file the family does not, and is still never
/// dropped for it: a wide view is not a re-description of a narrower
/// family nested in it that also reaches a third file. The family, being
/// the heavier duplication, ranks first.
fn assert_whole_file_copy_survives_beside_the_family(
    report: &Value,
    scan_root: &Path,
    family: &Value,
) -> Result<()> {
    let published = clusters(report);
    assert_eq!(
        published.len(),
        2,
        "the declaration family and the whole-file copy are the report: {report:#}"
    );
    assert_eq!(rank_of(report, family)?, 0, "the family outranks the copy");
    let copy = published
        .iter()
        .find(|candidate| !std::ptr::eq(*candidate, family))
        .ok_or_else(|| anyhow::anyhow!("two clusters asserted above"))?;
    let copy_files: Vec<&str> = occurrences(copy)
        .iter()
        .map(occurrence_path)
        .collect::<Result<_>>()?;
    assert_eq!(
        copy_files, JS_FAMILY_WHOLE_FILE_COPIES,
        "the copy is the .js/.mjs pair"
    );
    for occurrence in occurrences(copy) {
        assert_eq!(
            (
                field(occurrence, "start_line").as_u64(),
                field(occurrence, "end_line").as_u64()
            ),
            (
                Some(*JS_FAMILY_WHOLE_FILE_LINES.start()),
                Some(*JS_FAMILY_WHOLE_FILE_LINES.end())
            ),
            "each copy is the whole file: {copy:#}"
        );
    }
    assert_structural_only_contract(copy, JS_FAMILY_FIXTURE);
    assert_no_pair_surface_on_cluster(copy, JS_FAMILY_FIXTURE);
    assert!(
        !has_verbatim_pair(scan_root, copy)?,
        "the copies differ by their import specifier alone: {copy:#}"
    );
    Ok(())
}

#[test]
fn js_and_jsx_cluster_as_the_same_javascript_language() -> Result<()> {
    // A `.jsx` file and a `.js` file carrying the same logic are both the
    // `javascript` language, so the same-language filter lets them cluster.
    // Both occurrences publish at the same authored extent — the
    // `buildBadgeModel` declaration, which the two files share byte for
    // byte ([PIPELINE-CLUSTER-EXACT-SCOPE]). The `export` keyword in front
    // of one copy is not part of the function and must not widen one
    // occurrence into a byte-distinct view of the other.
    assert_bucketed_clone("js-jsx-family", 10, &["BadgeList.jsx", "useBadge.js"], true)
}

#[test]
fn ts_and_tsx_clone_does_not_cluster_by_default() -> Result<()> {
    let report = run_report(&fixture("ts-tsx-separate"), 8)?;
    // Guard against a vacuous pass: both files must actually be analysed, so
    // "no clusters" proves the language boundary held rather than a parser
    // silently dropping one of them.
    assert_files_analysed(&report, 2, "both the .ts and .tsx file must be analysed");
    // One `.ts` file and one `.tsx` file that are near-identical must NOT
    // cluster: `typescript` and `tsx` are separate languages and cross-
    // language comparison is off by default.
    assert!(
        clusters(&report).is_empty(),
        "a lone .ts and a lone .tsx are different languages and must not cluster: {report:#}"
    );
    Ok(())
}

#[test]
fn ts_and_tsx_clones_stay_in_separate_language_clusters() -> Result<()> {
    let report = run_report(&fixture("ts-tsx-language-split"), 10)?;
    // The `.ts` pair clusters with the `.ts` pair and the `.tsx` pair with
    // the `.tsx` pair — two clusters that never share an occurrence.
    let typescript_pair = expect_cluster_spanning(&report, &["formatA.ts", "formatB.ts"])?;
    assert!(
        has_verbatim_pair(&fixture("ts-tsx-language-split"), typescript_pair)?,
        "the .ts pair is byte-proven: {typescript_pair:#}"
    );
    let react_pair = expect_cluster_spanning(&report, &["BadgeA.tsx", "BadgeB.tsx"])?;
    assert!(
        has_verbatim_pair(&fixture("ts-tsx-language-split"), react_pair)?,
        "the .tsx pair is byte-proven: {react_pair:#}"
    );
    for cluster in clusters(&report) {
        let extensions: BTreeSet<String> = cluster_file_set(cluster)
            .iter()
            .filter_map(|name| {
                Path::new(name)
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(ToOwned::to_owned)
            })
            .collect();
        assert!(
            !(extensions.contains("ts") && extensions.contains("tsx")),
            "no cluster may mix .ts and .tsx occurrences (separate languages): {cluster:#}"
        );
    }
    Ok(())
}
