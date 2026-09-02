//! [FUSED-CONTENT-GATE] Pair-only content admission must retain a
//! supported Type-2 pair while excluding a same-shape rewrite.
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
//! The divergent method lacks pair-content support, so it must be rejected
//! before closure. The two supported methods still form the visible
//! component; no cluster evidence is needed to decide either result.

use anyhow::Result;

use crate::common::{
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    verdict::*,
    *,
};

const DOMESTIC_SPAN: (u64, u64) = (18, 30);
const REGIONAL_SPAN: (u64, u64) = (32, 44);
const EXPECTED_DUPLICATED_LOC: u64 = 26;

#[test]
fn a_divergent_same_shape_sibling_does_not_join_the_real_pair() -> Result<()> {
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
        2,
        "only the raw-content-supported methods may enter closure: {cluster:#}"
    );
    assert_eq!(
        occurrence_files(cluster),
        vec!["BillingAccruals.cs", "BillingAccruals.cs"],
        "the component is single-file by construction: {cluster:#}"
    );
    let spans: Vec<(u64, u64)> = occurrences(cluster)
        .iter()
        .map(|occurrence| {
            (
                field(occurrence, "start_line").as_u64().unwrap_or_default(),
                field(occurrence, "end_line").as_u64().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        spans,
        [DOMESTIC_SPAN, REGIONAL_SPAN],
        "the Domestic and Regional methods survive; the Export rewrite may not: {cluster:#}"
    );

    // [PIPELINE-CLUSTER-CLOSURE] The admitted pair renders as a
    // mass-honest, clean-surfaced, byte-distinct cluster.
    assert_structural_only_contract(cluster, "mixed declaration component");
    assert_no_pair_surface_on_cluster(cluster, "mixed declaration component");
    assert!(
        !has_verbatim_pair(&scan_root, cluster)?,
        "the two bodies are a rename family and must slice to differing \
         bytes: {cluster:#}"
    );

    assert_eq!(
        clusters_hidden(&report),
        0,
        "the supported pair is visible and the divergent method is rejected \
         before closure, so nothing may be hidden: \
         {report:#}"
    );
    assert_eq!(
        duplicated_loc(&report),
        EXPECTED_DUPLICATED_LOC,
        "the visible component's lines are duplicated lines; a hidden component \
         would zero this metric and understate the file: {report:#}"
    );
    Ok(())
}
