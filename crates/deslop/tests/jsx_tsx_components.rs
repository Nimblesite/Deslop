//! JSX / TSX component E2E tests ([LANG-CAND-JAVASCRIPT],
//! [LANG-CAND-TYPESCRIPT], [PIPELINE-NORMALIZE-AST]).
//!
//! React-style components exercise the JSX paths of both grammars: the TSX
//! grammar (`.tsx`) and the JSX path of the plain JavaScript grammar
//! (`.jsx`). Renamed components must still cluster, JSX text and HTML
//! entities must collapse to literals so entity-vs-text differences do not
//! leak into the clone shape, and two genuinely different components must
//! not be merged.

use anyhow::Result;

use crate::common::*;

#[test]
fn tsx_renamed_components_cluster_nearly_identical() -> Result<()> {
    // Two `.tsx` components with identical JSX structure (typed props,
    // conditional className, list rendering) but renamed throughout.
    assert_bucketed_clone(
        "jsx-tsx-components",
        10,
        &["TeamPanel.tsx", "UserPanel.tsx"],
        "nearly_identical",
    )
}

#[test]
fn js_and_jsx_renamed_components_cluster_across_extensions() -> Result<()> {
    // A `.js` component and a `.jsx` component with the same card markup,
    // renamed: the plain-JS grammar parses JSX, and both are the
    // `javascript` language, so they cluster. The five identical CSS
    // class-name literals anchor the bijective prop rename, so
    // [FUSED-CONTENT-GATE] routes the pair act-now `nearly_identical`.
    assert_bucketed_clone(
        "jsx-js-components",
        10,
        &["OfferTile.js", "ProductCard.jsx"],
        "nearly_identical",
    )
}

#[test]
fn jsx_html_entity_and_plain_text_collapse_to_the_same_clone() -> Result<()> {
    // `<b>&amp;</b>` and `<b>plus</b>` differ only in that one slot is an
    // `html_character_reference` and the other is `jsx_text`. Both collapse
    // to the literal placeholder, so the two renamed components reach an
    // identical structure and cluster — the entity does not leak into the
    // fingerprint.
    assert_bucketed_clone(
        "jsx-entity-invariance",
        8,
        &["Ampersand.jsx", "Plus.jsx"],
        "nearly_identical",
    )
}

#[test]
fn two_unrelated_jsx_components_do_not_cluster() -> Result<()> {
    let report = run_report(&fixture("jsx-unrelated-components"), 10)?;
    // Guard against a vacuous pass: prove both `.jsx` files were parsed and
    // analysed, so "no clusters" reflects real non-duplication rather than a
    // silently-broken JSX parser producing zero fingerprints.
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both unrelated components must be analysed: {report:#}"
    );
    assert!(
        clusters(&report).is_empty(),
        "structurally different components must not be reported as a clone: {report:#}"
    );
    Ok(())
}
