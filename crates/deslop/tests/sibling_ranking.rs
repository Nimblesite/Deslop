//! CLI regression tests for same-file sibling-window ranking.
//!
//! Implements [PIPELINE-RANK-WORST-FIRST]: overlapping sibling windows
//! in one physical file are not separate duplicate locations and must
//! not survive as ranked report clusters.

use std::{fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

/// Same-file overlap collapse must not leave singleton report rows.
#[test]
fn same_file_overlapping_sibling_windows_do_not_render_singletons() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_single_file_overlap_fixture(&scan_root)?;
    let report = run_and_load_report(tmp.path(), &scan_root)?;
    let clusters = report_clusters(&report);
    assert!(
        !clusters.is_empty(),
        "fixture must still contain real duplicate clusters"
    );
    let singletons = singleton_cluster_summaries(&clusters);
    assert!(
        singletons.is_empty(),
        "same-file overlapping sibling windows must collapse out of \
         ranked output, not render singleton duplicate clusters: {singletons:#?}"
    );
    Ok(())
}

/// Writes one C# file with three equivalent adjacent loops. The
/// sibling-window pass can form overlapping same-file windows, but
/// there is no second non-overlapping file/location to rank.
fn write_single_file_overlap_fixture(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join("Only.cs"),
        "namespace Solo\n\
         {\n\
         public class Worker\n\
         {\n\
         public int Run(int limit)\n\
         {\n\
         int first = 0;\n\
         for (int i = 0; i < limit; i = i + 1) { first = first + i; }\n\
         int second = 0;\n\
         for (int j = 0; j < limit; j = j + 1) { second = second + j; }\n\
         int third = 0;\n\
         for (int k = 0; k < limit; k = k + 1) { third = third + k; }\n\
         return first + second + third;\n\
         }\n\
         }\n\
         }\n",
    )?;
    Ok(())
}

/// Runs the CLI and parses the JSON report written under `tmp`.
fn run_and_load_report(tmp: &Path, scan_root: &Path) -> Result<Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(scan_root)
        .args(["--min-nodes", "4", "--output"])
        .arg(tmp.join("report"))
        .assert()
        .success();
    read_json_report(&report_json_path(tmp))
}

/// Returns the `<tmp>/report.json` path used by the CLI run.
fn report_json_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _changed = path.set_extension("json");
    path
}

/// Reads one JSON report file.
fn read_json_report(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

/// Returns rendered clusters, or an empty vector if the report omits
/// the field.
fn report_clusters(report: &Value) -> Vec<Value> {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Finds report rows that cannot represent a duplicate because they
/// have fewer than two logical locations after overlap collapse.
fn singleton_cluster_summaries(clusters: &[Value]) -> Vec<String> {
    clusters
        .iter()
        .filter(|cluster| cluster_size(cluster) < 2 || occurrence_count(cluster) < 2)
        .map(cluster_summary)
        .collect()
}

/// Reads the cluster's visible membership — `occurrence_count` on the
/// mass-only wire (the `size` field was removed with the bucket surface).
fn cluster_size(cluster: &Value) -> u64 {
    cluster
        .get("occurrence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// Reads `cluster.occurrences_total`, defaulting to zero for malformed
/// rows.
fn occurrence_count(cluster: &Value) -> u64 {
    cluster
        .get("occurrences_total")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// Formats a compact failure line for one bad cluster.
/// One-line summary for a singleton row.
fn cluster_summary(cluster: &Value) -> String {
    format!(
        "id={id} count={count} occurrences_total={total}",
        id = cluster.get("id").and_then(Value::as_str).unwrap_or("?"),
        count = cluster_size(cluster),
        total = cluster
            .get("occurrences_total")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    )
}

/// Reads a numeric field for failure output.
fn _unused_number_field(value: &Value, name: &str) -> f64 {
    value.get(name).and_then(Value::as_f64).unwrap_or(0.0)
}
