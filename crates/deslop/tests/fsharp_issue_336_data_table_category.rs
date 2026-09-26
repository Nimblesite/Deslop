//! F# data tables must not outrank genuine clones merely because their shapes match.
//! [CLONE-NOISE-LITERAL-TABLE] [FUSED-CONTENT-GATE] [RANK-MASS-SUM]
//!
//! The byte-identical clone leads the report. Tables with disjoint values cannot
//! form a clone; any shape-only findings have zero mass and no rank. Tables
//! admitted as clones must have content support and rank below the genuine copy.
//! Retired data-clone settings do not change the report. Copied tables remain clones.

use std::path::{Path, PathBuf};

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

/// [ACCURACY-RECOVERY-PLAN-LITERAL-TABLE] Distinct HTTP method names and
/// World Bank payloads share only the grammar of module-level bindings.
const UNRELATED_BINDING_FILES: [&str; 2] = ["http_methods.fs", "world_bank_mocks.fs"];
const BINDING_MIN_NODES: u32 = 20;
const BINDING_FILE_COUNT: u64 = 4;
const COPIED_OCCURRENCES: u64 = 2;
const NO_DUPLICATED_LINES: u64 = 0;
const HTTP_METHOD_BINDINGS: &str = r#"let PropFind = "PROPFIND"
/// Updates the properties on an HTTP resource.
let PropPatch = "PROPPATCH"
/// Creates a collection resource.
let MkCol = "MKCOL"
/// Copies an HTTP resource.
let Copy = "COPY"
/// Moves an HTTP resource.
let Move = "MOVE"
/// Acquires an HTTP resource lock.
let Lock = "LOCK"
/// Releases an HTTP resource lock.
let Unlock = "UNLOCK"
/// Applies partial changes to a resource.
let Patch = "PATCH"
"#;
const WORLD_BANK_MOCKS: &str = r#"module WorldBankMocks

let mockIndicatorResponse = """
[
  { "page": 1, "pages": 1, "per_page": 1000, "total": 2 },
  [
    { "id": "AG.AGR.TRAC.NO", "name": "Agricultural machinery, tractors",
      "source": { "id": "2", "value": "World Development Indicators" },
      "topics": [{ "id": "9", "value": "Infrastructure" }] },
    { "id": "AG.CON.FERT.ZS", "name": "Fertilizer consumption",
      "source": { "id": "2", "value": "World Development Indicators" },
      "topics": [{ "id": "6", "value": "Environment" }] }
  ]
]
"""

let mockCountryResponse = """
[
  { "page": 1, "pages": 1, "per_page": 1000, "total": 3 },
  [
    { "id": "ABW", "name": "Aruba", "capitalCity": "Oranjestad" },
    { "id": "AFG", "name": "Afghanistan", "capitalCity": "Kabul" }
  ]
]
"""

let mockDataResponse = """
[
  { "page": 1, "pages": 1, "per_page": 1000, "total": 3 },
  [
    { "date": "2020", "value": "65894.86" },
    { "date": "2019", "value": "62888.17" },
    { "date": "2018", "value": null }
  ]
]
"""
"#;

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

/// [CLONE-NOISE-CONSTANT-TABLE] [ACCURACY-RECOVERY-PLAN-LITERAL-TABLE]
/// A long mock payload and unrelated HTTP
/// string bindings have no reusable logic; the copied function beside them
/// proves the F# analyser still publishes real duplicates.
#[test]
fn unrelated_fsharp_literal_bindings_stay_out_of_report() -> Result<()> {
    let (_workspace, root, report) = literal_binding_report()?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(BINDING_FILE_COUNT)
    );
    assert_copied_fsharp_function(&root, &report)?;
    assert_unrelated_bindings_absent(&report);
    Ok(())
}

/// The two unrelated data families and copied-function control share a scan.
fn literal_binding_report() -> Result<(tempfile::TempDir, PathBuf, Value)> {
    let mut files = vec![
        (
            UNRELATED_BINDING_FILES[0].to_owned(),
            http_methods("HttpMethods"),
        ),
        (
            UNRELATED_BINDING_FILES[1].to_owned(),
            WORLD_BANK_MOCKS.to_owned(),
        ),
    ];
    files.extend(genuine_pair(
        CLONE_FILES[0],
        CLONE_FILES[1],
        FSHARP_GENUINE_CLONE,
    ));
    report_for_with_root(&files, BINDING_MIN_NODES)
}

/// Builds one F# module from shared HTTP method bindings.
fn http_methods(module_name: &str) -> String {
    format!("module {module_name}\n\n{HTTP_METHOD_BINDINGS}")
}

/// [CLONE-NOISE-CONSTANT-TABLE] [ACCURACY-RECOVERY-PLAN-LITERAL-TABLE]
/// A copied table is real duplication even
/// when its enclosing module name changes.
#[test]
fn copied_fsharp_literal_bindings_survive_module_rename() -> Result<()> {
    const COPY_FILES: [&str; 2] = ["methods_a.fs", "methods_b.fs"];
    const OTHER_FILE: &str = "world_bank_mocks.fs";
    const EXPECTED_FILES: u64 = 3;
    let files = [
        (COPY_FILES[0].to_owned(), http_methods("FirstMethods")),
        (COPY_FILES[1].to_owned(), http_methods("SecondMethods")),
        (OTHER_FILE.to_owned(), WORLD_BANK_MOCKS.to_owned()),
    ];
    let (_workspace, _root, report) = report_for_with_root(&files, BINDING_MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(EXPECTED_FILES)
    );
    let copy = expect_cluster_spanning(&report, &COPY_FILES)?;
    assert_eq!(cluster_size(copy), COPIED_OCCURRENCES, "{copy:#}");
    assert!(!cluster_file_set(copy).contains(OTHER_FILE), "{copy:#}");
    for path in COPY_FILES {
        assert!(duplicated_loc_for(&report, path) > 0, "{path}: {report:#}");
    }
    assert_eq!(duplicated_loc_for(&report, OTHER_FILE), NO_DUPLICATED_LINES);
    Ok(())
}

/// A genuine byte-identical function must remain visible and counted.
fn assert_copied_fsharp_function(root: &Path, report: &Value) -> Result<()> {
    let copy = expect_cluster_spanning(report, &CLONE_FILES)?;
    assert_eq!(
        cluster_size(copy),
        COPIED_OCCURRENCES,
        "both copies: {copy:#}"
    );
    assert!(has_verbatim_pair(root, copy)?, "copied function: {copy:#}");
    for path in CLONE_FILES {
        assert!(duplicated_loc_for(report, path) > 0, "{path}: {report:#}");
    }
    Ok(())
}

/// Neither unrelated table may surface in a visible cluster or LOC metric.
fn assert_unrelated_bindings_absent(report: &Value) {
    for cluster in clusters(report) {
        let paths = cluster_file_set(cluster);
        assert!(
            UNRELATED_BINDING_FILES
                .iter()
                .all(|path| !paths.contains(*path)),
            "unrelated literal bindings share no extractable code: {cluster:#}"
        );
    }
    for path in UNRELATED_BINDING_FILES {
        assert_eq!(
            duplicated_loc_for(report, path),
            NO_DUPLICATED_LINES,
            "{path}: {report:#}"
        );
    }
}

/// [CLONE-NOISE-LITERAL-TABLE] [RANK-MASS-SUM] The genuine clone leads the report.
/// [FUSED-CONTENT-GATE] Tables sharing no literal cannot form a clone.
#[test]
fn fsharp_numeric_tables_and_clone_publish_ranked_by_mass() -> Result<()> {
    let (_workspace, root, report) = tables_report(None)?;
    let clone = expect_cluster_spanning(&report, &CLONE_FILES)?;
    assert_eq!(
        clusters(&report).first(),
        Some(clone),
        "the genuine clone precedes every informational finding"
    );
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

/// Tables are byte-distinct. Informational findings have zero weight; table clones
/// rank below the genuine copy and never join tables that share no value.
fn assert_table_cluster_is_honest(root: &Path, cluster: &Value) -> Result<()> {
    let is_clone = crate::common::findings::is_clone_finding(cluster);
    assert!(
        !has_verbatim_pair(root, cluster)?,
        "the table family is byte-distinct — same shape, different values — \
         and must not read as a copy: {cluster:#}"
    );
    if !is_clone {
        return Ok(());
    }
    assert!(
        field(cluster, "rank").as_u64() > Some(CLONE_RANK),
        "[CLONE-NOISE-LITERAL-TABLE]: a distinct-value table family \
         ranking at or above the genuine clone is the reported defect: {cluster:#}"
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
