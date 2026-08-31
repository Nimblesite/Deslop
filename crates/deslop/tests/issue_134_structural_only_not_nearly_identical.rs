//! Issue #134: a same-shape family whose members differ in *substance*
//! must NOT be labeled `nearly_identical`. The fixture is three
//! handlers sharing one 96-node skeleton whose renamed identifiers map
//! consistently but whose loop strides (`+ 1` / `+ 2` / `+ 3`) diverge
//! at the aligned literal position: [FUSED-CONTENT-GATE] measures zero
//! literal preservation, so no content evidence vouches for the family
//! and it stays a hidden structural-only match (test scaffolding,
//! generated boilerplate). The divergent literal is what separates this
//! family from a genuine Type-2 clone — an identical-logic rename with
//! its literals preserved is the *reportable* side of the same line
//! (`fused_golden_bands.rs`, `type2_rename_anchor_floor.rs`,
//! [TECH-PMATCH-BAKER]).
//!
//! Acceptance: no cluster in the rendered report carries
//! `bucket=nearly_identical` together with `structural >= 0.99`,
//! `token_jaccard < 0.05`, and `embedding_cos < 0.05`.

use anyhow::Result;

use crate::common::*;

#[test]
fn issue_134_structural_only_clusters_are_not_nearly_identical() -> Result<()> {
    let scan_root = fixture("csharp-issue-134-structural-only");
    let report = run_report(&scan_root, 30)?;
    // [METRICS-REPO] `clusters_total` counts only the clusters the report
    // renders. This fixture's sole family is a hidden structural-only
    // cluster, so it contributes zero to the metric while still being
    // detected (`clusters_hidden`) — a structural-only shape match cannot
    // inflate the percentage.
    let visible_contributing = report
        .pointer("/metrics/clusters_total")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        visible_contributing, 0,
        "hidden structural-only cluster must not count as a visible \
         contributing cluster: {report}"
    );
    let clusters_hidden = report
        .get("clusters_hidden")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    assert!(
        clusters_hidden >= 1,
        "fixture must produce at least one hidden structural-only cluster so \
         the bucketing rule is actually exercised: {report}"
    );
    let duplication_percent = report
        .pointer("/metrics/duplication_percent")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(-1.0);
    assert!(
        (0.0..=0.0001).contains(&duplication_percent),
        "structural-only shape matches must not influence duplication_percent: \
         got {duplication_percent}"
    );
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "nearly_identical")
        .filter(|cluster| {
            signal(cluster, "structural") >= 0.99
                && signal(cluster, "token_jaccard") < 0.05
                && signal(cluster, "embedding_cos") < 0.05
        })
        .map(|cluster| {
            format!(
                "cluster {} signals={{structural={:.2}, token_jaccard={:.2}, \
                 embedding_cos={:.2}}}",
                cluster_id(cluster),
                signal(cluster, "structural"),
                signal(cluster, "token_jaccard"),
                signal(cluster, "embedding_cos"),
            )
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "issue #134: structural-only clusters (token_jaccard < 0.05 and \
         embedding_cos < 0.05) must not be labeled `nearly_identical`. \
         Offending clusters: {offenders:#?}"
    );
    Ok(())
}
