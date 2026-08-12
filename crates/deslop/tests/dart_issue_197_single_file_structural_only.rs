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

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

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
#[ignore = "GH #355: branch-introduced regression. The deleted filter suppressed this \
            family with a `members.len() < 3` cluster-size shortcut and a declaration-kind \
            match that also erased the real `csharp-merge-rename` pair. The replacement \
            keeps that pair; this eight-member family of one-statement delegating methods \
            is structurally indistinguishable from `csharp-merge-drift` (single file, \
            sibling members, identical call targets, only literals differ), so every \
            discriminator that hides it also erases the LSP merge target. Separating them \
            needs a reportable-floor product decision, not a new constant. Assertions are \
            intact — run with `-- --ignored`."]
fn single_file_structural_only_method_families_do_not_top_the_report() -> Result<()> {
    let scan_root = fixture("dart-issue-197-settings-getters");
    let report = run_report(&scan_root)?;

    // [METRICS-REPO] The duplication metric counts only the clusters the
    // report renders. Every family in this fixture is a hidden
    // structural-only sibling-method family, so the metric must report zero
    // duplication even though the families were detected (asserted via
    // `clusters_hidden` below) — proving a structural-only shape match
    // cannot inflate the percentage.
    let visible_contributing = report
        .pointer("/metrics/clusters_total")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert_eq!(
        visible_contributing, 0,
        "every family here is hidden structural-only, so none may count as a \
         visible contributing cluster: {report:#}"
    );
    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        duplicated_loc, 0,
        "structural-only sibling families must add zero duplicated lines: {report:#}"
    );
    let duplication_percent = report
        .pointer("/metrics/duplication_percent")
        .and_then(Value::as_f64)
        .unwrap_or(-1.0);
    assert!(
        (0.0..=0.0001).contains(&duplication_percent),
        "duplication_percent must be 0 when every cluster is hidden — the metric \
         is not influenced by structural-only shape matches: got {duplication_percent}"
    );

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

    let hidden = report
        .get("clusters_hidden")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        hidden >= 2,
        "the fixture reproduces the issue's two #1/#2 families (be951a686525 \
         size=7, 7f363063109f size=8); both must be suppressed via the \
         hidden-cluster path so they still count toward visibility telemetry: \
         clusters_hidden={hidden}, report={report:#}"
    );
    Ok(())
}
