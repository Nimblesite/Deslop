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

use std::{collections::BTreeSet, ffi::OsStr, path::Path};

use anyhow::Result;

mod common;
use crate::common::*;

#[test]
fn javascript_family_clusters_across_js_mjs_and_cjs_extensions() -> Result<()> {
    let report = run_report(&fixture("js-mjs-cjs-family"), 8)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(3),
        "every .js/.mjs/.cjs file must be discovered and analysed: {report:#}"
    );
    // All three extensions carry the same reconciliation routine; because
    // they are one language they cluster together into a single family.
    let clone = expect_cluster_spanning(&report, &["inventory.js", "ledger.mjs", "stock.cjs"])?;
    assert_eq!(cluster_bucket(clone), "identical");
    assert!(approx(signal(clone, "structural"), 1.0));
    assert!(approx(signal(clone, "token_jaccard"), 1.0));
    Ok(())
}

#[test]
fn js_and_jsx_cluster_as_the_same_javascript_language() -> Result<()> {
    let report = run_report(&fixture("js-jsx-family"), 10)?;
    // A `.jsx` file and a `.js` file carrying the same logic are both the
    // `javascript` language, so the same-language filter lets them cluster.
    let clone = expect_cluster_spanning(&report, &["BadgeList.jsx", "useBadge.js"])?;
    assert_eq!(cluster_bucket(clone), "identical");
    assert!(approx(signal(clone, "structural"), 1.0));
    assert!(approx(signal(clone, "token_jaccard"), 1.0));
    Ok(())
}

#[test]
fn ts_and_tsx_clone_does_not_cluster_by_default() -> Result<()> {
    let report = run_report(&fixture("ts-tsx-separate"), 8)?;
    // Guard against a vacuous pass: both files must actually be analysed, so
    // "no clusters" proves the language boundary held rather than a parser
    // silently dropping one of them.
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both the .ts and .tsx file must be analysed: {report:#}"
    );
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
    assert_eq!(cluster_bucket(typescript_pair), "identical");
    let react_pair = expect_cluster_spanning(&report, &["BadgeA.tsx", "BadgeB.tsx"])?;
    assert_eq!(cluster_bucket(react_pair), "identical");
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
