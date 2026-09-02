//! E2E regression for GH #154.
//!
//! `top-offenders` was surfacing `structural_only` clusters
//! (`structural=1.00, token_jaccard=0.00, embedding_cos=0.00`) that
//! matched function signatures whose bodies were entirely unrelated.
//! The pattern is unavoidable in any codebase that uses shared-context
//! structs (`fn check_*(ctx: &mut Ctx)`), so refactoring the source to
//! silence deslop would hurt readability. The pipeline must drop these
//! evidence-free clusters from the ranked report.
//!
//! Acceptance: the rendered report MUST NOT contain any cluster with
//! `bucket="structural_only"` AND `token_jaccard < 0.1`. By construction
//! every `structural_only` cluster the renderer emits already satisfies
//! the second condition, so this assertion really demands that the
//! whole bucket be filtered from `clusters`.

use anyhow::Result;
use serde_json::Value;

use crate::common::scan_dir::run_report_min_nodes;
use crate::common::signals::{assert_no_pair_surface_on_cluster, assert_structural_only_contract};
use crate::common::*;

#[test]
fn structural_only_signature_clusters_are_dropped_from_the_report() -> Result<()> {
    let scan_root = fixture("rust-issue-154-structural-only");
    let report = run_report_min_nodes(&scan_root, "4")?;
    // [PIPELINE-CLUSTER-CLOSURE] The mass-only wire carries cluster facts
    // only; the `structural_only` bucket and its token floor are gone. The
    // acceptance holds on wire facts: every visible cluster must carry the
    // clean-surface contract (and with the signature-only families hidden,
    // nothing byte-distinct-and-unsupported may be labelled anything). The
    // hidden-cluster telemetry below pins the suppression.
    for cluster in clusters(&report) {
        assert_no_pair_surface_on_cluster(cluster, "issue #154");
        assert_structural_only_contract(cluster, "issue #154");
    }
    // [METRICS-REPO] This fixture mixes visible near-miss clones with hidden
    // structural_only families. The metric must count only the lines the
    // visible clusters cover, so the suppressed families add nothing.
    let reported = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("report missing metrics.duplicated_loc: {report:#}"))?;
    assert_eq!(
        reported,
        visible_duplicated_loc(&report),
        "duplicated_loc must equal the visible-cluster line union — hidden \
         structural_only families must not inflate the metric: {report:#}"
    );
    let hidden = report
        .get("clusters_hidden")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("report missing clusters_hidden: {report:#}"))?;
    assert!(
        hidden >= 1,
        "the fixture's structural_only clusters must be suppressed via \
         the hidden-cluster path so they still count toward visibility \
         telemetry: clusters_hidden={hidden}, report={report:#}"
    );
    Ok(())
}
