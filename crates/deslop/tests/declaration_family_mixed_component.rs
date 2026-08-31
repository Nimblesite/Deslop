//! [RANK-STRUCTURAL-ONLY] A mixed same-shape component must never be
//! hidden wholesale because one sibling differs.
//!
//! `csharp-mixed-declaration-component` is one class with three
//! same-skeleton methods: `AccrueDomestic` and `AccrueRegional` are a
//! real Type-2 clone pair (consistent 1:1 rename, literals agree), and
//! `AccrueExport` is the divergent sibling (different literals,
//! non-bijective identifier alignment). Cluster-wide content evidence
//! reads the component as "substance varies" on the strength of the
//! third member alone — exactly the evidence a cluster-wide
//! suppression would use to convict the first two.
//!
//! The declaration-family filter must not accept that conviction: every
//! window here covers exactly one declaration, and every body carries a
//! loop, an accumulator, a branch and arithmetic, so no member proves
//! the forwarding shape and the plurality proof fails on every path
//! ([RANK-STRUCTURAL-ONLY-FORWARDING]). The whole component stays visible
//! and renders the evidence of one admitted pair. The divergent closure
//! member must neither dilute that pair nor erase it.

use anyhow::Result;

use crate::common::{
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    verdict::*,
    *,
};

#[test]
fn a_divergent_sibling_does_not_erase_the_real_pair() -> Result<()> {
    let scan_root = fixture("csharp-mixed-declaration-component");
    let report = run_report(&scan_root, 20)?;

    let visible = clusters(&report);
    assert_eq!(
        visible.len(),
        1,
        "the three same-skeleton methods form one component and it must be \
         reported: hiding it erases the liftable AccrueDomestic/AccrueRegional \
         pair on evidence measured against AccrueExport. report={report:#}"
    );
    let cluster = visible
        .first()
        .ok_or_else(|| anyhow::anyhow!("the visible cluster asserted above is missing"))?;
    assert_eq!(
        cluster_size(cluster),
        3,
        "all three methods are occurrences of the surviving component: {cluster:#}"
    );
    assert_eq!(
        occurrence_files(cluster),
        vec![
            "BillingAccruals.cs",
            "BillingAccruals.cs",
            "BillingAccruals.cs"
        ],
        "the component is single-file by construction: {cluster:#}"
    );
    let texts = occurrence_texts(&scan_root, cluster)?;
    for method in ["AccrueDomestic", "AccrueRegional", "AccrueExport"] {
        assert!(
            texts.iter().any(|text| text.contains(method)),
            "{method} must be one of the reported occurrences — the real pair \
             survives and the divergent sibling is reported alongside it, not \
             silently dropped: {texts:#?}"
        );
    }

    // [PIPELINE-CLUSTER-CLOSURE] The verdict and the axes are pair-scoped
    // now. The acceptance on the wire: the three-body family is admitted,
    // mass-honest, clean-surfaced and byte-distinct (the divergent sibling
    // changes the bytes).
    assert_structural_only_contract(cluster, "mixed declaration component");
    assert_no_pair_surface_on_cluster(cluster, "mixed declaration component");
    assert!(
        !has_verbatim_pair(&scan_root, cluster)?,
        "the three bodies are a rename family and must slice to differing \
         bytes: {cluster:#}"
    );

    assert_eq!(
        clusters_hidden(&report),
        0,
        "nothing here proves a declaration family — no window covers two \
         siblings and every body is logic-bearing — so nothing may be hidden: \
         {report:#}"
    );
    assert_eq!(
        duplicated_loc(&report),
        39,
        "the visible component's lines are duplicated lines; a hidden component \
         would zero this metric and understate the file: {report:#}"
    );
    Ok(())
}
