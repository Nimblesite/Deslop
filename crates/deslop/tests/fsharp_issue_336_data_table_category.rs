//! End-to-end regression coverage for the F# data-table false positive
//! ([CLONE-NOISE-LITERAL-TABLE],
//! [FUSED-CONTENT-GATE], [RANK-MASS-SUM]).
//!
//! The defect was a false positive: an F# integer array literal family —
//! same 24-slot shape, different values in every file — ranked #1 on
//! `dotnet/fsharp`, above every genuine clone. [CLONE-NOISE-LITERAL-TABLE]
//! names that report as the defect, and [FUSED-CONTENT-GATE] states the
//! rule that closes it: a data table's literals all differ, so a
//! shape-saturated table pair "falls low" on content and is not admitted.
//! [RANK-MASS-SUM] then orders whatever *is* admitted by mass alone.
//!
//! What this suite pins on the mass-only wire:
//! - the genuine byte-identical clone is the report's first cluster,
//!   byte-proven, spanning exactly its two files;
//! - no cluster touching a distinct-value table ranks at or above it —
//!   the original report, asserted as the bug it is;
//! - two tables that share no literal value never weld: with zero
//!   agreement and nothing to rename, no admission route can carry them;
//! - any table cluster that does publish is byte-distinct, and every
//!   cluster keeps the structural-only contract;
//! - the retired `data_clones` / `data_clone_weight` knobs still parse
//!   (backwards compatibility) but must not change the report;
//! - the #190 verbatim escape hatch: a byte-for-byte copied table is
//!   proven duplication and is byte-proven like any copy.

use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;

use crate::common::{
    corpora::*,
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    *,
};

/// The genuine clone: two byte-identical F# modules.
const CLONE_FILES: [&str; 2] = ["parse_a.fs", "parse_b.fs"];

/// Where the genuine clone must sit on the wire: first, ahead of every
/// table. Report ranks are one-based.
const CLONE_RANK: u64 = 1;

/// Table pairs whose 24 values are pairwise disjoint. `fsharp_table_file`
/// fills slot `i` of seed `s` with `(37s + 13i) mod 97`; since
/// `13⁻¹ ≡ 15 (mod 97)` each file is the window `13·[k, k+23]` with
/// `k = 0, 70, 43, 16` for seeds `0..3`, and only the seed-0 and seed-3
/// windows overlap (eight values). Every other pair shares nothing, so
/// its content agreement is the module keyword and the `lookup` name —
/// far below every admission floor ([FUSED-CONTENT-GATE]).
const DISJOINT_TABLE_PAIRS: [(&str, &str); 5] = [
    ("tables_0.fs", "tables_1.fs"),
    ("tables_0.fs", "tables_2.fs"),
    ("tables_1.fs", "tables_2.fs"),
    ("tables_1.fs", "tables_3.fs"),
    ("tables_2.fs", "tables_3.fs"),
];

/// True for the distinct-value table files in the shared #336 corpus.
fn is_table_file(name: &str) -> bool {
    name.starts_with("tables_")
}

/// Whether the cluster carries a distinct-value table file.
fn touches_table(cluster: &Value) -> bool {
    cluster_file_set(cluster)
        .iter()
        .any(|name| is_table_file(name))
}

/// Renders the shared #336 corpus with an optional `.deslop.toml` body.
fn tables_report(config: Option<&str>) -> Result<(tempfile::TempDir, PathBuf, Value)> {
    let mut files = fsharp_tables_corpus();
    if let Some(body) = config {
        files.push((".deslop.toml".to_owned(), body.to_owned()));
    }
    report_for_with_root(&files, 20)
}

/// [CLONE-NOISE-LITERAL-TABLE] / [RANK-MASS-SUM]: the genuine
/// clone is the first cluster and no distinct-value table outranks it.
/// [FUSED-CONTENT-GATE]: tables sharing no literal never weld. Each
/// cluster is byte-honest on the wire.
#[test]
fn fsharp_numeric_tables_and_clone_publish_ranked_by_mass() -> Result<()> {
    let (_workspace, root, report) = tables_report(None)?;
    let clone = expect_cluster_spanning(&report, &CLONE_FILES)?;
    assert_eq!(
        field(clone, "rank").as_u64(),
        Some(CLONE_RANK),
        "[RANK-MASS-SUM]: the byte-identical clone is the heaviest \
         genuine finding and must lead the report: {report:#}"
    );
    assert!(
        has_verbatim_pair(&root, clone)?,
        "the genuine clone is byte-proven from the fixture source: {clone:#}"
    );
    for cluster in clusters(&report) {
        if touches_table(cluster) {
            assert_table_cluster_is_honest(&root, cluster)?;
        }
        assert_structural_only_contract(cluster, "fsharp #336");
        assert_no_pair_surface_on_cluster(cluster, "fsharp #336");
    }
    Ok(())
}

/// One published table cluster: it ranks below the clone, it is
/// byte-distinct, and it never welds two tables that share no value.
fn assert_table_cluster_is_honest(root: &PathBuf, cluster: &Value) -> Result<()> {
    assert!(
        field(cluster, "rank").as_u64() > Some(CLONE_RANK),
        "[CLONE-NOISE-LITERAL-TABLE]: a distinct-value table family \
         ranking at or above the genuine clone is the reported defect: {cluster:#}"
    );
    assert!(
        !has_verbatim_pair(root, cluster)?,
        "the table family is byte-distinct — same shape, different values — \
         and must not read as a copy: {cluster:#}"
    );
    let files = cluster_file_set(cluster);
    for (left, right) in DISJOINT_TABLE_PAIRS {
        assert!(
            !(files.contains(left) && files.contains(right)),
            "[FUSED-CONTENT-GATE]: {left} and {right} share no literal value; a \
             cluster welding them admitted a pair with no content support: {cluster:#}"
        );
    }
    Ok(())
}

/// The retired `data_clones` / `data_clone_weight` knobs still parse for
/// backwards compatibility but [RANK-STRUCTURAL-ONLY] forbids them from
/// changing weight: every legacy body must render the identical report.
#[test]
fn retired_data_clone_knobs_do_not_change_the_report() -> Result<()> {
    let (_baseline_workspace, _baseline_root, baseline) = tables_report(None)?;
    let ranked_baseline = rankable(&baseline);
    for (body, label) in [
        (
            "[ranking]\ndata_clones = \"ignore\"\n",
            "data_clones ignore",
        ),
        ("[ranking]\ndata_clone_weight = 1.0\n", "data_clone_weight"),
    ] {
        let (_workspace, _root, report) = tables_report(Some(body))?;
        assert_eq!(
            rankable(&report),
            ranked_baseline,
            "{label}: the retired {label} knob must not change mass or order — \
             weight means mass and nothing else ([RANK-STRUCTURAL-ONLY]): {report:#}"
        );
        assert_eq!(
            field(&report, "clusters_hidden").as_u64(),
            Some(0),
            "{label}: the retired knob must not hide the table family: {report:#}"
        );
    }
    assert!(!ranked_baseline.is_empty());
    Ok(())
}

/// The stable, order-insensitive fingerprint of a report's ranking:
/// `(rank, id, mass)` per cluster.
fn rankable(report: &Value) -> Vec<(u64, &str, u64)> {
    let mut rows: Vec<(u64, &str, u64)> = clusters(report)
        .iter()
        .map(|cluster| {
            (
                field(cluster, "rank").as_u64().unwrap_or(0),
                cluster_id(cluster),
                field(cluster, "mass").as_u64().unwrap_or(0),
            )
        })
        .collect();
    rows.sort_unstable();
    rows
}

// [CLONE-NOISE-LITERAL-TABLE] verbatim escape hatch (#190): a
// byte-for-byte copied table is genuine duplication and is byte-proven
// like any copy — never misread as a shape-only family.
#[test]
fn verbatim_copied_fsharp_table_is_byte_proven() -> Result<()> {
    let table = fsharp_table_file("SharedTable", 2);
    let files = genuine_pair("copy_a.fs", "copy_b.fs", &table);
    let (_workspace, root, report) = report_for_with_root(&files, 20)?;
    let copy = expect_cluster_spanning(&report, &["copy_a.fs", "copy_b.fs"])?;
    assert!(
        has_verbatim_pair(&root, copy)?,
        "a byte-identical table pair is a proven copy: {report:#}"
    );
    assert_structural_only_contract(copy, "fsharp copied table");
    assert_no_pair_surface_on_cluster(copy, "fsharp copied table");
    Ok(())
}
