//! End-to-end regression coverage for the categorisation half of #336
//! ([RANK-CATEGORY], [CLONE-NOISE-LITERAL-TABLE]): data-table
//! classification must be language-agnostic. `CloneCategory::DataTable`
//! shipped with a Dart-only classifier, so an F# numeric array family —
//! the exact shape that tops the `dotnet/fsharp` report — carried
//! `category: "logic"` at full weight and the `[ranking] data_clones`
//! policy had nothing to act on.
//!
//! Correct behaviour pinned here: a cluster of same-shape,
//! different-value numeric tables is labelled `data` in any language,
//! the default policy demotes it below genuine logic clones while
//! keeping it visible, `ignore` drops it, `data_clone_weight = 1.0`
//! restores it — and the #190 verbatim escape hatch still routes a
//! byte-for-byte copied table to `logic`, because copied data is real
//! duplication.

use serde_json::Value;

mod common;
use crate::common::{corpora::*, *};

/// True for the distinct-value table files in the shared #336 corpus.
fn is_table_file(name: &str) -> bool {
    name.starts_with("tables_")
}

/// Renders the shared #336 corpus with an optional `.deslop.toml` body.
fn tables_report(config: Option<&str>) -> Result<Value> {
    let mut files = fsharp_tables_corpus();
    if let Some(body) = config {
        files.push((".deslop.toml".to_owned(), body.to_owned()));
    }
    report_for(&files, 20)
}

// [RANK-CATEGORY] / #336: the distinct-value F# array family must carry
// the `data` category so the data policy governs it, stay visible under
// the default demote policy, and rank below the genuine logic clone.
#[test]
fn fsharp_numeric_tables_carry_data_category_and_demote_by_default() -> Result<()> {
    let report = tables_report(None)?;
    assert_eq!(
        category_where(&report, is_table_file),
        "data",
        "distinct-value F# arrays must be categorised data: {report:#}"
    );
    let table_rank = rank_where(&report, is_table_file);
    assert!(
        table_rank.is_some(),
        "the default demote policy keeps the data table visible: {report:#}"
    );
    let clone_rank = rank_where(&report, |name| name.starts_with("parse_"));
    assert!(
        clone_rank < table_rank,
        "the genuine logic clone must outrank the demoted data table: \
         clone at {clone_rank:?}, table at {table_rank:?}\n{report:#}"
    );
    assert_eq!(
        category_where(&report, |name| name.starts_with("parse_")),
        "logic",
        "the genuine clone stays logic: {report:#}"
    );
    Ok(())
}

// [RANK-CATEGORY] ignore mode: the labelled table is dropped outright
// while the genuine clone survives.
#[test]
fn ignore_mode_drops_fsharp_tables_and_keeps_the_logic_clone() -> Result<()> {
    let report = tables_report(Some("[ranking]\ndata_clones = \"ignore\"\n"))?;
    assert!(
        rank_where(&report, is_table_file).is_none(),
        "ignore mode must drop the F# data table from the report: {report:#}"
    );
    assert!(
        rank_where(&report, |name| name.starts_with("parse_")).is_some(),
        "the genuine clone must survive ignore mode: {report:#}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the dropped table must be counted under clusters_hidden: {report:#}"
    );
    Ok(())
}

// [RANK-CATEGORY] keep mode: `data_clone_weight = 1.0` restores the
// table's full weight, which out-ranks the two-file logic clone —
// proving the knob now drives F# tables, not just Dart ones.
#[test]
fn full_data_clone_weight_restores_fsharp_tables_to_the_top() -> Result<()> {
    let report = tables_report(Some("[ranking]\ndata_clone_weight = 1.0\n"))?;
    let table_rank = rank_where(&report, is_table_file);
    let clone_rank = rank_where(&report, |name| name.starts_with("parse_"));
    assert!(table_rank.is_some(), "table must appear: {report:#}");
    assert!(clone_rank.is_some(), "clone must appear: {report:#}");
    assert!(
        table_rank < clone_rank,
        "full data weight must let the larger table family out-rank the \
         clone: table at {table_rank:?}, clone at {clone_rank:?}\n{report:#}"
    );
    assert_eq!(
        category_where(&report, is_table_file),
        "data",
        "the table stays labelled data at full weight: {report:#}"
    );
    Ok(())
}

// [CLONE-NOISE-LITERAL-TABLE] verbatim escape hatch (#190): a
// byte-for-byte copied table is genuine duplication and must stay
// `logic` at the `identical` bucket — never demoted as data.
#[test]
fn verbatim_copied_fsharp_table_stays_logic() -> Result<()> {
    let table = fsharp_table_file("SharedTable", 2);
    let files = genuine_pair("copy_a.fs", "copy_b.fs", &table);
    let report = report_for(&files, 20)?;
    let copy = expect_cluster_spanning(&report, &["copy_a.fs", "copy_b.fs"])?;
    assert_eq!(
        cluster_bucket(copy),
        "identical",
        "a byte-identical table pair is a proven copy: {report:#}"
    );
    assert_eq!(
        category_where(&report, |name| name.starts_with("copy_")),
        "logic",
        "a verbatim-copied table is real duplication, not demotable data: {report:#}"
    );
    Ok(())
}
