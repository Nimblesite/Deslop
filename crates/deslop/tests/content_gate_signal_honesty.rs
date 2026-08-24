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

use deslop_core::buckets::{RENAME_CONSISTENCY_DISCOUNT, STRUCTURAL_SATURATION_FLOOR};

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
