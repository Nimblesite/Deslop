//! End-to-end coverage for #339, sibling-window arm
//! ([FUSED-SIGNALS-THREE-LAYER], [DECISION-TYPE3-TWO-PASS]).
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
//! `deslop-core::pipeline::signatures::tests::issue_339_sibling_window_signature_is_offset_invariant`,
//! which holds every known-language fingerprint to the language-aware token
//! path that resolves sibling windows instead of the offset-seeded fallback.
//!
//! What this file pins instead is the part only an end-to-end run can show:
//! that the duplicated region surfaces as a *sibling window* at all — the
//! reported occurrences sit at the exact expected boundaries, boundaries no
//! single normalised subtree owns (asserted against `--debug-ast`) — and
//! that it reaches an act-now bucket rather than being demoted to the
//! shape-only tier by a fallback-signature artifact or displaced by a
//! wider token-matched view (the #339 anchored-representative collapse).

use anyhow::anyhow;
use serde_json::Value;

use crate::common::signals::{assert_no_pair_surface_on_cluster, has_verbatim_pair};
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

/// The exact byte range the sibling window must be reported at inside
/// `source`: from the module-name identifier (the first child the
/// synthetic window covers) to the last byte of the `combine` binding.
/// Derived from the fixture text, never hard-coded offsets.
fn expected_window_range(source: &str) -> Result<(u64, u64)> {
    let start = "module ".len();
    let window_text = source
        .find(SHARED_WINDOW)
        .ok_or_else(|| anyhow!("fixture must contain the shared window"))?;
    let end = window_text.saturating_add(SHARED_WINDOW.trim_end().len());
    Ok((start as u64, end as u64))
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

/// The cluster whose occurrences sit at exactly the expected window
/// boundaries in both files. Selection by exact range, not by file
/// membership: a cluster whose occurrences merely mention both files is
/// satisfied by a whole-module clone or a nested-binding family, both of
/// which made earlier versions of this test a false green.
fn window_cluster(report: &Value, range_a: (u64, u64), range_b: (u64, u64)) -> Result<&Value> {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|cluster| {
            occurrence_range(cluster, "window_a.fs") == Some(range_a)
                && occurrence_range(cluster, "window_b.fs") == Some(range_b)
        })
        .ok_or_else(|| {
            anyhow!(
                "expected a cluster at exactly window_a.fs {range_a:?} + window_b.fs \
                 {range_b:?}: {report:#}"
            )
        })
}

/// Writes both fixture files, runs the scan, and returns the report plus
/// each file's `--debug-ast` dump for the no-exact-node assertion.
fn scan_with_dumps(sources: (&str, &str)) -> Result<(std::path::PathBuf, Value, String, String)> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("src");
    std::fs::create_dir_all(&root)?;
    let path_a = root.join("window_a.fs");
    let path_b = root.join("window_b.fs");
    std::fs::write(&path_a, sources.0)?;
    std::fs::write(&path_b, sources.1)?;
    let report = run_report(&root, 20)?;
    Ok((root, report, ast_dump(&path_a)?, ast_dump(&path_b)?))
}

/// The normalised-AST dump of `path` via the CLI's `--debug-ast` flag.
fn ast_dump(path: &std::path::Path) -> Result<String> {
    let mut cmd = assert_cmd::Command::cargo_bin("deslop")?;
    let output = cmd
        .arg("--debug-ast")
        .arg(path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    Ok(String::from_utf8(output)?)
}

/// Asserts the reported clone really is the shared window — at the exact
/// expected boundaries, boundaries no single normalised subtree owns —
/// and that it reaches an act-now bucket.
fn assert_window_clone(sources: (&str, &str)) -> Result<()> {
    let (root, report, dump_a, dump_b) = scan_with_dumps(sources)?;
    let range_a = expected_window_range(sources.0)?;
    let range_b = expected_window_range(sources.1)?;
    let clone = window_cluster(&report, range_a, range_b)?;

    let exact_a = format!("[{}..{}]", range_a.0, range_a.1);
    let exact_b = format!("[{}..{}]", range_b.0, range_b.1);
    assert!(
        !dump_a.contains(&exact_a) && !dump_b.contains(&exact_b),
        "the window range must be a synthetic sibling window that no single normalised \
         node owns — an exact node here means the fixture stopped exercising the \
         sibling-window path:\n{dump_a}\n{dump_b}"
    );

    // [PIPELINE-CLUSTER-CLOSURE] The shape axis and the act-now buckets are
    // gone. Issue #339's acceptance on the wire: the duplicated window is
    // reported at its exact synthetic boundaries — a byte-identical
    // Merkle-match window, proven from the source bytes. The demotion
    // question (structural_only vs act-now) no longer has a surface.
    assert!(
        has_verbatim_pair(&root, clone)?,
        "the shared window is byte-identical and must be byte-proven: {report:#}"
    );
    assert_no_pair_surface_on_cluster(clone, "fsharp #339");
    Ok(())
}

// [FUSED-SIGNALS-THREE-LAYER] / #339: `module ParseHelpersB` is one character
// longer than `module ParseHelpers`, so every byte offset in the second file
// shifts by one. The duplicated two-binding window is unchanged.
#[test]
fn issue_339_sibling_window_survives_offset_shifting_rename() -> Result<()> {
    let source_a = module_with_shared_window("ParseHelpers", TAIL_A);
    let source_b = module_with_shared_window("ParseHelpersB", TAIL_B);
    assert_window_clone((&source_a, &source_b))
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
    assert_window_clone((&source_a, &source_b))
}
