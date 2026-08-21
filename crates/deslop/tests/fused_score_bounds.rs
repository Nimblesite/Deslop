//! End-to-end regression coverage for issue #3 and #415
//! [FUSION-STRATEGY-BOUNDED-MAX]: rendered fused scores must stay in
//! the public confidence range `[0, 1]`, and the guard itself must be
//! fail-closed. The previous guard returned `None` when the `clusters`
//! array was absent and `?`-exited on the first cluster missing a
//! `signals`/`fused` field, so an empty report — or a renamed field —
//! passed the spec's only bound check without inspecting anything.

use anyhow::anyhow;

mod common;
use crate::common::*;

/// Every cluster's `(id, signals.fused)`, erroring on a missing
/// `clusters` array or a missing/non-numeric `fused` so schema drift
/// fails the bound check instead of skipping it.
fn rendered_fused_values(report: &serde_json::Value) -> Result<Vec<(String, f64)>> {
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("report renders no clusters array: {report:#}"))?;
    clusters
        .iter()
        .map(|cluster| {
            let id = cluster_id(cluster).to_owned();
            let fused = cluster
                .pointer("/signals/fused")
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    anyhow!("cluster {id} renders no numeric signals.fused: {cluster:#}")
                })?;
            Ok((id, fused))
        })
        .collect()
}

// Implements [FUSION-STRATEGY-BOUNDED-MAX]: component scores are public
// confidence signals in [0, 1], and the fused confidence reported to
// agents must be bounded to the same range — proven over a corpus that
// is required to produce clusters, so the assertion can never pass by
// inspecting nothing (gh #415).
#[test]
fn rendered_fused_scores_are_bounded_to_unit_interval() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = deslop_cmd(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let report = load_json(&tmp.path().join("report.json"))?;
    let fused_values = rendered_fused_values(&report)?;
    assert!(
        !fused_values.is_empty(),
        "csharp-small must render at least one cluster — an empty report \
         proves nothing about the fused bound: {report:#}"
    );
    for (id, fused) in &fused_values {
        assert!(
            (0.0..=1.0).contains(fused),
            "cluster {id} reports fused={fused}, outside [0, 1]: {report:#}"
        );
    }
    Ok(())
}
