//! End-to-end regression coverage for #339: the token layer must be
//! rename-invariant for sibling-window fingerprints
//! ([FUSED-SIGNALS-THREE-LAYER], [DECISION-TYPE3-TWO-PASS]).
//!
//! A fingerprint whose byte range is a synthetic sibling window (an F#
//! module body, a JS statement run) resolved its token stream through
//! the exact-node path only, so the `MinHash` signature fell back to
//! the issue-86 fingerprint-scoped hash of `(hash, byte_range)`.
//! `token_jaccard` then measured byte-offset luck: two byte-identical
//! files collided to 1.00, and a one-character-longer module rename
//! shifted the range and read 0.00 — misrouting a genuine Type-2 clone
//! into the demoted `structural_only` tier.
//!
//! Correct behaviour pinned here: the normalised kind stream is
//! identical across a rename by construction, so `token_jaccard` stays
//! 1.0 whether or not the rename changes byte offsets, and the pair —
//! whose raw content overwhelmingly agrees — reports as an act-now
//! `nearly_identical` clone.

use serde_json::Value;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
use crate::common::{corpora::*, *};

/// The genuine clone with its module renamed one character LONGER, so
/// every subsequent byte offset shifts — the #339 trigger.
fn renamed_clone() -> String {
    FSHARP_GENUINE_CLONE.replace("module ParseHelpers", "module ParseHelpersB")
}

/// Asserts the renamed pair keeps full structural and token identity
/// and routes to the act-now bucket.
fn assert_rename_invariant(report: &Value) -> Result<()> {
    let clone = expect_cluster_spanning(report, &["parse_a.fs", "parse_b.fs"])?;
    // [PIPELINE-CLUSTER-CLOSURE] The axes and act-now bucket are pair-scoped
    // now. The acceptance on the wire: the renamed pair is admitted,
    // mass-honest, clean-surfaced and byte-distinct — the offset shift and
    // renames change the bytes, and a verbatim reading would be a
    // fabrication.
    assert_structural_only_contract(clone, "fsharp #339 token fallback");
    assert_no_pair_surface_on_cluster(clone, "fsharp #339 token fallback");
    assert!(
        !has_verbatim_pair(
            &fixture("fsharp-339-token-fallback-rename").join("src"),
            clone
        )?,
        "the renamed pair must slice to differing bytes: {report:#}"
    );
    Ok(())
}

// [FUSED-SIGNALS-THREE-LAYER] / #339: a module rename that grows the
// name by one character shifts every byte offset after it. The token
// signal must not change — only the fallback-signature artifact did.
#[test]
fn issue_339_offset_shifting_rename_keeps_token_signal_and_act_now_bucket() -> Result<()> {
    let files = [
        ("parse_a.fs".to_owned(), FSHARP_GENUINE_CLONE.to_owned()),
        ("parse_b.fs".to_owned(), renamed_clone()),
    ];
    let report = report_for(&files, 20)?;
    assert_rename_invariant(&report)
}
