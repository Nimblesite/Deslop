//! [FUSION-CONTENT-GATE] — the gate may correct a signal it can prove,
//! and may not publish one it did not measure (gh #431).
//!
//! `apply_content_gate` rewrites `token_jaccard` to `1.0` for a
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
//! Two published numbers are fabricated for a cluster in that band:
//! `token_jaccard`, directly, and `shape` — `max(structural,
//! token_jaccard)` — which inherits it. `fused` does not, because it is
//! computed from the pre-correction triple, and that split is what makes
//! the defect visible from outside the engine: the rendered signals stop
//! reproducing the rendered confidence.
//!
//! # Why this fixture cannot pass by going blind
//!
//! `ledger_credit.py` / `ledger_debit.py` differ in exactly one operator
//! (`+` against `-` on the `shifted` line) inside a twelve-line shared
//! body — the smallest input that lands inside the band, measuring
//! `structural = 0.9907`. `control_alpha.py` / `control_beta.py` hold a
//! byte-identical function whose members *are* digest-equal, so the
//! saturated triple the correction exists to protect is asserted in the
//! same run. A fix that bought honesty by desaturating everything fails
//! on the control; a fix that left the band fabricated fails on the
//! ledger pair.

use serde_json::Value;

use deslop_core::buckets::{
    CONTENT_SUPPORT_FLOOR, RENAME_CONSISTENCY_DISCOUNT, SATURATING_TOKEN_FLOOR,
    STRUCTURAL_SATURATION_FLOOR,
};

use crate::common::{signals::*, *};

/// Node floor low enough that each fixture function body is a candidate
/// window — the same floor `operator_drift_is_not_duplication` renders
/// the fixture at, so both suites describe one measured corpus.
const MIN_NODES: u32 = 8;

/// The pair that lands inside the `[0.99, 1.0)` band: one operator
/// apart, so their normalised subtrees differ and no digest is shared.
const LEDGER_PAIR: [&str; 2] = ["ledger_credit.py", "ledger_debit.py"];

/// The byte-identical pair, whose members genuinely share one digest —
/// the population the Merkle correction is entitled to.
const CONTROL_PAIR: [&str; 2] = ["control_alpha.py", "control_beta.py"];

/// Saturation: the reading the gate may publish only for members proven
/// to share a normalised subtree.
const SATURATED: f64 = 1.0;

/// Renders the operator-drift fixture once for every assertion below.
fn render() -> Result<Value> {
    run_report(&fixture("operator-drift"), MIN_NODES)
}

/// The confidence [FUSION-CONTENT-GATE] fuses shape against: pooled byte
/// agreement, or a discounted literal-anchored rename proof, whichever
/// is the stronger evidence. Read off the *rendered* wire so the check
/// below is a statement about the published report alone.
fn rendered_content_confidence(cluster: &Value) -> f64 {
    signal(cluster, "agreement")
        .max(RENAME_CONSISTENCY_DISCOUNT * signal(cluster, "rename_consistency"))
}

/// Asserts the published signal triple is internally consistent: `shape`
/// is the shape reading of the two axes beside it, and `fused` is that
/// reading scaled by the content evidence rendered next to it (or the
/// semantic signal, when that is stronger).
///
/// This is the property a fabricated signal breaks without any knowledge
/// of what the engine measured. `fused` is computed inside the gate from
/// the pre-correction triple; overwrite an axis afterwards and the
/// published numbers no longer reproduce the published confidence, which
/// is observable from the report alone.
fn assert_signals_reproduce_fused(cluster: &Value, label: &str) {
    let structural = signal(cluster, "structural");
    let token_jaccard = signal(cluster, "token_jaccard");
    let shape = signal(cluster, "shape");
    let fused = signal(cluster, "fused");
    assert!(
        approx(shape, structural.max(token_jaccard)),
        "{label}: rendered shape={shape} is not max(structural={structural}, \
         token_jaccard={token_jaccard}) — a published axis disagrees with the \
         reading derived from it: {cluster:#}"
    );
    let expected =
        signal(cluster, "embedding_cos").max(shape * rendered_content_confidence(cluster));
    assert!(
        approx(fused, expected),
        "{label}: rendered fused={fused} is not reproducible from the rendered \
         signals (expected {expected}) — the confidence was computed from a \
         different triple than the one published: {cluster:#}"
    );
}

// The defect: a `nearly_identical` cluster whose members are not
// digest-equal is published carrying `token_jaccard = 1.00`, a value no
// measurement supports, and a `shape = 1.00` derived from it.
//
// The ledger pair is the whole point of the fixture here: it is the one
// family that clusters at all once the operator reaches the digest, so
// it is the only input that can reach the band. Its `structural` is
// asserted strictly below saturation first — that is the precondition
// that makes every assertion after it meaningful, and it is what a
// future normalisation change would break silently.
#[test]
fn the_content_gate_publishes_no_token_jaccard_it_did_not_measure() -> Result<()> {
    let report = render()?;
    let ledger = expect_cluster_spanning(&report, &LEDGER_PAIR)?;
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
    assert_signals_reproduce_fused(ledger, "ledger pair");
    Ok(())
}

// The other half of the contract: the correction must keep working for
// the population the Merkle argument actually covers. A digest-equal
// pair renders the saturated triple, and its confidence saturates with
// it.
//
// Asserted in the same run as the ledger pair so "the band is honest"
// can never be bought by desaturating every cluster in the report.
#[test]
fn a_digest_equal_pair_keeps_its_saturated_signals() -> Result<()> {
    let report = render()?;
    let control = expect_cluster_spanning(&report, &CONTROL_PAIR)?;
    assert_eq!(
        cluster_bucket(control),
        "identical",
        "the byte-identical control must stay in the identical bucket, \
         otherwise the run proves nothing about what the gate protects: \
         {report:#}"
    );
    for axis in ["structural", "token_jaccard", "shape", "fused"] {
        let value = signal(control, axis);
        assert!(
            approx(value, SATURATED),
            "the byte-identical control renders {axis}={value}, not {SATURATED}: \
             a correction scoped to genuine digest equality must leave this \
             pair saturated: {control:#}"
        );
    }
    assert_signals_reproduce_fused(control, "byte-identical control");
    Ok(())
}

/// The pair that reproduces gh #460: two unrelated tree-sitter field
/// accessors — different node kind, different field, different body —
/// whose only shared authored logic is the grammar-mandated accessor
/// idiom. They measure `structural = 0.82`, `token_jaccard = 0.73`, so
/// neither axis saturates and [FUSION-CONTENT-GATE] never runs on them.
const ACCESSOR_PAIR: [&str; 2] = ["accessor_argument.rs", "accessor_assignment.rs"];

/// The byte-identical control in the same fixture: both axes saturate,
/// the gate runs, and `agreement = 1.00` genuinely corroborates.
const SATURATED_CONTROL_PAIR: [&str; 2] = ["control_alpha.rs", "control_beta.rs"];

/// The clause `render::signals::corroborated_verdict` emits. It tells
/// the reader the measured content evidence was weighed against the
/// shape reading and left it standing.
const CORROBORATED_CLAUSE: &str = "the content evidence did not discount that";

/// Renders the unsaturated-gate fixture once per assertion below. Both
/// of its pairs are single function bodies, so they cluster at the same
/// [`MIN_NODES`] floor the ledger corpus uses.
fn render_unsaturated() -> Result<Value> {
    run_report(&fixture("content-gate-unsaturated"), MIN_NODES)
}

// [FUSION-CONTENT-GATE] gh #460 — the report may not tell a reader that
// content evidence corroborated a match the gate never consulted.
//
// `buckets::routing::route_shape_identical` returns before the gate
// whenever `has_saturating_shape_evidence` is false, and
// `buckets::gate::content_gated_signals` leaves `fused` untouched on the
// same condition. So for every cluster below saturation `fused == shape`
// by construction, and `render::signals::content_evidence_verdict` —
// which sees only the signal triple and branches on `fused + eps >=
// shape` — can reach no branch but `corroborated_verdict`. It publishes
// "the content evidence did not discount that" for a cluster whose
// evidence was measured at 0.31 and then discarded, which is the single
// strongest available disproof of the match rendered to the reader as
// corroboration.
//
// Measured on this repo's own tree (2026-08-27, 1316 visible clusters):
// 637 of 637 non-saturated clusters carry this clause, and 637 of 637
// render `fused == shape`. The branch is unreachable, not merely rare.
#[test]
fn a_gate_skipped_cluster_is_not_told_its_content_evidence_agreed() -> Result<()> {
    let report = render_unsaturated()?;
    let accessor = expect_cluster_spanning(&report, &ACCESSOR_PAIR)?;
    assert!(
        ACT_NOW_BUCKETS.contains(&cluster_bucket(accessor)),
        "the accessor pair must reach an act-now bucket for its verdict to be \
         the sentence a reader acts on — it routed {bucket}: {report:#}",
        bucket = cluster_bucket(accessor),
    );
    let structural = signal(accessor, "structural");
    let token_jaccard = signal(accessor, "token_jaccard");
    assert!(
        structural < STRUCTURAL_SATURATION_FLOOR && token_jaccard < SATURATING_TOKEN_FLOOR,
        "the accessor pair must sit below both saturation floors, which is what \
         scopes [FUSION-CONTENT-GATE] out of it — it measured \
         structural={structural}, token_jaccard={token_jaccard}: {accessor:#}"
    );
    let support = signal(accessor, "agreement").max(signal(accessor, "rename_consistency"));
    assert!(
        support < CONTENT_SUPPORT_FLOOR,
        "the accessor pair's content evidence must be measured and low for the \
         verdict to be making a false claim about it — support={support}: \
         {accessor:#}"
    );
    let verdict = cluster_verdict(accessor);
    assert!(
        !verdict.contains(CORROBORATED_CLAUSE),
        "the gate never ran on this cluster (structural={structural} and \
         token_jaccard={token_jaccard} are both below saturation), so its \
         measured content evidence — support={support} — was discarded rather \
         than weighed. Telling the reader it `did not discount` the shape \
         renders the disproof as corroboration: {verdict}"
    );
    Ok(())
}

// The other half of the contract, asserted in the same run so honesty
// for the gate-skipped population can never be bought by deleting the
// sentence everywhere. The byte-identical control saturates both axes,
// the gate does run on it, and `agreement = 1.00` is a real corroboration
// the reader is entitled to read.
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
    let agreement = signal(control, "agreement");
    assert!(
        approx(agreement, SATURATED),
        "the control's copies are byte-identical, so every collapsed position \
         agrees: agreement={agreement}: {control:#}"
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
        "one cluster agrees on 1.00 of its content and the other on 0.31; a \
         report that says the same sentence about both has told the reader \
         nothing: {report:#}"
    );
    Ok(())
}

/// A cluster's rendered `evidence_verdict`, the sentence every surface
/// quotes verbatim ([FUSION-CONTENT-GATE]).
fn cluster_verdict(cluster: &Value) -> String {
    field(cluster, "evidence_verdict")
        .as_str()
        .unwrap_or_default()
        .to_owned()
}
