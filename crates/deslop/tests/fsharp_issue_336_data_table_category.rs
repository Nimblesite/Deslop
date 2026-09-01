//! End-to-end regression coverage for the categorisation half of #336
//! ([RANK-CATEGORY], [CLONE-NOISE-LITERAL-TABLE]).
//!
//! [RANK-CATEGORY] says a detection-time finding kind may drive an
//! explicit exclusion *before* ranking but is not carried as
//! clone-cluster similarity metadata, and [RANK-STRUCTURAL-ONLY]
//! retired the `data_clone_weight` / `data_clones` ranking modes:
//! weight means mass and nothing else. The `category` wire label and
//! the data-table policy are therefore gone.
//!
//! What this suite still pins, at full strength, on the mass-only wire:
//! - the distinct-value F# table family and the genuine byte-identical
//!   clone both publish, ranked by pure mass ([RANK-MASS-SUM]);
//! - the table family is byte-distinct (same shape, different values —
//!   never a copy-paste of itself) and the genuine clone is byte-proven;
//! - the retired `data_clones` / `data_clone_weight` knobs still parse
//!   (backwards compatibility) but must not change the report;
//! - the #190 verbatim escape hatch: a byte-for-byte copied table is
//!   proven duplication and is byte-proven like any copy.

use std::{collections::BTreeSet, path::PathBuf};

use anyhow::Result;
use serde_json::Value;

use crate::common::{
    corpora::*,
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    *,
};

/// Every distinct-value table file the #336 corpus stages. All four hold
/// the same 24-slot array shape and differ only in their values, so they
/// are one family and must publish as one.
const TABLE_FILES: [&str; 4] = ["tables_0.fs", "tables_1.fs", "tables_2.fs", "tables_3.fs"];

/// How many members the table family owes the report.
const TABLE_FILE_COUNT: usize = TABLE_FILES.len();

/// True for the distinct-value table files in the shared #336 corpus.
fn is_table_file(name: &str) -> bool {
    name.starts_with("tables_")
}

/// The files carried by the published table-family cluster, empty when
/// no cluster touches a table file at all.
fn table_family_files(report: &Value) -> BTreeSet<String> {
    rank_where(report, is_table_file)
        .and_then(|rank| clusters(report).get(rank).cloned())
        .map(|cluster| cluster_file_set(&cluster))
        .unwrap_or_default()
}

/// Renders the shared #336 corpus with an optional `.deslop.toml` body.
fn tables_report(config: Option<&str>) -> Result<(tempfile::TempDir, PathBuf, Value)> {
    let mut files = fsharp_tables_corpus();
    if let Some(body) = config {
        files.push((".deslop.toml".to_owned(), body.to_owned()));
    }
    report_for_with_root(&files, 20)
}

// [RANK-MASS-SUM] / #336: the distinct-value F# array family (four
// members) and the genuine two-copy logic clone both publish; mass
// ranks the four-member table family first (`24 × 3 = 72` vs
// `22 × 1 = 44`), and each cluster is byte-honest on the wire.
#[test]
fn fsharp_numeric_tables_and_clone_publish_ranked_by_mass() -> Result<()> {
    let (_workspace, root, report) = tables_report(None)?;
    let table = rank_where(&report, is_table_file);
    let clone = rank_where(&report, |name| name.starts_with("parse_"));
    assert!(
        table.is_some() && clone.is_some(),
        "both the table family and the genuine clone must publish: {report:#}"
    );
    assert_eq!(
        table_family_files(&report),
        TABLE_FILES
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<String>>(),
        "[RANK-MASS-SUM]: all {TABLE_FILE_COUNT} distinct-value tables are one shape-identical \
         family. Publishing a subset is a false negative — the members that share \
         no literal with any sibling are the ones dropped, and mass computed over \
         a truncated family cannot rank correctly: {report:#}"
    );
    assert!(
        clone > table,
        "[RANK-MASS-SUM]: the four-member table family (rank {table:?}) \
         out-ranks the two-copy clone (rank {clone:?}) by pure mass: {report:#}"
    );
    for cluster in clusters(&report) {
        let is_table = cluster_file_set(cluster)
            .iter()
            .any(|name| is_table_file(name));
        assert_eq!(
            has_verbatim_pair(&root, cluster)?,
            !is_table,
            "the table family is byte-distinct (same shape, different values) \
             and the clone is byte-proven — the byte truth must match which \
             cluster this is: {cluster:#}"
        );
        assert_structural_only_contract(cluster, "fsharp #336");
        assert_no_pair_surface_on_cluster(cluster, "fsharp #336");
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
