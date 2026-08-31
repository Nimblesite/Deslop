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

use std::{collections::BTreeMap, fs, path::Path, path::PathBuf};

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

/// Writes a Python fixture under the same path family as the reported
/// NAP runaway (`alembic/versions/003_cascade_delete_config.py`). The
/// two files contain equivalent migration-shaped code, which exercises
/// the sibling-window path without depending on a private checkout.
fn write_phantom_occurrence_fixture(dir: &Path) -> Result<()> {
    let alembic_dir = dir.join("alembic").join("versions");
    let tests_dir = dir.join("tests");
    fs::create_dir_all(&alembic_dir)?;
    fs::create_dir_all(&tests_dir)?;
    fs::write(
        alembic_dir.join("003_cascade_delete_config.py"),
        phantom_occurrence_body("upgrade", "rules", "configs"),
    )?;
    fs::write(
        tests_dir.join("test_sandbox_coverage.py"),
        phantom_occurrence_body("exercise", "jobs", "agents"),
    )?;
    Ok(())
}

fn phantom_occurrence_body(function: &str, child: &str, parent: &str) -> String {
    format!(
        "\"\"\"Synthetic cascade-delete migration fixture.\"\"\"\n\
         from alembic import op\n\
         import sqlalchemy as sa\n\
         \n\
         revision = \"003\"\n\
         down_revision = \"002\"\n\
         branch_labels = None\n\
         depends_on = None\n\
         \n\
         \n\
         def {function}():\n\
             config_id = sa.Column(\"config_id\", sa.Integer(), nullable=False)\n\
             op.add_column(\"{child}\", config_id)\n\
             op.create_index(\"ix_{child}_config_id\", \"{child}\", [\"config_id\"])\n\
             op.create_foreign_key(\n\
                 \"fk_{child}_config_id\",\n\
                 \"{child}\",\n\
                 \"{parent}\",\n\
                 [\"config_id\"],\n\
                 [\"id\"],\n\
                 ondelete=\"CASCADE\",\n\
             )\n\
             op.execute(\"UPDATE {child} SET config_id = 1 WHERE config_id IS NULL\")\n\
             op.alter_column(\"{child}\", \"config_id\", nullable=False)\n\
             op.drop_constraint(\"old_{child}_config_id_fkey\", \"{child}\", type_=\"foreignkey\")\n\
             op.create_foreign_key(\n\
                 \"fk_{child}_config_id_strict\",\n\
                 \"{child}\",\n\
                 \"{parent}\",\n\
                 [\"config_id\"],\n\
                 [\"id\"],\n\
                 ondelete=\"CASCADE\",\n\
             )\n\
             op.drop_index(\"ix_{child}_legacy_config\", table_name=\"{child}\")\n"
    )
}

/// Runs the CLI against `scan_root`, writing reports under
/// `<tmp>/report.*`, and returns the parsed JSON report.
fn run_and_load_report(tmp: &Path, scan_root: &Path) -> Result<serde_json::Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(scan_root)
        .args(["--min-nodes", "4", "--output"])
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

/// Builds a temp scan root, writes a fixture into it via `write_fixture`,
/// runs the CLI, and returns the owning `TempDir` (kept alive for the
/// caller), the `scan_root` path, and the parsed JSON report. The
/// `TempDir` must be held by the caller so the fixture survives until the
/// assertions finish.
fn prepared_report(
    write_fixture: fn(&Path) -> Result<()>,
) -> Result<(tempfile::TempDir, PathBuf, serde_json::Value)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_fixture(&scan_root)?;
    let report = run_and_load_report(tmp.path(), &scan_root)?;
    Ok((tmp, scan_root, report))
}

/// Returns `value`'s `key` array, or an empty slice when the field is
/// absent or not an array.
fn array_field<'a>(value: &'a serde_json::Value, key: &str) -> &'a [serde_json::Value] {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Returns the report's `clusters` array as an owned `Vec`, or an empty
/// `Vec` when the field is absent or not an array.
fn clusters_array(report: &serde_json::Value) -> Vec<serde_json::Value> {
    array_field(report, "clusters").to_vec()
}

/// Returns a cluster's short id, or `"?"` when the field is absent, so
/// every failure message names the cluster the same way.
fn cluster_id(cluster: &serde_json::Value) -> &str {
    cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?")
}

/// Returns a cluster's `occurrences` array, or an empty slice when the
/// field is absent or not an array.
fn occurrences_of(cluster: &serde_json::Value) -> &[serde_json::Value] {
    array_field(cluster, "occurrences")
}

/// Reads an unsigned integer field, defaulting to `0` when it is absent
/// or not a number.
fn u64_field(value: &serde_json::Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

/// Applies `probe` to every cluster in `report` and returns the first
/// failure description it produces. `None` means every cluster is clean.
fn first_cluster_finding(
    report: &serde_json::Value,
    probe: impl Fn(&serde_json::Value) -> Option<String>,
) -> Option<String> {
    array_field(report, "clusters").iter().find_map(probe)
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
    first_cluster_finding(report, first_overlap_in_cluster)
}

/// Inner loop of [`first_overlap`]: reports the first overlapping pair
/// inside a single cluster, annotated with the cluster id for the
/// failure message.
fn first_overlap_in_cluster(cluster: &serde_json::Value) -> Option<String> {
    let occurrences = occurrences_of(cluster);
    let id = cluster_id(cluster);
    occurrences.iter().enumerate().find_map(|(index, left)| {
        occurrences
            .iter()
            .skip(index.saturating_add(1))
            .find_map(|right| overlap_pair(id, left, right))
    })
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
    first_cluster_finding(report, |cluster| {
        let id = cluster_id(cluster);
        occurrences_of(cluster)
            .iter()
            .find_map(|occurrence| out_of_bounds_line(id, occurrence, scan_root))
    })
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
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for path in occurrences_of(cluster)
        .iter()
        .filter_map(|occurrence| occurrence.get("path").and_then(serde_json::Value::as_str))
    {
        let entry = counts.entry(path.to_owned()).or_insert(0_usize);
        *entry = entry.saturating_add(1);
    }
    counts.into_iter().collect()
}

fn first_cluster_count_mismatch(report: &serde_json::Value) -> Option<String> {
    first_cluster_finding(report, |cluster| {
        let count = u64_field(cluster, "occurrence_count");
        let occurrences_total = u64_field(cluster, "occurrences_total");
        let visible = occurrences_of(cluster)
            .iter()
            .filter(|occurrence| {
                !occurrence
                    .get("hidden")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            })
            .count();
        let occurrences_len = u64::try_from(visible).ok()?;
        let full_len = u64::try_from(occurrences_of(cluster).len()).ok()?;
        let truncated = cluster
            .get("occurrences_truncated")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let id = cluster_id(cluster);
        (count != occurrences_len || occurrences_total != full_len || truncated).then(|| {
            format!(
                "cluster {id}: occurrence_count={count}, occurrences_total={occurrences_total}, visible={occurrences_len}, full={full_len}, truncated={truncated}"
            )
        })
    })
}

fn first_bad_mass(report: &serde_json::Value) -> Option<String> {
    first_cluster_finding(report, |cluster| {
        let id = cluster_id(cluster);
        let nodes = cluster.get("canonical_node_count")?.as_u64()?;
        let count = cluster.get("occurrence_count")?.as_u64()?;
        let mass = cluster.get("mass")?.as_u64()?;
        let expected = nodes.saturating_mul(count.saturating_sub(1));
        (mass != expected).then(|| format!("cluster {id} mass={mass} expected={expected}"))
    })
}

fn duplication_percent(report: &serde_json::Value) -> f64 {
    report
        .get("metrics")
        .and_then(|metrics| metrics.get("duplication_percent"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(f64::NAN)
}

fn max_occurrences_for_path(report: &serde_json::Value, needle: &str) -> usize {
    array_field(report, "clusters")
        .iter()
        .map(|cluster| occurrences_for_path(cluster, needle))
        .max()
        .unwrap_or_default()
}

fn occurrences_for_path(cluster: &serde_json::Value, needle: &str) -> usize {
    occurrences_of(cluster)
        .iter()
        .filter(|occurrence| {
            occurrence
                .get("path")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| path == needle)
        })
        .count()
}

fn line_count(path: &Path) -> Result<usize> {
    Ok(fs::read_to_string(path)?.lines().count())
}

// Regression for NAP's "overlapping byte ranges inside the same file"
// claim. With the fix in place, no cluster may contain two
// occurrences that point at the same file and overlap.
#[test]
fn clusters_never_contain_overlapping_occurrences_in_same_file() -> Result<()> {
    let (_tmp, _scan_root, report) = prepared_report(write_nested_clone_fixture)?;
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
    let (_tmp, _scan_root, report) = prepared_report(write_nested_clone_fixture)?;
    let clusters = clusters_array(&report);
    assert!(
        !clusters.is_empty(),
        "fixture must produce at least one clone cluster: {report:#}"
    );
    for cluster in &clusters {
        // The `size` field was removed with the bucket surface; the
        // mass-only wire carries `occurrence_count` (visible membership)
        // beside `occurrences_total`. The invariant is the same: every
        // reported occurrence must be counted, and nothing may hide
        // behind the cluster id ([PIPELINE-CLUSTER-CLOSURE]). Visible
        // membership excludes report-hidden occurrences
        // ([RANK-MASS-SUM]).
        let count = u64_field(cluster, "occurrence_count");
        let visible_len = u64::try_from(
            occurrences_of(cluster)
                .iter()
                .filter(|occurrence| {
                    !occurrence
                        .get("hidden")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                })
                .count(),
        )?;
        let occurrences_total = u64_field(cluster, "occurrences_total");
        let truncated = cluster
            .get("occurrences_truncated")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        assert_eq!(
            count, visible_len,
            "cluster.occurrence_count must equal the visible occurrences after overlap \
         dedup: {cluster:#}"
        );
        assert_eq!(
            occurrences_total,
            u64::try_from(occurrences_of(cluster).len())?,
            "occurrences_total must equal the full reported membership (hidden included): {cluster:#}"
        );
        assert!(
            !truncated,
            "no occurrence may be truncated from the report: {cluster:#}"
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
    let (_tmp, scan_root, report) = prepared_report(write_nested_clone_fixture)?;
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
    let (_tmp, _scan_root, report) = prepared_report(write_nested_clone_fixture)?;
    let clusters = clusters_array(&report);
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
            count,
            1,
            "sibling-window cluster {:?} reports {count} occurrences in {path}; \
             overlapping windows must collapse to one per file",
            cluster_id(&sibling_cluster),
        );
    }
    Ok(())
}

// Tracker issue #7 rolls the lower-level phantom-occurrence symptoms
// into one public contract: report occurrences must be physically
// grounded in source files, not inflated by sibling-window fanout, and
// public report metrics/scores must stay sane.
#[test]
fn phantom_occurrence_fixture_respects_report_invariants() -> Result<()> {
    let (_tmp, scan_root, report) = prepared_report(write_phantom_occurrence_fixture)?;
    let clusters = clusters_array(&report);
    assert!(
        !clusters.is_empty(),
        "phantom-occurrence fixture must produce clone clusters: {report:#}"
    );
    assert!(
        first_out_of_bounds(&report, &scan_root).is_none(),
        "every occurrence must point inside its source file: {}",
        first_out_of_bounds(&report, &scan_root).unwrap_or_default()
    );
    assert!(
        first_overlap(&report).is_none(),
        "cluster occurrences must not overlap inside one file: {}",
        first_overlap(&report).unwrap_or_default()
    );
    assert!(
        first_cluster_count_mismatch(&report).is_none(),
        "cluster counts must be internally consistent: {}",
        first_cluster_count_mismatch(&report).unwrap_or_default()
    );
    assert!(
        first_bad_mass(&report).is_none(),
        "every cluster mass must be canonical_node_count × (occurrence_count − 1): {}",
        first_bad_mass(&report).unwrap_or_default()
    );
    let duplication = duplication_percent(&report);
    assert!(
        (0.0..=100.0).contains(&duplication),
        "duplication_percent must be within [0, 100], got {duplication}: {report:#}"
    );
    let alembic_path = "alembic/versions/003_cascade_delete_config.py";
    let alembic_lines = line_count(&scan_root.join(alembic_path))?;
    let max_alembic_occurrences = max_occurrences_for_path(&report, alembic_path);
    assert!(
        max_alembic_occurrences <= alembic_lines,
        "one cluster reports {max_alembic_occurrences} occurrences in {alembic_path}, \
         but the file has only {alembic_lines} lines"
    );
    Ok(())
}

/// Returns the largest `end_byte - start_byte` across a cluster's
/// occurrences. Used to identify sibling-window clusters — per-loop
/// subtree matches in the fixture span <100 bytes each, sibling
/// windows over 2–3 contiguous loops span ≥100.
fn max_span_bytes(cluster: &serde_json::Value) -> u64 {
    occurrences_of(cluster)
        .iter()
        .map(|occurrence| {
            u64_field(occurrence, "end_byte").saturating_sub(u64_field(occurrence, "start_byte"))
        })
        .max()
        .unwrap_or_default()
}
