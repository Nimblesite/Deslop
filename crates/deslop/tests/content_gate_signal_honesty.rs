//! [FUSED-CONTENT-GATE] — the gate may correct a signal it can prove,
//! and may not publish one it did not measure (gh #431).
//!
//! `content_gated_signals` rewrites `token_jaccard` to `1.0` for a
//! `nearly_identical` cluster, on a Merkle argument: members that share
//! one normalised-subtree digest have equal kind streams *by
//! construction*, so a lower measured value is a fingerprint-scoped
//! fallback-signature artifact rather than evidence. That argument is
//! sound, and it holds at digest equality and nowhere else.
//!
//! The guard it was given is `structural >= STRUCTURAL_SATURATION_FLOOR`
//! — a near-miss **routing** tolerance. Since #408 measured `structural`
//! as graded subtree overlap rather than digest equality, every value in
//! `[0.99, 1.0)` reaches that guard, and every one of them means the
//! members' subtrees provably differ: there is no shared digest, so the
//! Merkle argument covers none of them. Routing tolerance is not
//! evidence of identity.
//!
//! The published `token_jaccard` is fabricated for a cluster in that
//! band, and `shape` — `max(structural, token_jaccard)` — inherits it.
//!
//! # Why this fixture cannot pass by going blind
//!
//! `ledger_alpha.py` / `ledger_beta.py` are a long Type-3 pair whose one
//! control-flow node changes from `if` to `while`. The edit keeps their
//! measured structural overlap inside `[0.99, 1.0)` without relying on an
//! operator that the normaliser must treat as a hard contradiction. The
//! byte-identical control in `content-gate-unsaturated` proves the
//! digest-equal correction separately. A fix that bought honesty by
//! desaturating everything fails on that control; a fix that fabricated
//! the band still fails on the Type-3 pair.

use serde_json::Value;

use deslop_core::buckets::{CONTENT_SUPPORT_FLOOR, STRUCTURAL_SATURATION_FLOOR};

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

/// Saturation: the reading the gate may publish only for members proven
/// to share a normalised subtree.
const SATURATED: f64 = 1.0;

/// Renders the deliberately near-saturated Type-3 fixture.
fn render_saturation_band() -> Result<Value> {
    run_report(
        &fixture("content-gate-saturation-band"),
        SATURATION_BAND_MIN_NODES,
    )
}

/// Asserts the published signal triple is internally consistent: `shape`
/// is the shape reading of the two axes beside it
/// ([FUSED-CONTENT-GATE]). A gate that overwrote an axis after computing
/// the reading would publish numbers that disagree with the reading
/// derived from them, observable from the report alone.
fn assert_shape_is_reproducible(cluster: &Value, label: &str) {
    let structural = signal(cluster, "structural");
    let token_jaccard = signal(cluster, "token_jaccard");
    let shape = signal(cluster, "shape");
    assert!(
        approx(shape, structural.max(token_jaccard)),
        "{label}: rendered shape={shape} is not max(structural={structural}, \
         token_jaccard={token_jaccard}) — a published axis disagrees with the \
         reading derived from it: {cluster:#}"
    );
    assert!(
        cluster.pointer("/signals/fused").is_none(),
        "{label}: no cluster-level fused field may survive on the wire \
         ([FUSED-SCOPE]): {cluster:#}"
    );
}

// The defect: a `nearly_identical` cluster whose members are not
// digest-equal must not be published carrying `token_jaccard = 1.00`, a
// value no measurement supports, nor a `shape = 1.00` derived from it.
//
// The Type-3 pair is the whole point of the fixture here. Its `structural` is
// asserted strictly below saturation first — that is the precondition
// that makes every assertion after it meaningful, and it is what a
// future normalisation change would break silently.
#[test]
fn the_content_gate_publishes_no_token_jaccard_it_did_not_measure() -> Result<()> {
    let report = render_saturation_band()?;
    let ledger = clusters(&report)
        .iter()
        .find(|cluster| {
            SATURATION_BAND_PAIR
                .iter()
                .all(|file| cluster_file_set(cluster).contains(*file))
                && (STRUCTURAL_SATURATION_FLOOR..SATURATED).contains(&signal(cluster, "structural"))
        })
        .ok_or_else(|| anyhow::anyhow!("expected a cluster in the [0.99, 1.0) band: {report:#}"))?;
    assert!(
        ACT_NOW_BUCKETS.contains(&cluster_bucket(ledger)),
        "the ledger pair must reach a shape-identical bucket for the gate to \
         run on it at all — it routed {bucket}: {report:#}",
        bucket = cluster_bucket(ledger),
    );
    let structural = signal(ledger, "structural");
    assert!(
        (STRUCTURAL_SATURATION_FLOOR..SATURATED).contains(&structural),
        "the ledger pair must measure inside the [0.99, 1.0) band this defect \
         lives in — it measured structural={structural}: {report:#}"
    );
    let token_jaccard = signal(ledger, "token_jaccard");
    assert!(
        token_jaccard < SATURATED,
        "the ledger pair renders token_jaccard={token_jaccard}: the members' \
         subtrees differ (structural={structural} < 1.0), so they share no \
         digest and nothing proves their kind streams equal — a saturated \
         token axis here is fabricated, not measured: {ledger:#}"
    );
    let shape = signal(ledger, "shape");
    assert!(
        shape < SATURATED,
        "the ledger pair renders shape={shape}: the shape reading inherits the \
         fabricated token axis and claims a perfect shape match for members \
         measured at structural={structural}: {ledger:#}"
    );
    assert_shape_is_reproducible(ledger, "ledger pair");
    Ok(())
}

// The other half of the contract: the correction must keep working for
// the population the Merkle argument actually covers. A digest-equal
// pair renders the saturated triple.
//
// Asserted in the same run as the ledger pair so "the band is honest"
// can never be bought by desaturating every cluster in the report.
#[test]
fn a_digest_equal_pair_keeps_its_saturated_signals() -> Result<()> {
    let report = render_unsaturated()?;
    let control = expect_cluster_spanning(&report, &SATURATED_CONTROL_PAIR)?;
    assert_eq!(
        cluster_bucket(control),
        "identical",
        "the byte-identical control must stay in the identical bucket, \
         otherwise the run proves nothing about what the gate protects: \
         {report:#}"
    );
    for axis in ["structural", "token_jaccard", "shape"] {
        let value = signal(control, axis);
        assert!(
            approx(value, SATURATED),
            "the byte-identical control renders {axis}={value}, not {SATURATED}: \
             a correction scoped to genuine digest equality must leave this \
             pair saturated: {control:#}"
        );
    }
    assert_shape_is_reproducible(control, "byte-identical control");
    Ok(())
}

/// The pair that reproduces gh #460: two unrelated tree-sitter field
/// accessors — different node kind, different field, different body —
/// whose only shared authored logic is the grammar-mandated accessor
/// idiom. Their shared-subtree shape does not saturate, so the content
/// gate measures its observations but does not use them for routing.
const ACCESSOR_PAIR: [&str; 2] = ["accessor_argument.rs", "accessor_assignment.rs"];

/// The byte-identical control in the same fixture: both axes saturate,
/// the gate runs, and `pair_agreement = 1.00` genuinely corroborates.
const SATURATED_CONTROL_PAIR: [&str; 2] = ["control_alpha.rs", "control_beta.rs"];

/// The clause `render::signals::corroborated_verdict` emits. It tells
/// the reader the measured content evidence vouches for the shape
/// reading.
const CORROBORATED_CLAUSE: &str = "the content evidence vouches for it";

/// The clause that makes the below-saturation routing boundary explicit.
const GATE_SKIPPED_CLAUSE: &str = "the content check runs only where the shape match saturates";

/// Renders the unsaturated-gate fixture once per assertion below. Both
/// of its pairs are single function bodies, so they cluster at the same
/// [`MIN_NODES`] floor the ledger corpus uses.
fn render_unsaturated() -> Result<Value> {
    run_report(&fixture("content-gate-unsaturated"), MIN_NODES)
}

// [FUSED-CONTENT-GATE] gh #460 — the report may not tell a reader that
// content evidence corroborated a match whose evidence did not
// corroborate it.
//
// The shared accessor shape stays below saturation, so the content gate
// cannot route it. The pair keeps the anchor-free `nearly_identical`
// verdict while its evidence sentence states that the measured content
// values were observations, not corroboration. This E2E pins that
// distinction through the whole pipeline.
#[test]
fn a_cluster_whose_evidence_did_not_corroborate_is_not_told_it_agreed() -> Result<()> {
    let report = render_unsaturated()?;
    let accessor = expect_cluster_spanning(&report, &ACCESSOR_PAIR)?;
    assert_eq!(
        cluster_bucket(accessor),
        "nearly_identical",
        "below-saturation content observations do not override the anchor-free \
         route — it routed {bucket}: {report:#}",
        bucket = cluster_bucket(accessor),
    );
    let structural = signal(accessor, "structural");
    assert!(
        structural < STRUCTURAL_SATURATION_FLOOR,
        "the content gate must be skipped only because the elected pair stays \
         below the saturation floor: {accessor:#}"
    );
    let support =
        signal(accessor, "pair_agreement").max(signal(accessor, "pair_rename_consistency"));
    assert!(
        support < CONTENT_SUPPORT_FLOOR,
        "the accessor pair's content evidence must be measured and below the \
         floor for the verdict to be making a false claim about it — \
         support={support}: {accessor:#}"
    );
    let verdict = cluster_verdict(accessor);
    assert!(
        !verdict.contains(CORROBORATED_CLAUSE),
        "the evidence did not clear the content floor (structural={structural}, \
         support={support}), so the verdict must not claim the content evidence \
         vouches for the shape — {verdict}"
    );
    assert!(
        verdict.contains(GATE_SKIPPED_CLAUSE),
        "the report must say why the measured evidence did not participate in \
         routing: {verdict}"
    );
    Ok(())
}

// The other half of the contract, asserted in the same run so honesty
// for the gate-skipped population can never be bought by deleting the
// sentence everywhere. The byte-identical control saturates both axes,
// the gate does run on it, and `pair_agreement = 1.00` is a real
// corroboration the reader is entitled to read.
#[test]
fn a_gated_cluster_still_reports_the_evidence_that_corroborated_it() -> Result<()> {
    let report = render_unsaturated()?;
    let control = expect_cluster_spanning(&report, &SATURATED_CONTROL_PAIR)?;
    assert_eq!(
        cluster_bucket(control),
        IDENTICAL_BUCKET,
        "the byte-identical control must stay identical, otherwise the run \
         proves nothing about the population the gate does serve: {report:#}"
    );
    let agreement = signal(control, "pair_agreement");
    assert!(
        approx(agreement, SATURATED),
        "the control's copies are byte-identical, so every collapsed position \
         agrees: pair_agreement={agreement}: {control:#}"
    );
    let control_verdict = cluster_verdict(control);
    assert!(
        control_verdict.contains(CORROBORATED_CLAUSE),
        "the gate ran here and the evidence did corroborate, so the reader must \
         still be told so: {control_verdict}"
    );
    let accessor_verdict = cluster_verdict(expect_cluster_spanning(&report, &ACCESSOR_PAIR)?);
    assert_ne!(
        control_verdict, accessor_verdict,
        "one cluster agrees on 1.00 of its content and the other falls below \
         the content floor; a report that says the same sentence about both \
         has told the reader nothing: {report:#}"
    );
    Ok(())
}

/// A cluster's rendered `evidence_verdict`, the sentence every surface
/// quotes verbatim ([FUSED-CONTENT-GATE]).
fn cluster_verdict(cluster: &Value) -> String {
    field(cluster, "evidence_verdict")
        .as_str()
        .unwrap_or_default()
        .to_owned()
}
