//! End-to-end regression coverage for #339, sibling-window arm
//! ([FUSION-SIGNALS-THREE-LAYER], [DECISION-TYPE3-TWO-PASS]).
//!
//! `fsharp_issue_339_token_fallback_rename.rs` pins the exact-node arm:
//! a whole-module clone resolves through `locate` and keeps its token
//! signal. This file pins the arm that `locate` cannot serve.
//!
//! A sibling-window fingerprint spans `first.byte_range.start ..
//! last.byte_range.end` across several consecutive children, so no
//! single subtree carries that range. `token_stream_for_fingerprint`
//! resolves ranges through the exact-node path only, returns `None`, and
//! the signature falls back to the issue-86 fingerprint-scoped hash of
//! `(hash, byte_range)`. Two copies of one window share a structural
//! hash but not their offsets, so `token_jaccard` measures whether the
//! byte ranges happen to coincide — byte-offset luck, not token
//! evidence.
//!
//! F# and PHP reach this every time: `is_import_boilerplate_carrier`
//! has no arm for either, so `exact_range_contains_boilerplate` is
//! always false and the language-aware path — the one that *can*
//! resolve a sibling window — is never selected.
//!
//! Correct behaviour pinned here: the normalised kind stream is
//! rename-invariant by construction, so a module rename that shifts
//! every following offset must leave `token_jaccard` at 1.0 and leave
//! the pair in the act-now bucket.

use serde_json::Value;

mod common;
use crate::common::*;

/// Two consecutive top-level bindings that are duplicated verbatim, plus
/// a third that differs per file. The duplicated region is therefore a
/// two-child window inside a three-child module — a range no single
/// subtree covers, which is what forces the sibling-window path.
fn module_with_shared_window(module_name: &str, tail_seed: i32) -> String {
    format!(
        "module {module_name}\n\n\
         let accumulate (values: int list) (floor: int) =\n\
         \x20   let mutable total = 0\n\
         \x20   for value in values do\n\
         \x20       if value > floor then\n\
         \x20           total <- total + value * 2\n\
         \x20       else\n\
         \x20           total <- total - 1\n\
         \x20   total\n\n\
         let combine (values: int list) (ceiling: int) =\n\
         \x20   let mutable carried = 1\n\
         \x20   for value in values do\n\
         \x20       if value < ceiling then\n\
         \x20           carried <- carried * value + 7\n\
         \x20       else\n\
         \x20           carried <- carried - 3\n\
         \x20   carried\n\n\
         let tail (input: int) =\n\
         \x20   input + {tail_seed}\n"
    )
}

/// Asserts the shared window keeps full token identity across a rename
/// that shifts every byte offset in the second file.
fn assert_window_rename_invariant(report: &Value) -> Result<()> {
    let clone = expect_cluster_spanning(report, &["window_a.fs", "window_b.fs"])?;
    assert!(
        approx(signal(clone, "structural"), 1.0),
        "the shared window is a Merkle match — structural must be 1.0: {report:#}"
    );
    assert!(
        approx(signal(clone, "token_jaccard"), 1.0),
        "issue #339: a sibling-window fingerprint must score token_jaccard from its \
         normalised kind stream, which a rename cannot change. Reading 0.0 here means \
         the signature fell back to blake3(hash, byte_range) and measured offset luck: \
         {report:#}"
    );
    assert_ne!(
        cluster_bucket(clone),
        "structural_only",
        "issue #339: a duplicated window whose content agrees must not be demoted to \
         the shape-only tier by a fallback-signature artifact: {report:#}"
    );
    assert_eq!(
        cluster_bucket(clone),
        "nearly_identical",
        "a renamed copy whose content agrees must stay act-now: {report:#}"
    );
    assert!(
        signal(clone, "fused") >= 0.85,
        "content-supported window rename must keep act-now confidence: {report:#}"
    );
    Ok(())
}

// [FUSION-SIGNALS-THREE-LAYER] / #339: `module ParseHelpersB` is one
// character longer than `module ParseHelpers`, so every byte offset in
// the second file shifts by one. The duplicated two-binding window is
// unchanged, so every token-layer reading must be unchanged too.
#[test]
fn issue_339_sibling_window_survives_offset_shifting_rename() -> Result<()> {
    let files = [
        (
            "window_a.fs".to_owned(),
            module_with_shared_window("ParseHelpers", 11),
        ),
        (
            "window_b.fs".to_owned(),
            module_with_shared_window("ParseHelpersB", 29),
        ),
    ];
    let report = report_for(&files, 20)?;
    assert_window_rename_invariant(&report)
}

// The control: identical module names keep the two windows at identical
// byte offsets, so the fallback signature collides and reads 1.00. Both
// files must report the same token evidence as the renamed pair above —
// a signal that changes when only the offsets change is not measuring
// tokens. Pinning both arms is what makes the 1.00 in the aligned case
// evidence rather than luck.
#[test]
fn issue_339_sibling_window_token_signal_is_offset_independent() -> Result<()> {
    let aligned = [
        (
            "window_a.fs".to_owned(),
            module_with_shared_window("ParseHelpers", 11),
        ),
        (
            "window_b.fs".to_owned(),
            module_with_shared_window("ParseHelpers", 29),
        ),
    ];
    let report = report_for(&aligned, 20)?;
    let clone = expect_cluster_spanning(&report, &["window_a.fs", "window_b.fs"])?;
    assert!(
        approx(signal(clone, "token_jaccard"), 1.0),
        "the aligned window pair must read full token identity: {report:#}"
    );
    Ok(())
}
