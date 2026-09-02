//! Shared report-wire assertions for the `deslop-lsp` integration
//! binaries. The LSP publishes one slim report shape, so the shell
//! contract, occurrence-path extraction and signal access belong in one
//! place rather than being restated per binary.

#![allow(dead_code)]

use std::{
    collections::BTreeSet,
    fs,
    io::BufReader,
    path::Path,
    process::{ChildStdin, ChildStdout},
    time::Duration,
};

use anyhow::{anyhow, Result};
use serde_json::Value;

use super::{at, fixture, path as json_path, wait_for_report_matching};

/// How long a published-report wait may run before the harness calls it
/// a hang. Generous by design: it bounds a deadlock, it does not assert
/// anything about how fast analysis is.
pub const REPORT_TIMEOUT: Duration = Duration::from_secs(30);

/// Waits for the next published report matching `predicate`.
pub fn wait_for_report(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    predicate: impl Fn(&Value) -> bool,
) -> Result<Value> {
    wait_for_report_matching(stdin, stdout, REPORT_TIMEOUT, predicate)
}

/// Creates a temp workspace whose scan root sits beneath a
/// `node_modules` ancestor — the built-in-exclusion scoping case.
///
/// Canonicalises the temp dir first so the watcher root and the paths
/// `notify` reports share one namespace: macOS aliases `/var` to
/// `/private/var`, and a default tempdir would make the test exercise
/// path canonicalisation instead of the exclusion rule.
///
/// Returns the guard — which must outlive the server — and the root.
pub fn dependency_workspace() -> Result<(tempfile::TempDir, std::path::PathBuf)> {
    let canonical_temp = fs::canonicalize(std::env::temp_dir())?;
    let workspace = tempfile::tempdir_in(canonical_temp)?;
    let root = workspace.path().join("node_modules/workspace");
    Ok((workspace, root))
}

/// Copies a named subset of a fixture's files into `destination`,
/// creating it if needed.
pub fn copy_fixture_files(name: &str, files: &[&str], destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    let source = fixture(name);
    for file in files {
        let _bytes = fs::copy(source.join(file), destination.join(file))?;
    }
    Ok(())
}

/// Asserts the `initialize` response identifies the real server and
/// advertises capabilities.
pub fn assert_initialize_contract(frame: &Value) {
    assert_eq!(
        json_path(frame, &["result", "serverInfo", "name"]),
        "deslop-lsp"
    );
    assert!(json_path(frame, &["result", "serverInfo", "version"]).is_string());
    assert!(json_path(frame, &["result", "capabilities"]).is_object());
    assert!(frame.get("error").is_none(), "initialize failed: {frame}");
}

/// Asserts the full published-report shell for a report that must carry
/// clusters: file count, analysis settings, non-zero metrics, and the
/// slim wire shape the editor consumes.
pub fn assert_report_shell(report: &Value, expected_files: u64) {
    assert_eq!(at(report, "files_analysed"), expected_files, "{report:#}");
    assert_eq!(at(report, "min_nodes"), 30, "{report:#}");
    assert!(at(report, "tool_version")
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(at(report, "clusters")
        .as_array()
        .is_some_and(|clusters| !clusters.is_empty()));
    assert!(
        json_path(report, &["metrics", "analysed_loc"])
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert!(
        json_path(report, &["metrics", "duplicated_loc"])
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert!(
        json_path(report, &["metrics", "duplication_percent"])
            .as_f64()
            .unwrap_or_default()
            > 0.0
    );
    assert_eq!(
        at(report, "schema_doc"),
        "",
        "LSP report must use the slim wire shape"
    );
    // [MCP-TOOLS] normative cutover: `action_hints` retired from the wire
    // with the old report surface. A retired field leaking back into the
    // slim shape would resurrect the fat payload — pin its absence.
    assert!(
        report.get("action_hints").is_none(),
        "retired action_hints must not leak into the slim wire report: {report}"
    );
    assert!(at(report, "boilerplate_hints").is_array());
}

/// Asserts the report shell for a report that legitimately carries no
/// clusters — the same wire contract minus the population assertions.
pub fn assert_report_shell_without_clusters(report: &Value, expected_files: u64) {
    assert_eq!(at(report, "files_analysed"), expected_files, "{report:#}");
    assert_eq!(at(report, "min_nodes"), 30, "{report:#}");
    assert!(at(report, "tool_version").is_string());
    assert!(at(report, "metrics").is_object());
    assert_eq!(at(report, "schema_doc"), "");
    assert!(at(report, "clusters").is_array());
}

/// The report's `clusters` array, or an error naming the report.
pub fn report_clusters(report: &Value) -> Result<&Vec<Value>> {
    at(report, "clusters")
        .as_array()
        .ok_or_else(|| anyhow!("report carries no clusters array: {report}"))
}

/// Every occurrence path in the report, slash-normalised so the
/// assertions read the same on Windows.
pub fn occurrence_paths(report: &Value) -> Result<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    for cluster in report_clusters(report)? {
        for occurrence in at(cluster, "occurrences").as_array().unwrap_or(&Vec::new()) {
            if let Some(path) = at(occurrence, "path").as_str() {
                let _inserted = paths.insert(path.replace('\\', "/"));
            }
        }
    }
    Ok(paths)
}

/// One cluster's occurrence paths, slash-normalised.
pub fn cluster_paths(cluster: &Value) -> BTreeSet<String> {
    at(cluster, "occurrences")
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|occurrence| at(occurrence, "path").as_str())
        .map(|path| path.replace('\\', "/"))
        .collect()
}

/// Asserts no occurrence in the report is marked hidden.
pub fn assert_all_occurrences_visible(report: &Value) -> Result<()> {
    for cluster in report_clusters(report)? {
        for occurrence in at(cluster, "occurrences").as_array().unwrap_or(&Vec::new()) {
            assert_eq!(
                at(occurrence, "hidden"),
                false,
                "unexpected hidden occurrence: {occurrence}"
            );
        }
    }
    Ok(())
}

/// True when any path ends with `suffix`.
pub fn has_suffix(paths: &BTreeSet<String>, suffix: &str) -> bool {
    paths.iter().any(|path| path.ends_with(suffix))
}

/// True when any path contains `fragment`.
pub fn has_fragment(paths: &BTreeSet<String>, fragment: &str) -> bool {
    paths.iter().any(|path| path.contains(fragment))
}

/// One named signal off a cluster, `NaN` when absent so a missing signal
/// fails every band assertion rather than defaulting to zero.
pub fn signal(cluster: &Value, name: &str) -> f64 {
    at(cluster, "signals")[name].as_f64().unwrap_or(f64::NAN)
}
