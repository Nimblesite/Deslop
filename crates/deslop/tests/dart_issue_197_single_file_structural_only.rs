//! E2E regression for GH #197 (showstopper).
//!
//! `top-offenders` surfaced two single-file `structural_only` clusters
//! (`structural=1.00, token_jaccard=0.00, embedding_cos=0.00`) as the #1/#2
//! offenders on the official `meilisearch/meilisearch-dart` client: the
//! index-settings get/reset/update method family. Each method shares a
//! skeleton but targets a different endpoint literal and return type, so it
//! is the public REST API surface, not extract-worthy duplication.
//!
//! #134/#154 demoted *cross-file* structural-only scaffolding, but the floor
//! `files.len() >= 3` let an in-class sibling-method family — which lives in
//! one file — keep full `NearlyIdentical` weight and dominate the ranking.
//!
//! Acceptance: no `structural_only` cluster with `token_jaccard < 0.1` and
//! `size >= 3` may appear in the ranked report, and the fixture's families
//! must instead be counted in `clusters_hidden`. The fixture vendors the real
//! meilisearch-dart settings region, so the cluster ids `be951a686525` and
//! `7f363063109f` from the issue reproduce verbatim.
//!
//! Suppression is earned by [RANK-STRUCTURAL-ONLY-FORWARDING], not by any
//! count. These wrappers are one statement each, so every fingerprint window
//! covers exactly one declaration and a plurality-only test can never reach
//! them; what convicts them is that each body makes one client call and
//! returns it, with nothing computed on the way through. The same proof
//! acquits `csharp-merge-drift` and `csharp-merge-rename`, whose bodies bind
//! locals, loop, and branch — see the `code_action` and `refactor_merge`
//! suites, which must stay green alongside this one.

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::common::{
    signals::{assert_no_pair_surface_on_cluster, assert_structural_only_contract},
    verdict::*,
    *,
};

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(scan_root, &output)?
        .args(["--min-nodes", "30", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

#[test]
fn single_file_structural_only_method_families_do_not_top_the_report() -> Result<()> {
    let scan_root = fixture("dart-issue-197-settings-getters");
    let report = run_report(&scan_root)?;

    // [PIPELINE-CLUSTER-CLOSURE] The mass-only wire carries cluster facts
    // only; the `structural_only` bucket, the token floor and the row-4
    // signal triple are gone. The acceptance holds on wire facts: nothing
    // renders but the clean-surface contract, and the two families must be
    // counted in `clusters_hidden` (asserted below via
    // `assert_fully_suppressed`) — that telemetry is what proves the
    // REST-settings surface stayed off the ranked report.
    for cluster in clusters(&report) {
        assert_no_pair_surface_on_cluster(cluster, "issue #197");
        assert_structural_only_contract(cluster, "issue #197");
    }
    // [METRICS-REPO] The duplication metric counts only the clusters the
    // report renders. Every family in this fixture is a hidden
    // structural-only sibling-method family, so the metric must report zero
    // duplication even though the families were detected (asserted via
    // `clusters_hidden` below) — proving a structural-only shape match
    // cannot inflate the percentage.
    // The fixture reproduces the issue's two #1/#2 families (be951a686525
    // size=7, 7f363063109f size=8); both must be suppressed via the
    // hidden-cluster path so they still count toward visibility telemetry.
    assert_fully_suppressed(&report, 2);

    Ok(())
}
