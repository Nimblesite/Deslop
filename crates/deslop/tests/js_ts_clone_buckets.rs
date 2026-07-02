//! Clone-bucket boundary E2E tests for JavaScript and TypeScript
//! ([CLONE-TYPE-TAXONOMY], [CLONE-BUCKETS-ROUTING], [LANG-CAND-JAVASCRIPT],
//! [LANG-CAND-TYPESCRIPT]).
//!
//! These pin where the new languages land on the Type-1 / Type-2 / Type-3
//! axis. A byte-identical pair must reach the actionable `identical`
//! bucket; a renamed copy that the token layer does not independently
//! confirm routes to the demoted `structural_only` bucket by design (#134),
//! and near-miss edits surface either a token-supported `nearly_identical`
//! cluster or a `structural_only` shared subtree. Every bucket here is the
//! real value the engine produced for the fixture.

use anyhow::Result;

mod common;
use crate::common::*;

#[test]
fn javascript_byte_identical_pair_is_identical_bucket() -> Result<()> {
    let report = run_report(&fixture("js-type1-identical"), 10)?;
    let clone = expect_cluster_spanning(&report, &["tax_alpha.js", "tax_beta.js"])?;
    assert_eq!(cluster_bucket(clone), "identical");
    assert!(approx(signal(clone, "structural"), 1.0));
    assert!(approx(signal(clone, "token_jaccard"), 1.0));
    Ok(())
}

#[test]
fn typescript_byte_identical_pair_is_identical_bucket() -> Result<()> {
    let report = run_report(&fixture("ts-type1-identical"), 12)?;
    let clone = expect_cluster_spanning(&report, &["tax_alpha.ts", "tax_beta.ts"])?;
    assert_eq!(cluster_bucket(clone), "identical");
    assert!(approx(signal(clone, "structural"), 1.0));
    assert!(approx(signal(clone, "token_jaccard"), 1.0));
    Ok(())
}

#[test]
fn javascript_renamed_loop_clone_routes_to_structural_only() -> Result<()> {
    let report = run_report(&fixture("js-type2-loop"), 10)?;
    // Same loop-with-guards routine, every identifier renamed. It is a real
    // Type-2 clone (structural==1.0, cross-file) but, with no independent
    // token confirmation, it lands in the demoted `structural_only` bucket
    // rather than `identical` — the conservative routing from #134.
    let clone = expect_cluster_spanning(&report, &["inventory_gamma.js", "tax_alpha.js"])?;
    assert_eq!(cluster_bucket(clone), "structural_only");
    assert!(approx(signal(clone, "structural"), 1.0));
    assert!(signal(clone, "token_jaccard") < 0.05);
    Ok(())
}

#[test]
fn typescript_renamed_loop_clone_routes_to_structural_only() -> Result<()> {
    let report = run_report(&fixture("ts-type2-loop"), 12)?;
    let clone = expect_cluster_spanning(&report, &["inventory_gamma.ts", "tax_alpha.ts"])?;
    assert_eq!(cluster_bucket(clone), "structural_only");
    assert!(approx(signal(clone, "structural"), 1.0));
    assert!(signal(clone, "token_jaccard") < 0.05);
    Ok(())
}

#[test]
fn javascript_renamed_map_reduce_arrow_is_structural_only() -> Result<()> {
    let report = run_report(&fixture("js-structural-only"), 8)?;
    // A deeply-nested map/reduce/arrow pipeline, renamed: the structural
    // Merkle layer matches but the token signature is dominated by
    // placeholders, so it routes to `structural_only` (#134).
    let clone = expect_cluster_spanning(&report, &["invoices.js", "orders.js"])?;
    assert_eq!(cluster_bucket(clone), "structural_only");
    assert!(approx(signal(clone, "structural"), 1.0));
    assert!(signal(clone, "token_jaccard") < 0.05);
    Ok(())
}

#[test]
fn javascript_near_miss_extra_guard_clusters_structural_only() -> Result<()> {
    let report = run_report(&fixture("js-type3-guard"), 10)?;
    let clone = expect_cluster_spanning(&report, &["inventoryScan.js", "stockScan.js"])?;
    assert_eq!(cluster_bucket(clone), "structural_only");
    assert!(approx(signal(clone, "structural"), 1.0));
    Ok(())
}

#[test]
fn javascript_near_miss_extra_statement_keeps_shared_subtree_and_excludes_unrelated() -> Result<()>
{
    let report = run_report(&fixture("js-type3-stmt"), 10)?;
    // The two URL-decode loops share an inner subtree that still clusters
    // even though one copy writes two extra map entries.
    let clone = expect_cluster_spanning(&report, &["parseHeaders.js", "parseQuery.js"])?;
    assert!(approx(signal(clone, "structural"), 1.0));
    // The unrelated random-token generator in the same directory must never
    // be pulled into a clone cluster.
    assert!(
        clusters(&report)
            .iter()
            .all(|cluster| !cluster_file_set(cluster).contains("randomToken.js")),
        "an unrelated function must not join the near-miss cluster: {report:#}"
    );
    Ok(())
}

#[test]
fn typescript_near_miss_reordered_statements_cluster_structural_only() -> Result<()> {
    let report = run_report(&fixture("ts-type3-reorder"), 10)?;
    let clone = expect_cluster_spanning(&report, &["normalizeContact.ts", "normalizeUser.ts"])?;
    assert_eq!(cluster_bucket(clone), "structural_only");
    assert!(approx(signal(clone, "structural"), 1.0));
    Ok(())
}

#[test]
fn typescript_signature_anchored_near_miss_is_conservatively_suppressed() -> Result<()> {
    // `tallyPoints` and `tallyScores` share an identical typed signature
    // (`(rounds: number[][]): number`) but their bodies diverge by an extra
    // statement, so the only *exact* structural anchor is the signature. The
    // signature-only filter (#154) suppresses this as scaffolding rather than
    // letting an unrelated-body coincidence reach top-offenders — the same
    // conservative routing the other languages get. (Body-anchored near
    // misses still cluster: see the `*_near_miss_*` tests above and the
    // `typescript-type3` fixture.)
    let report = run_report(&fixture("ts-type3-stmt"), 10)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(3),
        "all three files must be analysed: {report:#}"
    );
    assert!(
        cluster_spanning(&report, &["pointBoard.ts", "scoreBoard.ts"]).is_none(),
        "a signature-anchored near miss must not surface as a visible clone: {report:#}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the signature-only family must be detected and hidden: {report:#}"
    );
    Ok(())
}
