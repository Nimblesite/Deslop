//! Renaming a literal *alongside the symbol it names* is part of the
//! rename, not evidence against it ([FUSION-CONTENT-GATE],
//! [TECH-PMATCH-BAKER], [REPAIR-RENAME-LITERAL-ECHO], gh #409).
//!
//! `content::rename` scores rename evidence as
//! `min(literal_consistency, coverage) * anchor_weight(anchors)`. That
//! first factor used to be a bare count of byte-identical literal
//! positions: it never asked *why* a literal differed. A string that
//! spells the name of a renamed class therefore read as a contradiction
//! of the rename, even though renaming it is what makes the rename
//! complete.
//!
//! The consequence was not merely a lower score, it was an inverted one.
//! These two fixtures are the same rename of the same class pair and
//! differ by a single string literal:
//!
//! - `ts-rename-literal-consistent` renames `"OrderService"` to
//!   `"UserService"` along with the `OrderService` symbol — the rename a
//!   careful developer performs.
//! - `ts-rename-literal-inconsistent` leaves `"OrderService"` behind —
//!   the same rename, done sloppily and left half-finished.
//!
//! Before the fix the sloppy one won: `nearly_identical` at
//! `fused 0.7714` / `rename_consistency 0.8571`, against
//! `structural_only` at `fused 0.3833` / `rename_consistency 0.4259`.
//! The thorough rename landed below the `< 0.6` band in which `CLAUDE.md`
//! instructs an agent to **write the copy anyway**, so the tool's advice
//! got worse the more consistently the developer renamed — a false
//! negative, and the whole of gh #409 in one variable.
//!
//! Green since [REPAIR-RENAME-LITERAL-ECHO]: a substituted literal whose
//! bytes transform into the partner's by an elected identifier
//! substitution counts as consistent, and corroborates that
//! substitution. Do not weaken this suite, and never raise or lower a
//! threshold to keep it passing — the monotonicity assertion is true
//! independently of where any floor sits. Changes to the literal term
//! must be re-measured against
//! `dart_issue_197_single_file_structural_only.rs` and the F# data-table
//! corpus, which that same term is what protects.

use serde_json::Value;

mod common;
use crate::common::{signals::*, *};

/// Node floor matching the rename suites, so the class body qualifies as
/// a candidate on both sides.
const MIN_NODES: u32 = 12;

/// The two sides of the rename, present in both fixtures.
const SIDES: [&str; 2] = ["order_gateway.ts", "user_gateway.ts"];

/// The one cluster spanning both sides of the rename.
fn rename_cluster(report: &Value) -> Result<&Value> {
    clusters(report)
        .iter()
        .find(|cluster| {
            SIDES.iter().all(|side| {
                occurrences(cluster)
                    .iter()
                    .any(|occurrence| occurrence_path(occurrence).ends_with(side))
            })
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no rendered cluster spans both gateways; a consistent rename of a \
                 whole class that reaches no visible cluster is a false negative"
            )
        })
}

/// An occurrence's reported path.
fn occurrence_path(occurrence: &Value) -> &str {
    occurrence
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// Scans one fixture and returns the cluster spanning both gateways.
fn rename_report(fixture_name: &str) -> Result<Value> {
    run_report(&fixture(fixture_name), MIN_NODES)
}

#[test]
fn renaming_a_literal_with_its_symbol_is_still_a_type2_clone() -> Result<()> {
    let root = fixture("ts-rename-literal-consistent");
    let report = rename_report("ts-rename-literal-consistent")?;
    let cluster = rename_cluster(&report)?;

    assert_eq!(
        cluster_size(cluster),
        2,
        "the rename has exactly two occurrences — {dump}",
        dump = signal_dump(cluster)
    );
    // Bytes differ, nothing hidden, saturating shape, an act-now bucket
    // that is not shape-only, fused inside the reuse band. Stated once in
    // `common::signals` so every rename fixture is judged by one rule.
    assert_proven_rename_contract(&root, cluster, "ts-rename-literal-consistent")
}

#[test]
fn a_more_consistent_rename_never_scores_lower_than_a_less_consistent_one() -> Result<()> {
    let consistent = rename_report("ts-rename-literal-consistent")?;
    let inconsistent = rename_report("ts-rename-literal-inconsistent")?;
    let thorough = rename_cluster(&consistent)?;
    let sloppy = rename_cluster(&inconsistent)?;

    // The fixtures are the same rename of the same classes; the only
    // difference is whether the literal naming the renamed symbol was
    // renamed too. Finishing the rename cannot be evidence against it.
    assert!(
        signal(thorough, "rename_consistency") >= signal(sloppy, "rename_consistency"),
        "completing a rename must not reduce the measured rename evidence: \
         consistent={consistent_rename:.4} < inconsistent={sloppy_rename:.4}\n  \
         consistent: {consistent_dump}\n  inconsistent: {sloppy_dump}",
        consistent_rename = signal(thorough, "rename_consistency"),
        sloppy_rename = signal(sloppy, "rename_consistency"),
        consistent_dump = signal_dump(thorough),
        sloppy_dump = signal_dump(sloppy),
    );
    assert!(
        signal(thorough, "fused") >= signal(sloppy, "fused"),
        "completing a rename must not reduce the rendered confidence — the tool's \
         advice would get worse the more carefully the developer renamed: \
         consistent={consistent_fused:.4} < inconsistent={sloppy_fused:.4}\n  \
         consistent: {consistent_dump}\n  inconsistent: {sloppy_dump}",
        consistent_fused = signal(thorough, "fused"),
        sloppy_fused = signal(sloppy, "fused"),
        consistent_dump = signal_dump(thorough),
        sloppy_dump = signal_dump(sloppy),
    );
    Ok(())
}
