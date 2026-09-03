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

use anyhow::Result;

use crate::common::{
    scan_dir::run_report_min_nodes,
    signals::{assert_no_pair_surface_on_cluster, assert_structural_only_contract},
    *,
};

const ANALYSED_FILES: u64 = 1;
const NO_VISIBLE_CLUSTERS: usize = 0;
/// The convicted components the settings region closes into: the
/// `resetX` wrappers, and the `getX`/`updateX` family beside them. Two
/// shape families, each proven scaffolding and each suppressed whole.
///
/// It was one while the `resetX` wrappers reached no candidate pair at
/// all — every identifier of a pair is byte-identical and only the route
/// literal moves, which [FUSED-CONTENT-GATE-PARAMETER] now reads as the
/// parameterisation it is ([REPAIR-RENAME-ANCHOR-MASS] then certifies
/// two whole authored declarations). The family is *found* where it was
/// previously invisible, and the acceptance below is unchanged by that:
/// no cluster is published, no line is counted, no percentage moves. An
/// exact count, not a floor — a third component, or either of these two
/// escaping suppression, still fails.
const CONVICTED_COMPONENTS: u64 = 2;
const NO_DUPLICATED_LINES: u64 = 0;
const NO_DUPLICATION_PERCENT: f64 = 0.0;

#[test]
fn single_file_structural_only_method_families_do_not_top_the_report() -> Result<()> {
    let scan_root = fixture("dart-issue-197-settings-getters");
    let report = run_report_min_nodes(&scan_root, "30")?;

    // [PIPELINE-CLUSTER-CLOSURE] The mass-only wire carries cluster facts
    // only; the `structural_only` bucket, the token floor and the row-4
    // signal triple are gone. The closure is one convicted component, so
    // its suppression contributes one hidden-cluster count and no output.
    for cluster in clusters(&report) {
        assert_no_pair_surface_on_cluster(cluster, "issue #197");
        assert_structural_only_contract(cluster, "issue #197");
    }
    // [METRICS-REPO] A convicted component must not contribute a visible
    // cluster, duplicated lines, or repository percentage.
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(ANALYSED_FILES)
    );
    assert_eq!(cluster_count(&report), NO_VISIBLE_CLUSTERS);
    assert_eq!(clusters_hidden(&report), CONVICTED_COMPONENTS);
    assert_eq!(
        metric_field(&report, "duplicated_loc").as_u64(),
        Some(NO_DUPLICATED_LINES)
    );
    assert_eq!(
        metric_field(&report, "duplication_percent").as_f64(),
        Some(NO_DUPLICATION_PERCENT)
    );

    Ok(())
}
