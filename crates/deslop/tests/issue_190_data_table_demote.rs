//! Regression coverage for #190 [RANK-CATEGORY] /
//! [CLONE-NOISE-DART-DATA-TABLE-LITERAL]: a top-level Dart collection
//! literal of near-identical constructor rows (`List<HighlightData> = [
//! HighlightData(...), … ]`) is un-refactorable *data*, not duplicated
//! *logic*. Before #190 it dominated the ranking and buried genuine
//! copy-pasted logic clones.
//!
//! The mass-only wire retired the `demote` ranking mode ([RANK-CATEGORY]:
//! "the retired `demote`, `data_clone_weight`, and every
//! evidence-weighted `[metrics]` table are forbidden") and the content
//! gate rejects shape-only same-file pairs below the promote floor
//! ([FUSED-CONTENT-GATE]). The data-table family therefore publishes
//! nothing on the current wire while the byte-proven logic clone keeps
//! its rank — the two assertions this suite pins end to end.
//!
//! Black-box E2E: drive the CLI against fixture repos and assert against the
//! rendered JSON reports only.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use deslop_test_support::write_dart_data_table_fixture;
use serde_json::Value;

use crate::common::signals::{assert_no_pair_surface_on_cluster, has_verbatim_pair};
use crate::common::*;

fn report_path(tmp: &Path, stem: &str) -> PathBuf {
    let mut path = tmp.join(stem);
    let _replaced = path.set_extension("json");
    path
}

fn cluster_touches(cluster: &Value, file_name: &str) -> bool {
    occurrence_paths(cluster)
        .iter()
        .any(|path| path.ends_with(file_name))
}

fn touches(report: &Value, file_name: &str) -> bool {
    clusters(report)
        .iter()
        .any(|cluster| cluster_touches(cluster, file_name))
}

/// Runs the CLI against `src`, writing JSON to `<tmp>/<stem>.json`, and
/// returns the parsed JSON report.
fn run_cli(src: &Path, tmp: &Path, stem: &str, min_nodes: &str) -> Result<Value> {
    let mut cmd = deslop_cmd(src, &tmp.join(stem))?;
    let _assertion = cmd
        .args(["--min-nodes", min_nodes, "--embeddings", "off", "--nohtml"])
        .assert()
        .success();
    let body = fs::read_to_string(report_path(tmp, stem))?;
    Ok(serde_json::from_str(&body)?)
}

fn write_ranking_config(src: &Path, body: &str) -> Result<()> {
    fs::write(src.join(".deslop.toml"), body)?;
    Ok(())
}

/// The byte-proven logic clone must head the report and the data-table
/// family must publish nothing — the mass-only contract for every
/// `data_clones` mode, since the content gate decides admission and no
/// evidence factor may change rank ([RANK-MASS-SUM]).
fn assert_logic_clone_leads_and_table_is_absent(report: &Value, scan_root: &Path) -> Result<()> {
    let logic = clusters(report)
        .iter()
        .find(|cluster| cluster_touches(cluster, "scorer_a.dart"))
        .ok_or_else(|| anyhow::anyhow!("logic clone must appear: {report:#}"))?;
    assert_eq!(
        clusters(report)
            .iter()
            .position(|cluster| std::ptr::eq(cluster, logic)),
        Some(0),
        "the byte-proven logic clone must rank first: {report:#}"
    );
    // The reported view spans the whole file including the differing class
    // name, so the byte-proven *method* shows as the occurrence text rather
    // than as a verbatim cluster; the admission + membership facts are the
    // wire contract ([PIPELINE-CLUSTER-CLOSURE]).
    let texts = occurrence_texts(scan_root, logic)?;
    assert!(
        texts.len() >= 2 && texts.iter().all(|text| text.contains("score(int v)")),
        "the logic clone's duplicated method must be reported in both occurrences: {texts:#?}"
    );
    assert_no_pair_surface_on_cluster(logic, "issue #190 logic clone");
    assert!(
        !touches(report, "highlight_data.dart"),
        "the data-table family must publish no cluster — the content gate \
         rejects its shape-only rows below the promote floor \
         ([FUSED-CONTENT-GATE]): {report:#}"
    );
    Ok(())
}

#[test]
fn default_mode_ranks_logic_clone_first_and_publishes_no_data_table() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    let scan_root = src.clone();
    write_dart_data_table_fixture(&src)?;

    // [RANK-CATEGORY] core proof: with the DEFAULT policy the byte-proven
    // logic clone outranks the data table, and the table publishes
    // nothing — `demote` is retired and no evidence factor may change
    // mass.
    let report = run_cli(&src, tmp.path(), "default", "30")?;
    assert_logic_clone_leads_and_table_is_absent(&report, &scan_root)?;
    Ok(())
}

#[test]
fn ignore_mode_drops_data_table_keeps_logic_clone() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    let scan_root = src.clone();
    write_dart_data_table_fixture(&src)?;
    write_ranking_config(&src, "[visibility]\ndata_clones = \"ignore\"\n")?;

    // [RANK-CATEGORY] ignore mode: the data table appears nowhere and the
    // byte-proven logic clone survives at the top.
    let report = run_cli(&src, tmp.path(), "ignore", "30")?;
    assert_logic_clone_leads_and_table_is_absent(&report, &scan_root)?;
    Ok(())
}

#[test]
fn keep_mode_keeps_logic_clone_first_and_publishes_no_data_table() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    let scan_root = src.clone();
    write_dart_data_table_fixture(&src)?;
    write_ranking_config(&src, "[visibility]\ndata_clones = \"keep\"\n")?;

    // [RANK-CATEGORY] keep mode: the table's fate is admission's, and
    // admission rejects it below the promote floor; the logic clone keeps
    // rank one.
    let report = run_cli(&src, tmp.path(), "keep", "30")?;
    assert_logic_clone_leads_and_table_is_absent(&report, &scan_root)?;
    Ok(())
}

#[test]
fn invalid_data_clone_weight_is_rejected_with_a_clear_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.dart"), "class A { int x = 1; }\n")?;

    // [RANK-CATEGORY] config validation: an out-of-range multiplier fails the
    // run with a ConfigThreshold-style diagnostic naming the offending key
    // and the accepted range — never a silent default.
    for (body, needle) in [
        ("[ranking]\ndata_clone_weight = 2.5\n", "range (0.0, 1.0]"),
        ("[ranking]\ndata_clone_weight = 0.0\n", "range (0.0, 1.0]"),
        ("[ranking]\ndata_clone_weight = nan\n", "must be finite"),
    ] {
        write_ranking_config(&src, body)?;
        let mut cmd = deslop_cmd(&src, &tmp.path().join("r"))?;
        let _assertion = cmd
            .args([
                "--min-nodes",
                "30",
                "--embeddings",
                "off",
                "--notext",
                "--nohtml",
            ])
            .assert()
            .failure()
            .stderr(predicates::str::contains("data_clone_weight"))
            .stderr(predicates::str::contains(needle));
    }
    Ok(())
}

#[test]
fn verbatim_copied_table_still_surfaces_as_duplication() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    let scan_root = src.clone();
    fs::create_dir_all(&src)?;

    // A data TABLE copied verbatim across two files. The escape hatch
    // ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]) requires ≥2 cluster members to
    // differ in raw bytes; a byte-for-byte cross-file copy does NOT, so the
    // cluster stays a real duplicate at full mass — genuine copy-paste.
    let table = "class Cfg {\n  const Cfg({this.a, this.b, this.c});\n  \
        final String a;\n  final int b;\n  final String c;\n}\n\n\
        const List<Cfg> table = [\n  Cfg(a: \"x1\", b: 1, c: \"p\"),\n  \
        Cfg(a: \"x2\", b: 2, c: \"q\"),\n  Cfg(a: \"x3\", b: 3, c: \"r\"),\n  \
        Cfg(a: \"x4\", b: 4, c: \"s\"),\n  Cfg(a: \"x5\", b: 5, c: \"t\"),\n  \
        Cfg(a: \"x6\", b: 6, c: \"u\"),\n];\n";
    fs::write(src.join("config_one.dart"), table)?;
    fs::write(src.join("config_two.dart"), table)?;

    // [RANK-CATEGORY] the verbatim-copied table is NOT dropped — it
    // surfaces as a genuine byte-proven cross-file duplicate.
    let report = run_cli(&src, tmp.path(), "verbatim", "20")?;
    let cluster = clusters(&report)
        .iter()
        .find(|cluster| {
            cluster_touches(cluster, "config_one.dart")
                && cluster_touches(cluster, "config_two.dart")
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "the cross-file verbatim copy must cluster across both files: {report:#}"
            )
        })?;
    assert!(
        has_verbatim_pair(&scan_root, cluster)?,
        "a verbatim-copied table is real duplication and must be byte-proven: {cluster:#}"
    );
    Ok(())
}
