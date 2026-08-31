//! Regression coverage for issue #141: `deslop-mcp` must bind to an
//! absolute, canonicalised workspace root. Returning duplicate
//! clusters from a wrong root (e.g. `~/.cargo/git/checkouts/...`) and
//! hanging on `session-config` are the two failure shapes the issue
//! tracks; the fixes are: canonicalise `--root` at start-up and
//! refuse to scan vendored Cargo cache trees.
//!
//! Under [MCP-IPC-CLIENT] every read tool call delegates to the LSP
//! over a unix socket, so each test spawns a companion `deslop-lsp`
//! against the same workspace before driving the MCP child.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::common;

fn initialized_mcp_at(
    root: &Path,
    root_argument: &str,
) -> Result<(common::McpHandle, common::ChildKillOnDrop)> {
    let lsp = common::spawn_lsp_and_wait_for_socket(root)?;
    let mut mcp = common::McpHandle::spawn_with_root_argument(root, root_argument)?;
    common::initialize_mcp(&mut mcp)?;
    Ok((mcp, lsp))
}

fn seed_one_csharp_file(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join("OnlyFile.cs"),
        "namespace OnlyNs { public class OnlyClass { public int OnlyMethod() { return 1; } } }\n",
    )?;
    Ok(())
}

// Issue #141: starting `deslop-mcp --root .` while CWD points at the
// intended workspace must bind the session to the absolute,
// canonicalised path of that directory — never the literal "." or a
// path elsewhere. Without this the renderer reports occurrences from
// a different filesystem root than the client thinks it is asking
// about.
#[test]
fn issue_141_relative_root_resolves_to_canonical_workspace() -> Result<()> {
    let workspace = TempDir::new()?;
    seed_one_csharp_file(workspace.path())?;
    let canonical_workspace = fs::canonicalize(workspace.path())?;

    let (mut mcp, _lsp) = initialized_mcp_at(&canonical_workspace, ".")?;
    let snapshot = common::call_tool(&mut mcp, "session", &json!({}))?;
    let reported_root = snapshot
        .get("root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("session did not return a string root: {snapshot}"))?;
    assert!(
        reported_root.is_absolute(),
        "session root must be absolute, got {reported_root:?}",
    );
    let canonical_reported = fs::canonicalize(&reported_root)?;
    assert_eq!(
        canonical_reported, canonical_workspace,
        "session root must canonicalise to the launch CWD",
    );
    Ok(())
}

// Issue #141 / #142: even when MCP is correctly rooted, vendored
// Cargo cache trees nested under the scan root (e.g. a developer who
// runs `cargo` inside the workspace) must not enter discovery. The
// built-in exclude set covers `.cargo/`; this test asserts the live
// MCP backend honours it.
#[test]
fn issue_141_cargo_cache_under_root_is_not_analysed() -> Result<()> {
    let workspace = TempDir::new()?;
    let canonical_workspace = fs::canonicalize(workspace.path())?;
    seed_one_csharp_file(&canonical_workspace)?;
    let cargo_checkouts = canonical_workspace
        .join(".cargo")
        .join("git")
        .join("checkouts")
        .join("ruff-deadbeef")
        .join("c6516e9");
    fs::create_dir_all(&cargo_checkouts)?;
    for index in 0..3 {
        fs::write(
            cargo_checkouts.join(format!("Generated{index}.cs")),
            "namespace G { public class W { public int R() { return 1; } } }\n",
        )?;
    }

    let workspace_str = canonical_workspace
        .to_str()
        .ok_or_else(|| anyhow!("workspace path is not UTF-8"))?;
    let (mut mcp, _lsp) = initialized_mcp_at(&canonical_workspace, workspace_str)?;
    let page = common::call_tool(
        &mut mcp,
        "duplicates",
        &json!({ "offset": 0, "limit": 50, "detail": "full" }),
    )?;
    let files_analysed = page
        .get("files_analysed")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("duplicates missing files_analysed: {page}"))?;
    assert_eq!(
        files_analysed, 1,
        "the three .cargo cache files must not enter discovery: {page}"
    );
    let clusters = page
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("duplicates missing clusters: {page}"))?;
    for cluster in clusters {
        let occurrences = cluster
            .get("occurrences")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("full duplicates cluster missing occurrences: {cluster}"))?;
        for occurrence in occurrences {
            let path = occurrence
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("occurrence missing path: {occurrence}"))?;
            assert!(
                !path.contains(".cargo/"),
                "cluster occurrence leaked cargo cache: {path}",
            );
        }
    }
    Ok(())
}
