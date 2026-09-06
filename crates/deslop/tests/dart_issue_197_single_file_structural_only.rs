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

mod common;
use crate::common::{verdict::*, *};

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

    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offenders: Vec<String> = clusters
        .iter()
        .filter(|cluster| cluster_bucket(cluster) == "structural_only")
        .filter(|cluster| signal(cluster, "token_jaccard") < 0.1)
        .filter(|cluster| cluster_size(cluster) >= 3)
        .map(|cluster| {
            format!(
                "cluster {id} size={size} weight={weight:.0} signals={{structural={s:.2}, \
                 token_jaccard={t:.2}, embedding_cos={e:.2}}}",
                id = cluster.get("id").and_then(Value::as_str).unwrap_or("?"),
                size = cluster_size(cluster),
                weight = cluster.get("weight").and_then(Value::as_f64).unwrap_or(0.0),
                s = signal(cluster, "structural"),
                t = signal(cluster, "token_jaccard"),
                e = signal(cluster, "embedding_cos"),
            )
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "issue #197: a single-file structural_only sibling-method family \
         (token_jaccard < 0.1, size >= 3) has no real evidence and must not \
         surface in the ranked report regardless of file spread. Offending \
         clusters: {offenders:#?}"
    );
    Ok(())
}
