//! End-to-end coverage for #339, sibling-window arm
//! ([FUSION-SIGNALS-THREE-LAYER], [DECISION-TYPE3-TWO-PASS]).
//!
//! **What this file can and cannot prove.** The token-evidence question —
//! does a sibling-window fingerprint score `token_jaccard` from its normalised
//! kind stream, or from the issue-86 offset-seeded fallback? — is *not*
//! answerable from a rendered report. `buckets::content_gated_signals`
//! overwrites `token_jaccard` to `1.0` for every shape-identical cluster it
//! routes `NearlyIdentical`:
//!
//! ```text
//! let token_jaccard = if kind == ClusterKind::NearlyIdentical && signals.structural >= 0.99 {
//!     1.0
//! } else { signals.token_jaccard };
//! ```
//!
//! So a rendered `1.00` is supplied by the renderer, not measured, and an E2E
//! assertion on it passes whether or not the signature layer works. That
//! question is pinned where it is answerable, at the signature layer:
//! `deslop-core::pipeline::signatures::tests::issue_339_sibling_window_signature_is_offset_invariant`.
//! **That test is currently RED — #339 is live.**
//!
//! What this file pins instead is the part only an end-to-end run can show:
//! that the duplicated region surfaces as a *sibling window* at all — a range
//! spanning several consecutive children that no single subtree covers — and
//! that it reaches an act-now bucket rather than being demoted to the
//! shape-only tier by a fallback-signature artifact.
//!
//! F# and PHP reach the sibling-window path every time:
//! `is_import_boilerplate_carrier` has no arm for either, so
//! `exact_range_contains_boilerplate` is always false and the language-aware
//! path — the one that *can* resolve a sibling window — is never selected.

use anyhow::anyhow;
use serde_json::Value;

mod common;
use crate::common::*;

/// The duplicated region: two consecutive top-level bindings, verbatim in
/// both files.
const SHARED_WINDOW: &str = "\
let accumulate (values: int list) (floor: int) =
    let mutable total = 0
    for value in values do
        if value > floor then
            total <- total + value * 2
        else
            total <- total - 1
    total

let combine (values: int list) (ceiling: int) =
    let mutable carried = 1
    for value in values do
        if value < ceiling then
            carried <- carried * value + 7
        else
            carried <- carried - 3
    carried
";

/// The tail of `window_a.fs`.
///
/// Structurally different from [`TAIL_B`] — a different *shape*, not a
/// different literal. The first version of this fixture varied only a numeric
/// literal, and normalisation collapses literals, so both modules normalised
/// to one whole-file clone: the reported cluster spanned bytes `0..524` of a
/// 525-byte file and the sibling-window path was never reached at all. The
/// tails must diverge in shape or there is no window, only a file.
const TAIL_A: &str = "
let tail (input: int) =
    input + 11
";

/// The tail of `window_b.fs` — a match expression plus an extra binding.
const TAIL_B: &str = "
let tail (input: int) =
    match input with
    | 0 -> \"zero\"
    | 1 -> \"one\"
    | other -> string other

let extra (a: int) (b: int) (c: int) =
    let mutable acc = a
    while acc < b do
        acc <- acc + c
    acc
";

/// A module whose middle is [`SHARED_WINDOW`] and whose tail is `tail`.
fn module_with_shared_window(module_name: &str, tail: &str) -> String {
    format!("module {module_name}\n\n{SHARED_WINDOW}{tail}")
}

/// The `(start, end)` byte range of an occurrence in `file`.
fn occurrence_range(cluster: &Value, file: &str) -> Option<(u64, u64)> {
    cluster
        .get("occurrences")?
        .as_array()?
        .iter()
        .find(|occurrence| {
            occurrence
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| path.ends_with(file))
        })
        .and_then(|occurrence| {
            Some((
                occurrence.get("start_byte")?.as_u64()?,
                occurrence.get("end_byte")?.as_u64()?,
            ))
        })
}

/// Asserts the reported clone really is the shared window — not the whole
/// module — and that it reaches an act-now bucket.
///
/// Selection is by *range*, not by file membership: `expect_cluster_spanning`
/// returns the first cluster whose occurrences merely mention both files,
/// which is satisfied by a whole-module clone and was what made the earlier
/// version of this test a false green.
fn assert_window_clone(report: &Value, sources: (&str, &str), expected: &str) -> Result<()> {
    let clone = expect_cluster_spanning(report, &["window_a.fs", "window_b.fs"])?;

    let (start_a, end_a) =
        occurrence_range(clone, "window_a.fs").ok_or_else(|| anyhow!("no window_a occurrence"))?;
    let (start_b, end_b) =
        occurrence_range(clone, "window_b.fs").ok_or_else(|| anyhow!("no window_b occurrence"))?;

    assert!(
        end_a < sources.0.len() as u64,
        "the clone must not span all of window_a.fs ({end_a} of {} bytes) — a whole-module \
         match is the exact-node path, not the sibling-window path this file exists to \
         cover: {report:#}",
        sources.0.len(),
    );
    assert!(
        end_b < sources.1.len() as u64,
        "the clone must not span all of window_b.fs ({end_b} of {} bytes): {report:#}",
        sources.1.len(),
    );
    assert!(
        end_a > start_a && end_b > start_b,
        "both occurrences must cover a real range: {report:#}"
    );
    assert!(
        approx(signal(clone, "structural"), 1.0),
        "the shared window is a Merkle match — structural must be 1.0: {report:#}"
    );
    assert_ne!(
        cluster_bucket(clone),
        "structural_only",
        "issue #339: a duplicated window whose content agrees must not be demoted to the \
         shape-only tier by a fallback-signature artifact: {report:#}"
    );
    // Actionability is asserted by bucket, deliberately, not by
    // `fused >= FUSED_THRESHOLD`. [REPORTING-CONTEXT] is explicit that a
    // proven Type-2 clone may render *below* the admission bar while
    // remaining actionable, so gating this on the rendered confidence would
    // assert the opposite of the documented contract.
    assert_eq!(
        cluster_bucket(clone),
        expected,
        "a duplicated window whose content agrees must stay act-now: {report:#}"
    );
    assert!(
        is_act_now(cluster_bucket(clone)),
        "and `{expected}` must be an act-now bucket: {report:#}"
    );
    Ok(())
}

/// The wire bucket labels [CLONE-BUCKETS] calls actionable.
fn is_act_now(bucket: &str) -> bool {
    matches!(bucket, "identical" | "nearly_identical")
}

// [FUSION-SIGNALS-THREE-LAYER] / #339: `module ParseHelpersB` is one character
// longer than `module ParseHelpers`, so every byte offset in the second file
// shifts by one. The duplicated two-binding window is unchanged.
#[test]
fn issue_339_sibling_window_survives_offset_shifting_rename() -> Result<()> {
    let source_a = module_with_shared_window("ParseHelpers", TAIL_A);
    let source_b = module_with_shared_window("ParseHelpersB", TAIL_B);
    let files = [
        ("window_a.fs".to_owned(), source_a.clone()),
        ("window_b.fs".to_owned(), source_b.clone()),
    ];
    let report = report_for(&files, 20)?;
    assert_window_clone(&report, (&source_a, &source_b), "nearly_identical")
}

// The control: identical module names keep the two windows at identical byte
// offsets, which makes the shared region byte-for-byte equivalent — so the
// engine proves `identical` here and only `nearly_identical` above. Both are
// act-now, which is the invariant: shifting every offset with a rename must
// not push the window out of the actionable tier. A routing decision that
// *degrades* when only the offsets change is not measuring the code.
#[test]
fn issue_339_sibling_window_routing_is_offset_independent() -> Result<()> {
    let source_a = module_with_shared_window("ParseHelpers", TAIL_A);
    let source_b = module_with_shared_window("ParseHelpers", TAIL_B);
    let files = [
        ("window_a.fs".to_owned(), source_a.clone()),
        ("window_b.fs".to_owned(), source_b.clone()),
    ];
    let report = report_for(&files, 20)?;
    assert_window_clone(&report, (&source_a, &source_b), "identical")
}
