//! [MCP-START-RECOVERY] An agent can recover without guessing command syntax.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::common::{
    call_tool, copied_fixture, initialized_mcp, request_duplicates_summary,
    spawn_lsp_and_wait_for_socket, wait_for_state_then_init_mcp,
};

const PAGE_LIMIT: u64 = 5;
const MISSING_ENGINE: i64 = -32_004;
const REPORT_PATH: &str = ".deslop/deslop-report.json";
const CLI_ARGUMENTS: [&str; 3] = ["--notext", "--nohtml", "--no-color"];
const FOR_AI: &str = "https://deslop.live/docs/for-ai/";
const TAXONOMY: &str = "https://github.com/Nimblesite/Deslop/blob/main/docs/specs/taxonomy.md";

/// Recovery names the snapshot and a command adjacent to this bundled MCP.
fn assert_recovery(reply: &Value, root: &Path) -> Result<()> {
    assert_eq!(reply.pointer("/error/code"), Some(&json!(MISSING_ENGINE)));
    let data = reply
        .pointer("/error/data")
        .context("structured recovery data")?;
    assert_reports(data, root);
    assert_cli_arguments(data, root);
    assert_cli_command(data)?;
    assert_eq!(
        data.pointer("/cli_fallback/cache_enabled_by_default"),
        Some(&json!(true))
    );
    assert_eq!(data.get("guide"), Some(&json!(FOR_AI)));
    Ok(())
}

/// Snapshot lookup order and exact paths are part of the recovery contract.
fn assert_reports(data: &Value, root: &Path) {
    let report = root
        .join(REPORT_PATH)
        .components()
        .collect::<std::path::PathBuf>();
    assert_eq!(data.pointer("/report_fallback/path"), Some(&json!(report)));
    assert_eq!(
        data.get("fallback_order"),
        Some(&json!([
            "report_fallback",
            "cache_fallback",
            "cli_fallback"
        ]))
    );
}

/// Arguments are separate strings, ready to execute without a guessed subcommand.
fn assert_cli_arguments(data: &Value, root: &Path) {
    let arguments: Vec<_> = std::iter::once(root.to_string_lossy().into_owned())
        .chain(CLI_ARGUMENTS.into_iter().map(str::to_owned))
        .collect();
    assert_eq!(
        data.pointer("/cli_fallback/arguments"),
        Some(&json!(arguments))
    );
}

/// The command is the adjacent bundled executable rather than an unrelated PATH binary.
fn assert_cli_command(data: &Value) -> Result<()> {
    let command = data
        .pointer("/cli_fallback/command")
        .and_then(Value::as_str)
        .context("CLI executable")?;
    assert!(Path::new(command).is_absolute());
    assert_eq!(
        Path::new(command)
            .file_stem()
            .and_then(|name| name.to_str()),
        Some("deslop")
    );
    Ok(())
}

#[test]
fn unavailable_engine_provides_json_first_and_cached_cli_recovery() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    let mut mcp = initialized_mcp(workspace.path())?;
    let reply = request_duplicates_summary(&mut mcp, PAGE_LIMIT)?;
    assert_recovery(&reply, &workspace.path().canonicalize()?)
}

/// [REPORT-CONTEXT-SCOPE] A remote client can open the schema's authoritative links.
#[test]
fn live_schema_documentation_has_resolvable_reference_links() -> Result<()> {
    let workspace = copied_fixture()?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    let reply = call_tool(&mut mcp, "schema-doc", &json!({}))?;
    let schema = reply
        .get("schema_doc")
        .and_then(Value::as_str)
        .context("schema markdown")?;
    assert!(
        schema.contains(TAXONOMY),
        "schema references must open outside the repository: {schema}"
    );
    Ok(())
}
