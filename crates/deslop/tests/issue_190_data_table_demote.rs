//! Regression coverage for #190 [RANK-CATEGORY] /
//! [CLONE-NOISE-DART-DATA-TABLE-LITERAL]: a top-level Dart collection
//! literal of near-identical constructor rows (`List<HighlightData> = [
//! HighlightData(...), … ]`) is un-refactorable *data*, not duplicated
//! *logic*. Before #190 it dominated the ranking and buried genuine
//! copy-pasted logic clones. The three-way `[ranking] data_clones` policy
//! demotes (default), drops (`ignore`), or restores (`keep` /
//! `data_clone_weight = 1.0`) such tables, and labels them `category="data"`.
//!
//! Black-box E2E: drive the CLI against fixture repos and assert against the
//! rendered JSON / text reports only.

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

/// The hand-written copy-pasted logic clone, duplicated verbatim across two
/// files. Six-statement method body, large enough to cluster at the test's
/// `--min-nodes` floor.
const DUPLICATED_METHOD: &str = "  int score(int v) {\n    final w = v * 3;\n    \
     final z = w + v;\n    final q = z - w;\n    final r = q * z;\n    \
     return r + w - v;\n  }\n";

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

fn cluster_for<'a>(report: &'a Value, file_name: &str) -> Option<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| cluster_touches(cluster, file_name))
}

/// Zero-based rank of the cluster touching `file_name`, or [`usize::MAX`]
/// when no cluster does — callers assert presence with `< usize::MAX`.
fn rank_of(report: &Value, file_name: &str) -> usize {
    clusters(report)
        .iter()
        .position(|cluster| cluster_touches(cluster, file_name))
        .unwrap_or(usize::MAX)
}

fn category_of(report: &Value, file_name: &str) -> String {
    cluster_for(report, file_name)
        .and_then(|cluster| cluster.get("category"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn weight_of(report: &Value, file_name: &str) -> f64 {
    cluster_for(report, file_name)
        .and_then(|cluster| cluster.get("weight"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

/// Writes a fixture repo with a data table (≥3 differing constructor rows in
/// a top-level `List`) plus a genuinely copy-pasted method across two files.
fn write_fixture(src: &Path) -> Result<()> {
    fs::create_dir_all(src)?;
    let mut table = String::from(
        "class HighlightData {\n  const HighlightData({required this.title, \
         required this.wonder, required this.artifactId, required this.culture, \
         required this.date});\n  final String title;\n  final String wonder;\n  \
         final String artifactId;\n  final String culture;\n  final String date;\n}\n\n\
         const List<HighlightData> highlights = [\n",
    );
    let rows: &[(&str, &str, &str, &str)] = &[
        ("Pyramid", "Giza", "Egyptian", "2560 BC"),
        ("Great Wall", "China", "Chinese", "700 BC"),
        ("Petra", "Jordan", "Nabataean", "312 BC"),
        ("Colosseum", "Rome", "Roman", "80 AD"),
        ("Machu Picchu", "Peru", "Inca", "1450 AD"),
        ("Taj Mahal", "India", "Mughal", "1653 AD"),
        ("Chichen Itza", "Mexico", "Mayan", "600 AD"),
        ("Christ Redeemer", "Brazil", "Brazilian", "1931 AD"),
    ];
    for (index, (title, wonder, culture, date)) in rows.iter().enumerate() {
        let _ = writeln!(
            table,
            "  HighlightData(title: \"{title}\", wonder: \"{wonder}\", \
             artifactId: \"a{index}\", culture: \"{culture}\", date: \"{date}\"),"
        );
    }
    table.push_str("];\n");
    fs::write(src.join("highlight_data.dart"), table)?;

    fs::write(
        src.join("scorer_a.dart"),
        format!("class ScorerA {{\n{DUPLICATED_METHOD}}}\n"),
    )?;
    fs::write(
        src.join("scorer_b.dart"),
        format!("class ScorerB {{\n{DUPLICATED_METHOD}}}\n"),
    )?;
    Ok(())
}

/// Runs the CLI against `src`, writing JSON (and text for the label
/// assertion) to `<tmp>/<stem>.*`, and returns the parsed JSON report.
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

#[test]
fn default_demotes_data_table_below_logic_clone() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    write_fixture(&src)?;

    // [RANK-CATEGORY] assertion 1 (core proof): with the DEFAULT policy the
    // logic clone outranks the data table even though the table has a larger
    // raw size and node count.
    let report = run_cli(&src, tmp.path(), "default", "30")?;
    let logic_rank = rank_of(&report, "scorer_a.dart");
    let data_rank = rank_of(&report, "highlight_data.dart");
    assert!(
        logic_rank < usize::MAX,
        "logic clone must appear: {report:#}"
    );
    assert!(data_rank < usize::MAX, "data table must appear: {report:#}");
    assert!(
        logic_rank < data_rank,
        "default demote must rank the logic clone above the data table: \
         logic at #{logic_rank}, data at #{data_rank}\n{report:#}"
    );

    // [RANK-CATEGORY] assertion 2: the data cluster is labelled category=data;
    // the logic cluster stays category=logic.
    assert_eq!(
        category_of(&report, "highlight_data.dart"),
        "data",
        "data table must be labelled data: {report:#}"
    );
    assert_eq!(
        category_of(&report, "scorer_a.dart"),
        "logic",
        "logic clone must stay logic: {report:#}"
    );
    let logic_weight = weight_of(&report, "scorer_a.dart");
    let data_weight = weight_of(&report, "highlight_data.dart");
    assert!(
        logic_weight > data_weight,
        "demoted data weight must sit below the logic weight: \
         logic={logic_weight}, data={data_weight}"
    );

    // The text report carries the human-readable "data table" chip and the
    // builder / asset action sentence, so the demotion is visible to humans
    // and not just an AI label.
    let text = fs::read_to_string(tmp.path().join("default.txt"))?;
    assert!(
        text.contains("[data table]"),
        "text report must carry the data-table chip: {text}"
    );
    assert!(
        text.contains("move the rows to a JSON/CSV/asset"),
        "text report must carry the builder/asset action hint for data: {text}"
    );
    Ok(())
}

#[test]
fn ignore_mode_drops_data_table_keeps_logic_clone() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    write_fixture(&src)?;
    write_ranking_config(&src, "[ranking]\ndata_clones = \"ignore\"\n")?;

    // [RANK-CATEGORY] assertion 3: ignore mode drops the data table entirely
    // (it appears nowhere in the report) while the logic clone survives.
    let report = run_cli(&src, tmp.path(), "ignore", "30")?;
    assert!(
        !touches(&report, "highlight_data.dart"),
        "ignore mode must drop the data table from the report: {report:#}"
    );
    assert!(
        touches(&report, "scorer_a.dart"),
        "the logic clone must still appear under ignore mode: {report:#}"
    );
    assert_eq!(
        category_of(&report, "scorer_a.dart"),
        "logic",
        "surviving logic clone stays logic: {report:#}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the dropped data table must be counted under clusters_hidden: {report:#}"
    );
    Ok(())
}

#[test]
fn keep_mode_restores_data_table_to_the_top() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    write_fixture(&src)?;
    write_ranking_config(&src, "[ranking]\ndata_clone_weight = 1.0\n")?;

    // [RANK-CATEGORY] assertion 4: data_clone_weight = 1.0 restores the prior
    // ordering — the larger data table tops the ranking again — proving the
    // knob actually drives the weight. The category label is unchanged.
    let report = run_cli(&src, tmp.path(), "keep", "30")?;
    let data_rank = rank_of(&report, "highlight_data.dart");
    let logic_rank = rank_of(&report, "scorer_a.dart");
    assert!(data_rank < usize::MAX, "data table must appear: {report:#}");
    assert!(
        logic_rank < usize::MAX,
        "logic clone must appear: {report:#}"
    );
    assert!(
        data_rank < logic_rank,
        "data_clone_weight = 1.0 must let the data table out-rank the logic clone: \
         data at #{data_rank}, logic at #{logic_rank}\n{report:#}"
    );
    assert_eq!(
        category_of(&report, "highlight_data.dart"),
        "data",
        "data table stays labelled data even at full weight: {report:#}"
    );
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
    fs::create_dir_all(&src)?;

    // A data TABLE copied verbatim across two files. The escape hatch
    // ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]) requires ≥2 cluster members to
    // differ in raw bytes; a byte-for-byte cross-file copy does NOT, so the
    // cluster stays category=logic at full weight — genuine copy-paste.
    let table = "class Cfg {\n  const Cfg({this.a, this.b, this.c});\n  \
        final String a;\n  final int b;\n  final String c;\n}\n\n\
        const List<Cfg> table = [\n  Cfg(a: \"x1\", b: 1, c: \"p\"),\n  \
        Cfg(a: \"x2\", b: 2, c: \"q\"),\n  Cfg(a: \"x3\", b: 3, c: \"r\"),\n  \
        Cfg(a: \"x4\", b: 4, c: \"s\"),\n  Cfg(a: \"x5\", b: 5, c: \"t\"),\n  \
        Cfg(a: \"x6\", b: 6, c: \"u\"),\n];\n";
    fs::write(src.join("config_one.dart"), table)?;
    fs::write(src.join("config_two.dart"), table)?;

    // [RANK-CATEGORY] assertion 5: the verbatim-copied table is NOT demoted —
    // it surfaces as a genuine cross-file duplicate, category=logic.
    let report = run_cli(&src, tmp.path(), "verbatim", "20")?;
    let spans_both = clusters(&report).iter().any(|cluster| {
        cluster_touches(cluster, "config_one.dart") && cluster_touches(cluster, "config_two.dart")
    });
    assert!(
        spans_both,
        "the cross-file verbatim copy must cluster across both files: {report:#}"
    );
    assert_eq!(
        category_of(&report, "config_one.dart"),
        "logic",
        "a verbatim-copied table is real duplication and must NOT be demoted to data: {report:#}"
    );
    Ok(())
}
