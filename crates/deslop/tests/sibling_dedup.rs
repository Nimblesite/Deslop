//! End-to-end regression coverage for the phantom-occurrences bug
//! (tracker issue #2: sibling-extension runaway).
//!
//! The sibling-window pass at [`deslop_core::sibling`] emits one
//! fingerprint per contiguous sibling window of widths 2..=8. When a
//! physical clone spans enough siblings, multiple windows cover
//! overlapping byte ranges inside the same file. Before the fix those
//! nested windows all survived as distinct members of a single
//! cluster, producing:
//!
//! - thousands of logical "occurrences" inside files of a few dozen
//!   lines;
//! - `cluster.size` and ranking weight inflated by the width-fanout;
//! - occurrence byte ranges whose `end_byte` exceeded the source
//!   file's length;
//! - `cluster-by-id` payloads in the megabytes for a cluster
//!   representing a single physical repetition.
//!
//! These tests drive the `deslop` binary against fixtures whose exact
//! clone topology is known and assert the rendered JSON report carries
//! exactly the right number of occurrences, with every byte range
//! inside its source file and no two occurrences overlapping in the
//! same file. Each assertion maps to a specific claim in NAP's bug
//! report.

use std::{fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;

/// Writes two C# files that each contain three near-identical `for`
/// loops nested inside a single method. The sibling pass emits window
/// widths 2, 3, … over those three contiguous loops and — without
/// dedup — every window survives as a separate member of the same
/// cluster.
fn write_nested_clone_fixture(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let alpha = "namespace Alpha\n\
                 {\n\
                 public class Runner\n\
                 {\n\
                 public int Run(int input)\n\
                 {\n\
                 if (input < 0) { return 0; }\n\
                 int total = 0;\n\
                 for (int i = 0; i < input; i = i + 1) { total = total + i; }\n\
                 int doubled = 0;\n\
                 for (int j = 0; j < input; j = j + 1) { doubled = doubled + j; }\n\
                 int tripled = 0;\n\
                 for (int k = 0; k < input; k = k + 1) { tripled = tripled + k; }\n\
                 return total + doubled + tripled;\n\
                 }\n\
                 }\n\
                 }\n";
    let beta = "namespace Beta\n\
                {\n\
                public class Worker\n\
                {\n\
                public int Work(int limit)\n\
                {\n\
                if (limit < 0) { return 0; }\n\
                int sum = 0;\n\
                for (int a = 0; a < limit; a = a + 1) { sum = sum + a; }\n\
                int twice = 0;\n\
                for (int b = 0; b < limit; b = b + 1) { twice = twice + b; }\n\
                int thrice = 0;\n\
                for (int c = 0; c < limit; c = c + 1) { thrice = thrice + c; }\n\
                return sum + twice + thrice;\n\
                }\n\
                }\n\
                }\n";
    fs::write(dir.join("Alpha.cs"), alpha)?;
    fs::write(dir.join("Beta.cs"), beta)?;
    Ok(())
}

/// Runs the CLI against `scan_root`, writing reports under
/// `<tmp>/report.*`, and returns the parsed JSON report.
fn run_and_load_report(tmp: &Path, scan_root: &Path) -> Result<serde_json::Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(scan_root)
        .arg("--min-nodes")
        .arg("4")
        .arg("--output")
        .arg(tmp.join("report"))
        .assert()
        .success();
    let json_path: PathBuf = {
        let mut path = tmp.join("report");
        let _replaced = path.set_extension("json");
        path
    };
    let body = fs::read_to_string(&json_path)?;
    Ok(serde_json::from_str(&body)?)
}

/// Returns `true` when the half-open ranges `[a_start, a_end)` and
/// `[b_start, b_end)` share at least one byte.
fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
    a_start < b_end && b_start < a_end
}

/// Scans every cluster in `report` and returns the first pair of
/// occurrences inside the same cluster and file whose byte ranges
/// overlap. Returns `None` when every cluster is clean.
fn first_overlap(report: &serde_json::Value) -> Option<String> {
    let clusters = report.get("clusters")?.as_array()?;
    for cluster in clusters {
        if let Some(pair) = first_overlap_in_cluster(cluster) {
            return Some(pair);
        }
    }
    None
}

/// Inner loop of [`first_overlap`]: reports the first overlapping pair
/// inside a single cluster, annotated with the cluster id for the
/// failure message.
fn first_overlap_in_cluster(cluster: &serde_json::Value) -> Option<String> {
    let occurrences = cluster.get("occurrences")?.as_array()?;
    let id = cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    for (index, left) in occurrences.iter().enumerate() {
        for right in occurrences.iter().skip(index.saturating_add(1)) {
            if let Some(pair) = overlap_pair(id, left, right) {
                return Some(pair);
            }
        }
    }
    None
}

/// Returns a human-readable description when `left` and `right` refer
/// to the same file and have overlapping byte ranges.
fn overlap_pair(
    cluster_id: &str,
    left: &serde_json::Value,
    right: &serde_json::Value,
) -> Option<String> {
    let left_path = left.get("path")?.as_str()?;
    let right_path = right.get("path")?.as_str()?;
    if left_path != right_path {
        return None;
    }
    let left_start = left.get("start_byte")?.as_u64()?;
    let left_end = left.get("end_byte")?.as_u64()?;
    let right_start = right.get("start_byte")?.as_u64()?;
    let right_end = right.get("end_byte")?.as_u64()?;
    if !ranges_overlap(left_start, left_end, right_start, right_end) {
        return None;
    }
    Some(format!(
        "cluster {cluster_id} has overlapping occurrences in {left_path}: \
         [{left_start}, {left_end}) and [{right_start}, {right_end})"
    ))
}

/// Scans the whole report and returns the first occurrence whose
/// `end_byte` exceeds the length of the file it names. Returns `None`
/// when every occurrence is inside its source file's bounds.
fn first_out_of_bounds(report: &serde_json::Value, scan_root: &Path) -> Option<String> {
    let clusters = report.get("clusters")?.as_array()?;
    for cluster in clusters {
        if let Some(pair) = first_out_of_bounds_in_cluster(cluster, scan_root) {
            return Some(pair);
        }
    }
    None
}

/// Per-cluster walker for [`first_out_of_bounds`].
fn first_out_of_bounds_in_cluster(
    cluster: &serde_json::Value,
    scan_root: &Path,
) -> Option<String> {
    let occurrences = cluster.get("occurrences")?.as_array()?;
    let id = cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    for occurrence in occurrences {
        if let Some(report_line) = out_of_bounds_line(id, occurrence, scan_root) {
            return Some(report_line);
        }
    }
    None
}

/// Produces a failure description when `occurrence.end_byte` exceeds
/// the byte length of the referenced file on disk.
fn out_of_bounds_line(
    cluster_id: &str,
    occurrence: &serde_json::Value,
    scan_root: &Path,
) -> Option<String> {
    let path = occurrence.get("path")?.as_str()?;
    let start = occurrence.get("start_byte")?.as_u64()?;
    let end = occurrence.get("end_byte")?.as_u64()?;
    let absolute = scan_root.join(path);
    let file_len = fs::metadata(&absolute).ok()?.len();
    if end <= file_len && start <= file_len {
        return None;
    }
    Some(format!(
        "cluster {cluster_id}: occurrence in {path} reports byte range \
         [{start}, {end}) but the file is only {file_len} bytes long"
    ))
}

/// Counts how many `occurrences` a cluster reports inside each
/// `path` (returns a sorted `(path, count)` list so assertions have a
/// deterministic message).
fn per_file_counts(cluster: &serde_json::Value) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    if let Some(occurrences) = cluster.get("occurrences").and_then(serde_json::Value::as_array) {
        for occurrence in occurrences {
            if let Some(path) = occurrence.get("path").and_then(serde_json::Value::as_str) {
                let entry = counts.entry(path.to_owned()).or_insert(0_usize);
                *entry = entry.saturating_add(1);
            }
        }
    }
    counts.into_iter().collect()
}

// Regression for NAP's "overlapping byte ranges inside the same file"
// claim. With the fix in place, no cluster may contain two
// occurrences that point at the same file and overlap.
#[test]
fn clusters_never_contain_overlapping_occurrences_in_same_file() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_nested_clone_fixture(&scan_root)?;
    let report = run_and_load_report(tmp.path(), &scan_root)?;
    assert!(
        first_overlap(&report).is_none(),
        "cluster occurrences must be deduplicated per file before rendering: {}",
        first_overlap(&report).unwrap_or_default()
    );
    Ok(())
}

// Regression for NAP's "26,464 occurrences in a 53-line file" claim
// and the broader cluster-size inflation symptom. With three
// near-identical sibling statements per file, the pre-fix
// sibling-window pass emitted contiguous windows of widths 2 and 3,
// so every cluster that tracked those windows carried several
// overlapping members per file. After the fix, `size` equals
// `occurrences.len()` exactly — no duplicate members hiding behind
// the same cluster id.
#[test]
fn cluster_size_equals_occurrences_length_for_every_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_nested_clone_fixture(&scan_root)?;
    let report = run_and_load_report(tmp.path(), &scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !clusters.is_empty(),
        "fixture must produce at least one clone cluster: {report:#}"
    );
    for cluster in &clusters {
        let size = cluster
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let occurrences_len: u64 = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .and_then(|len| u64::try_from(len).ok())
            .unwrap_or(0);
        let occurrences_total = cluster
            .get("occurrences_total")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        assert_eq!(
            size, occurrences_len,
            "cluster.size must equal occurrences.len() after overlap dedup: {cluster:#}"
        );
        assert_eq!(
            occurrences_total, occurrences_len,
            "occurrences_total must equal occurrences.len(): {cluster:#}"
        );
    }
    Ok(())
}

// Regression for NAP's "occurrence byte range (621, 2588) inside an
// 81-line / 2589-byte file" evidence, and for the broader "phantom
// lines that don't exist in files" claim. Every rendered occurrence
// must have `end_byte <= file_size` — otherwise the report is pointing
// at bytes that aren't in the source.
#[test]
fn every_occurrence_byte_range_is_inside_its_source_file() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_nested_clone_fixture(&scan_root)?;
    let report = run_and_load_report(tmp.path(), &scan_root)?;
    assert!(
        first_out_of_bounds(&report, &scan_root).is_none(),
        "every occurrence byte range must lie inside its source file: {}",
        first_out_of_bounds(&report, &scan_root).unwrap_or_default(),
    );
    Ok(())
}

// Regression for NAP's "cluster.size / weight inflated by window
// fanout" claim. With three contiguous sibling loops per file, the
// pre-fix sibling pass emitted widths 2 and 3 over the same triple —
// i.e. multiple overlapping members in the same file inside a
// sibling-window cluster. After the fix, the largest cluster
// containing a span covering more than one loop (the sibling-window
// cluster, not the per-loop subtree match) collapses to one
// occurrence per file.
#[test]
fn sibling_window_cluster_has_one_occurrence_per_file() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_nested_clone_fixture(&scan_root)?;
    let report = run_and_load_report(tmp.path(), &scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let sibling_cluster = clusters
        .iter()
        .find(|cluster| max_span_bytes(cluster) >= 100)
        .cloned();
    assert!(
        sibling_cluster.is_some(),
        "no sibling-window cluster found in report: {report:#}"
    );
    let sibling_cluster = sibling_cluster.unwrap_or_default();
    for (path, count) in per_file_counts(&sibling_cluster) {
        assert_eq!(
            count, 1,
            "sibling-window cluster {:?} reports {count} occurrences in {path}; \
             overlapping windows must collapse to one per file",
            sibling_cluster
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("?"),
        );
    }
    Ok(())
}

/// Returns the largest `end_byte - start_byte` across a cluster's
/// occurrences. Used to identify sibling-window clusters — per-loop
/// subtree matches in the fixture span <100 bytes each, sibling
/// windows over 2–3 contiguous loops span ≥100.
fn max_span_bytes(cluster: &serde_json::Value) -> u64 {
    let Some(occurrences) = cluster.get("occurrences").and_then(serde_json::Value::as_array) else {
        return 0;
    };
    let mut best: u64 = 0;
    for occurrence in occurrences {
        let start = occurrence
            .get("start_byte")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let end = occurrence
            .get("end_byte")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let span = end.saturating_sub(start);
        if span > best {
            best = span;
        }
    }
    best
}
