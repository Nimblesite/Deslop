//! [FUSED-CONTENT-GATE] — the gate may correct a signal it can prove,
//! and may not publish one it did not measure (gh #431).
//!
//! The mass-only wire cutover removed the cluster `signals` block, the
//! routing bucket, and the `evidence_verdict` sentence: admission
//! evidence is pair-scoped and cluster surfaces carry cluster facts and
//! mass only ([PIPELINE-CLUSTER-CLOSURE]). The fabrication this suite
//! exists to catch — `content_gated_signals` rewriting `token_jaccard`
//! to `1.0` for a `nearly_identical` cluster on a Merkle argument that
//! held at digest equality and nowhere else — is therefore impossible
//! by construction on the current wire. The suite pins the migration
//! instead, at the same strength:
//!
//! 1. **Recall** — every pair this fixture exists for is still
//!    **admitted** and reported, with the same rank ordering and the
//!    wire mass formula `canonical_node_count × (occurrence_count − 1)`
//!    ([RANK-MASS-SUM]). A cutover that silently dropped the
//!    near-saturated Type-3 pair, the digest-equal control, or the
//!    accessor pair is caught exactly as the old signal assertions
//!    would have caught a silently-demoted cluster.
//! 2. **No fabricated evidence surface** — no cluster on the wire may
//!    carry `signals`, `bucket`, `category`, `evidence_verdict`, or any
//!    other pair-only or presentation field. Reintroducing a cluster
//!    signal that a gate could fabricate fails these negative pins the
//!    moment it lands.
//!
//! `ledger_alpha.py` / `ledger_beta.py` are a long Type-3 pair whose one
//! control-flow node changes from `if` to `while`, keeping their
//! measured structural overlap inside `[0.99, 1.0)` — the band the old
//! routing tolerance would have sat through. The byte-identical control
//! in `content-gate-unsaturated` proves the digest-equal population
//! separately.

use serde_json::Value;

use crate::common::{signals::*, *};

/// Node floor for the small control and accessor fixtures.
const MIN_NODES: u32 = 8;

/// Node floor that admits the 307-node Type-3 function roots while excluding
/// exact repeated statement windows, so the assertion cannot select a nested
/// digest-equal pair instead of the near-saturated pair under test.
const SATURATION_BAND_MIN_NODES: u32 = 200;

/// The pair that lands inside the `[0.99, 1.0)` band: one control-flow
/// node apart, so their normalised subtrees differ and no digest is shared.
const SATURATION_BAND_PAIR: [&str; 2] = ["ledger_alpha.py", "ledger_beta.py"];

/// The byte-identical control in the same fixture: both axes saturate,
/// the gate runs, and `pair_agreement = 1.00` genuinely corroborates.
const SATURATED_CONTROL_PAIR: [&str; 2] = ["control_alpha.rs", "control_beta.rs"];

/// The pair that reproduces gh #460: two unrelated tree-sitter field
/// accessors — different node kind, different field, different body —
/// whose only shared authored logic is the grammar-mandated accessor
/// idiom. Their shared-subtree shape does not saturate, so the content
/// gate measures its observations but does not use them for routing.
const ACCESSOR_PAIR: [&str; 2] = ["accessor_argument.rs", "accessor_assignment.rs"];

/// Renders the deliberately near-saturated Type-3 fixture.
fn render_saturation_band() -> Result<Value> {
    run_report(
        &fixture("content-gate-saturation-band"),
        SATURATION_BAND_MIN_NODES,
    )
}

/// Renders the unsaturated-gate fixture once per assertion below. Both
/// of its pairs are single function bodies, so they cluster at the same
/// [`MIN_NODES`] floor the ledger corpus uses.
fn render_unsaturated() -> Result<Value> {
    run_report(&fixture("content-gate-unsaturated"), MIN_NODES)
}

/// Asserts a cluster carries none of the pair-only or presentation
/// fields the mass-only wire forbids — the surface the old
/// `content_gated_signals` rewriting published through
/// ([PIPELINE-CLUSTER-CLOSURE]). A cluster that grows a `signals`
/// block, a bucket, an evidence verdict, or a weight is the fabrication
/// path reopening.
fn assert_no_pair_surface_on_cluster(cluster: &Value, label: &str) {
    for field in [
        "signals",
        "signal_source",
        "content",
        "evidence_verdict",
        "bucket",
        "category",
        "classification",
        "weight",
        "size",
        "summary",
        "interpretation",
        "language",
    ] {
        assert!(
            cluster.get(field).is_none(),
            "{label}: the mass-only wire forbids {field} on a cluster — a gate \
             could fabricate a value through it again: {cluster:#}"
        );
    }
}

/// Asserts the wire-visible admission contract for a reported pair:
/// it exists on the report, every occurrence is visible, and its mass
/// is the wire formula `canonical_node_count × (occurrence_count − 1)`
/// ([RANK-MASS-SUM]). The positive, human-readable half of every
/// guarantee this suite makes; the negative half is
/// [`assert_no_pair_surface_on_cluster`].
fn assert_admitted_pair(cluster: &Value, label: &str) {
    let canonical_nodes = field(cluster, "canonical_node_count").as_u64().unwrap_or(0);
    let occurrence_count = field(cluster, "occurrence_count").as_u64().unwrap_or(0);
    let mass = field(cluster, "mass").as_u64().unwrap_or(0);
    assert!(
        canonical_nodes > 0 && occurrence_count >= 2,
        "{label}: an admitted cluster must carry canonical_node_count and \
         occurrence_count — {dump}",
        dump = signal_dump(cluster)
    );
    assert_eq!(
        mass,
        canonical_nodes.saturating_mul(occurrence_count.saturating_sub(1)),
        "{label}: mass must be canonical_node_count × (occurrence_count − 1) — {dump}",
        dump = signal_dump(cluster)
    );
    assert!(
        !occurrences(cluster).iter().any(occurrence_is_hidden),
        "{label}: a reported pair may not hide an occurrence behind report_hide \
         — {dump}",
        dump = signal_dump(cluster)
    );
}

// The pair this fixture exists for: a Type-3 near-miss inside the
// [0.99, 1.0) band. The old defect published it carrying a fabricated
// `token_jaccard = 1.00` and a `shape = 1.00` derived from it, because
// the routing tolerance `STRUCTURAL_SATURATION_FLOOR` (0.99) let the
// gate correct signals it had not measured. With the cluster signals
// block gone, the fabrication surface is gone; what the test pins is
// that the band pair still admits, reports with the exact wire mass,
// and no pair-only field has crept back onto the cluster.
#[test]
fn the_content_gate_publishes_no_token_jaccard_it_did_not_measure() -> Result<()> {
    let report = render_saturation_band()?;
    let ledger = clusters(&report)
        .iter()
        .find(|cluster| {
            SATURATION_BAND_PAIR
                .iter()
                .all(|file| cluster_file_set(cluster).contains(*file))
        })
        .ok_or_else(|| {
            anyhow::anyhow!("expected the ledger pair admitted on the report: {report:#}")
        })?;
    assert_admitted_pair(ledger, "ledger pair");
    assert_no_pair_surface_on_cluster(ledger, "ledger pair");
    assert!(
        !clusters(&report)
            .iter()
            .any(|cluster| cluster.get("signals").is_some()),
        "no cluster on the report may carry a signals block — the fabrication \
         path for the gh #431 defect is a cluster signal: {report:#}"
    );
    Ok(())
}

// The other half of the contract: the digest-equal population the old
// correction served must keep admitting. A byte-identical control still
// reports, ranked ahead of every approximate pair in the same fixture,
// with the wire mass formula.
#[test]
fn a_digest_equal_pair_keeps_its_saturated_signals() -> Result<()> {
    let report = render_unsaturated()?;
    let control = expect_cluster_spanning(&report, &SATURATED_CONTROL_PAIR)?;
    assert_admitted_pair(control, "byte-identical control");
    assert_no_pair_surface_on_cluster(control, "byte-identical control");
    assert_eq!(
        field(control, "rank").as_u64().unwrap_or(0),
        1,
        "the byte-identical control must lead the fixture's report: {report:#}"
    );
    Ok(())
}

// [FUSED-CONTENT-GATE] gh #460 — the report may not tell a reader that
// content evidence corroborated a match whose evidence did not
// corroborate it. On the mass-only wire the corroboration sentence is
// gone with the `evidence_verdict` field; what survives is the
// admission ordering: the gate-skipped accessor pair still reports
// (recall), ranked below the corroborated control, and no pair-only
// surface claims anything about its content.
#[test]
fn a_cluster_whose_evidence_did_not_corroborate_is_not_told_it_agreed() -> Result<()> {
    let report = render_unsaturated()?;
    let accessor = expect_cluster_spanning(&report, &ACCESSOR_PAIR)?;
    assert_admitted_pair(accessor, "accessor pair");
    assert_no_pair_surface_on_cluster(accessor, "accessor pair");
    let control_rank = field(expect_cluster_spanning(&report, &SATURATED_CONTROL_PAIR)?, "rank")
        .as_u64()
        .unwrap_or(0);
    let accessor_rank = field(accessor, "rank").as_u64().unwrap_or(0);
    assert!(
        accessor_rank > control_rank,
        "the below-saturation accessor pair must rank after the corroborated \
         control (control={control_rank}, accessor={accessor_rank}): {report:#}"
    );
    Ok(())
}

// The other half of the contract, asserted in the same run: the
// corroborated control keeps leading the report, the gate-skipped
// accessor pair still reports behind it, and neither carries a
// pair-only surface — one cluster can no longer be told a sentence the
// other is not, because no cluster is told any sentence at all.
#[test]
fn a_gated_cluster_still_reports_the_evidence_that_corroborated_it() -> Result<()> {
    let report = render_unsaturated()?;
    let control = expect_cluster_spanning(&report, &SATURATED_CONTROL_PAIR)?;
    let accessor = expect_cluster_spanning(&report, &ACCESSOR_PAIR)?;
    assert_admitted_pair(control, "byte-identical control");
    assert_admitted_pair(accessor, "accessor pair");
    assert_no_pair_surface_on_cluster(control, "byte-identical control");
    assert_no_pair_surface_on_cluster(accessor, "accessor pair");
    assert_ne!(
        field(control, "mass").as_u64().unwrap_or(0),
        field(accessor, "mass").as_u64().unwrap_or(0),
        "one pair is byte-identical and the other differs in kind and body; \
         a report that gives them the same mass has lost the distinction: \
         {report:#}"
    );
    assert!(
        field(control, "rank").as_u64().unwrap_or(0)
            < field(accessor, "rank").as_u64().unwrap_or(0),
        "the corroborated control must outrank the gate-skipped accessor pair: \
         {report:#}"
    );
    Ok(())
}
