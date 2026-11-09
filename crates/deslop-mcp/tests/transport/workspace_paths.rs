//! [LIVE-IPC-SOCKET] [MCP-IPC-DISCOVERY] Workspace depth must not prevent IPC.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use anyhow::{ensure, Context, Result};
use assert_cmd::cargo::cargo_bin;
use deslop_core::{report::ReportCluster, Report};
use serde_json::{json, Value};

use crate::common::{
    array_field, call_tool, copied_fixture, initialized_mcp, lsp_child::spawn_lsp_and_initialize,
    u64_field, ChildKillOnDrop,
};

const LONG_COMPONENT: &str = "workspace";
const COMPONENT_REPETITIONS: usize = 16;
const MACOS_SOCKET_CAPACITY: usize = 104;
const CACHE: &str = ".deslop/cache";
const SOCKET: &str = ".deslop/cache/deslop.sock";
const BASELINE: &str = ".deslop/baseline";
const BASELINE_JSON: &str = ".deslop/baseline.json";
const DUPLICATES: &str = "duplicates";
const DETAIL: &str = "full";
const PAGE_LIMIT: usize = 10;
const OCCURRENCE_BUDGET: usize = 100;
const CLUSTERS: &str = "clusters";
const TOTAL_CLUSTERS: &str = "total_clusters";
const PERCENT: &str = "/metrics/duplication_percent";
const EXPECTED_CLUSTERS: usize = 2;
const EXPECTED_COPIES: usize = 2;
const EXPECTED_FILES: [&str; EXPECTED_CLUSTERS * EXPECTED_COPIES] =
    ["Alpha.cs", "Beta.cs", "Delta.cs", "Gamma.cs"];
const PORT_RECORD: &str = ".deslop/cache/deslop.port";
const CORRUPT_RECORD: &str = "invalid discovery record";
const DISCOVERY_ERROR: &str = "ipc discovery record corrupt";
const ERROR_MESSAGE: &str = "/error/message";
const UNICODE_COMPONENT: &str = "資料";
const UNICODE_REPETITIONS: usize = 24;
const RECONNECT_ATTEMPTS: usize = 3;
const SINGLE_CLUSTER: usize = 1;

/// [MCP-IPC-DISCOVERY] A broken endpoint names the workspace and underlying cause.
#[test]
fn connection_error_names_workspace_and_cause() -> Result<()> {
    let workspace = copied_fixture()?;
    fs::create_dir_all(workspace.path().join(CACHE))?;
    fs::write(workspace.path().join(PORT_RECORD), CORRUPT_RECORD)?;
    let mut mcp = initialized_mcp(workspace.path())?;
    let response = crate::common::request_duplicates_summary(&mut mcp, u64::try_from(PAGE_LIMIT)?)?;
    let message = response
        .pointer(ERROR_MESSAGE)
        .and_then(Value::as_str)
        .context("broken endpoint must return an actionable error")?;
    ensure!(message.contains(DISCOVERY_ERROR), "{message}");
    ensure!(
        message.contains(workspace.path().to_str().context("workspace UTF-8")?),
        "{message}"
    );
    Ok(())
}

/// [LIVE-IPC-SOCKET] [MCP-IPC-DISCOVERY] The default transport serves long roots.
#[test]
fn mcp_connects_for_long_workspace_roots() -> Result<()> {
    for component in [
        LONG_COMPONENT.repeat(COMPONENT_REPETITIONS),
        UNICODE_COMPONENT.repeat(UNICODE_REPETITIONS),
        format!(
            "{} with spaces",
            LONG_COMPONENT.repeat(COMPONENT_REPETITIONS)
        ),
    ] {
        assert_workspace_connections(&component)?;
    }
    Ok(())
}

/// Every path spelling serves identical reports across independent MCP clients.
fn assert_workspace_connections(component: &str) -> Result<()> {
    let fixture = copied_fixture()?;
    let workspace = fixture.path().join(component);
    move_fixture(fixture.path(), &workspace)?;
    ensure!(workspace.as_os_str().len() > MACOS_SOCKET_CAPACITY);
    ensure!(workspace.join(SOCKET).as_os_str().len() > MACOS_SOCKET_CAPACITY);
    let expected = cli_report(&workspace)?;
    assert_fixture(&expected);
    fs::remove_dir_all(workspace.join(CACHE))?;
    let _lsp = ChildKillOnDrop(spawn_lsp_and_initialize(&workspace)?);
    for _attempt in 0..RECONNECT_ATTEMPTS {
        assert_client_queries(&workspace, &expected)?;
    }
    Ok(())
}

/// Complete and paged replies agree with the CLI after each reconnection.
fn assert_client_queries(workspace: &Path, expected: &Report) -> Result<()> {
    let mut mcp = initialized_mcp(workspace)?;
    let arguments = json!({"detail": DETAIL, "limit": PAGE_LIMIT,
        "max_occurrences": OCCURRENCE_BUDGET});
    let page = call_tool(&mut mcp, DUPLICATES, &arguments)?;
    assert_report_parity(&page, expected)?;
    assert_tcp_discovery(workspace);
    for (offset, cluster) in expected.clusters.iter().enumerate() {
        let arguments = json!({"detail": DETAIL, "limit": SINGLE_CLUSTER,
            "offset": offset, "max_occurrences": OCCURRENCE_BUDGET});
        assert_single_cluster(&call_tool(&mut mcp, DUPLICATES, &arguments)?, cluster)?;
    }
    assert_report_parity(&call_tool(&mut mcp, DUPLICATES, &arguments)?, expected)
}

/// Successful long-root connections publish only the TCP discovery artefact.
fn assert_tcp_discovery(workspace: &Path) {
    assert!(
        workspace.join(PORT_RECORD).is_file(),
        "long roots publish TCP discovery"
    );
    assert!(
        !workspace.join(SOCKET).exists(),
        "long roots never leave a socket behind"
    );
}

/// Pagination preserves complete occurrences and the count before slicing.
fn assert_single_cluster(page: &Value, cluster: &ReportCluster) -> Result<()> {
    assert_eq!(
        serde_json::to_value(page_clusters(page)?)?,
        json!([cluster])
    );
    assert_eq!(
        u64_field(page, TOTAL_CLUSTERS)?,
        u64::try_from(EXPECTED_CLUSTERS)?
    );
    Ok(())
}

/// Relocates the existing fixture files without creating another fixture copy.
fn move_fixture(parent: &Path, workspace: &Path) -> Result<()> {
    fs::create_dir(workspace)?;
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::rename(entry.path(), workspace.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Renders the CLI's canonical JSON report for this exact workspace.
fn cli_report(workspace: &Path) -> Result<Report> {
    let output = Command::new(cargo_bin("deslop"))
        .arg(workspace)
        .arg("--output")
        .arg(workspace.join(BASELINE))
        .args(["--notext", "--nohtml", "--no-fail-over"])
        .output()?;
    ensure!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&fs::read(workspace.join(BASELINE_JSON))?).context("parse CLI baseline")
}

/// Pins the fixture so parity cannot pass if both surfaces return no clusters.
fn assert_fixture(report: &Report) {
    assert_eq!(report.clusters.len(), EXPECTED_CLUSTERS);
    for cluster in &report.clusters {
        assert_eq!(cluster.occurrence_count, EXPECTED_COPIES);
        assert_eq!(cluster.occurrences.len(), EXPECTED_COPIES);
    }
    let paths: BTreeSet<_> = report
        .clusters
        .iter()
        .flat_map(|cluster| cluster.occurrences.iter().map(|item| item.path.clone()))
        .collect();
    assert_eq!(paths, EXPECTED_FILES.map(std::path::PathBuf::from).into());
}

/// Decodes full cluster rows through the same model as the CLI baseline.
fn page_clusters(page: &Value) -> Result<Vec<ReportCluster>> {
    array_field(page, CLUSTERS)?
        .iter()
        .map(|cluster| serde_json::from_value(cluster.clone()))
        .collect::<std::result::Result<_, _>>()
        .context("decode full MCP clusters")
}

/// Compare ranked clusters, identities, buckets, occurrences and the core metric.
fn assert_report_parity(page: &Value, report: &Report) -> Result<()> {
    assert_eq!(
        u64_field(page, TOTAL_CLUSTERS)?,
        u64::try_from(EXPECTED_CLUSTERS)?
    );
    assert_eq!(
        serde_json::to_value(page_clusters(page)?)?,
        serde_json::to_value(&report.clusters)?
    );
    assert_eq!(
        page.pointer(PERCENT)
            .context("MCP duplication percentage")?,
        &json!(report.metrics.duplication_percent)
    );
    Ok(())
}
