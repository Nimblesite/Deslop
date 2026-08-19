//! Clone-bucket boundary E2E tests for JavaScript and TypeScript
//! ([CLONE-TYPE-TAXONOMY], [CLONE-BUCKETS-ROUTING], [LANG-CAND-JAVASCRIPT],
//! [LANG-CAND-TYPESCRIPT]).
//!
//! These pin where the new languages land on the Type-1 / Type-2 / Type-3
//! axis. A byte-identical pair must reach the actionable `identical`
//! bucket. Renamed copies split on **measured content evidence** per
//! [FUSION-CONTENT-GATE]: a rename whose identifier mapping is
//! corroborated — Baker anchor mass, preserved literals plus explained
//! identifier positions ([TECH-PMATCH-BAKER],
//! `[REPAIR-RENAME-ANCHOR-MASS]`) — is promoted to the act-now
//! `nearly_identical` bucket, while a family whose collapsed leaves
//! genuinely disagree carries no such proof and stays conservatively
//! `structural_only` (#134, the `js-classes` delegating-method family in
//! `js_language_features.rs`).
//!
//! Anchor *scarcity* is not the discriminator and never was: the
//! superseded four-literal cliff zeroed rename evidence outright and
//! demoted these renamed-loop fixtures, which is a false negative by the
//! contract `type2_rename_anchor_floor.rs` states. Every promoted case
//! here is held to the shared `assert_proven_rename_contract`, so the two
//! suites cannot drift back into asserting opposite verdicts about the
//! same signal triple.

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::{signals::assert_proven_rename_contract, *};

/// Drives `deslop` over `fixture_dir` and holds the renamed-loop clone
/// spanning `files` to the proven-rename contract plus the pair size the
/// fixture was built with — the JavaScript and TypeScript grammars are
/// separate but share one normalisation, so both sides must reach the
/// same verdict.
fn assert_renamed_loop_pair(fixture_dir: &str, min_nodes: u32, files: &[&str]) -> Result<()> {
    let root = fixture(fixture_dir);
    let report = run_report(&root, min_nodes)?;
    let clone = expect_cluster_spanning(&report, files)?;
    assert_proven_rename_contract(&root, clone, fixture_dir)?;
    assert_eq!(
        cluster_size(clone),
        2,
        "the renamed loop has exactly two occurrences: {report:#}"
    );
    Ok(())
}

/// Asserts `unrelated` joins no cluster at all: a near-miss family that
/// swallows a file sharing nothing with it is a false positive.
fn assert_never_clustered(report: &Value, unrelated: &str, why: &str) {
    assert!(
        clusters(report)
            .iter()
            .all(|cluster| !cluster_file_set(cluster).contains(unrelated)),
        "{why}: {report:#}"
    );
}

#[test]
fn javascript_byte_identical_pair_is_identical_bucket() -> Result<()> {
    assert_bucketed_clone(
        "js-type1-identical",
        10,
        &["tax_alpha.js", "tax_beta.js"],
        "identical",
    )
}

#[test]
fn typescript_byte_identical_pair_is_identical_bucket() -> Result<()> {
    assert_bucketed_clone(
        "ts-type1-identical",
        12,
        &["tax_alpha.ts", "tax_beta.ts"],
        "identical",
    )
}

#[test]
fn javascript_renamed_loop_clone_is_a_proven_rename() -> Result<()> {
    // The same loop-with-guards routine on both sides, every identifier
    // renamed (`lineItems`→`stockRows`, `taxRate`→`shrinkageRate`,
    // `amount`→`value`, and eight more). Eleven substitutions corroborate
    // each other across the pair and three literals survive in position,
    // so the identifier mapping is proven and the pair is duplication a
    // developer must act on — not shape-only evidence.
    assert_renamed_loop_pair("js-type2-loop", 10, &["inventory_gamma.js", "tax_alpha.js"])
}

#[test]
fn typescript_renamed_loop_clone_is_a_proven_rename() -> Result<()> {
    // The TypeScript side of the same rename, annotations included. The
    // two grammars are separate but share one normalisation, so the
    // verdict must not depend on which one parsed the clone.
    assert_renamed_loop_pair("ts-type2-loop", 12, &["inventory_gamma.ts", "tax_alpha.ts"])
}

#[test]
fn javascript_renamed_map_reduce_arrow_is_nearly_identical() -> Result<()> {
    // A deeply-nested map/reduce/arrow pipeline, maximally renamed
    // (invoice→order, rate→price, hours→quantity, deduction→discount) with
    // all five numeric literals preserved in position. The anchors prove
    // the bijective identifier mapping, so [FUSION-CONTENT-GATE] promotes
    // the pair to the act-now `nearly_identical` bucket, and the
    // shape-identical Merkle match corrects the placeholder-dominated token
    // fallback to its true value of 1.0 (#232).
    assert_bucketed_clone(
        "js-type2-pipeline",
        8,
        &["invoices.js", "orders.js"],
        "nearly_identical",
    )
}

#[test]
fn javascript_near_miss_extra_guard_is_a_proven_rename() -> Result<()> {
    // A four-way rename (`records`→`entries`, `flagged`→`alerts`,
    // `record`→`entry`, `level`→`available`) carrying one extra
    // `continue` guard on one side — a Type-3 near miss on top of a
    // Type-2 rename. Six property names survive the rename (`onHand`,
    // `reserved`, `reorderPoint`, `sku`, `deficit`, `push`), so the
    // mapping is corroborated and the extra guard costs confidence
    // without costing the verdict.
    let root = fixture("js-type3-guard");
    let report = run_report(&root, 10)?;
    let clone = expect_cluster_spanning(&report, &["inventoryScan.js", "stockScan.js"])?;
    assert_proven_rename_contract(&root, clone, "js-type3-guard")?;
    assert_never_clustered(
        &report,
        "formatLabel.js",
        "the unrelated label formatter must never join the near-miss cluster",
    );
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
    assert_never_clustered(
        &report,
        "randomToken.js",
        "an unrelated function must not join the near-miss cluster",
    );
    Ok(())
}

#[test]
fn typescript_near_miss_reordered_statements_cluster_nearly_identical() -> Result<()> {
    // Two normalizers, byte-identical except two independent statements
    // swapped and the function renamed — the canonical Type-3 near miss.
    // Positional content agreement stays high across the swap, so
    // [FUSION-CONTENT-GATE] keeps the pair in the act-now
    // `nearly_identical` bucket instead of demoting identical-content,
    // reordered code to "same shape, different content".
    assert_bucketed_clone(
        "ts-type3-reorder",
        10,
        &["normalizeContact.ts", "normalizeUser.ts"],
        "nearly_identical",
    )
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
