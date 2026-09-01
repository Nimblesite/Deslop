//! [Deslop#151] When the LSP IPC socket is missing, the error
//! surfaced to the MCP client must include the absolute path the
//! backend tried to connect to.
//!
//! Without this, a user whose MCP was launched with `--root .` against
//! the wrong cwd sees `MCP error -32004: LSP is not running` and has
//! no way to tell that the MCP and LSP are pointed at different roots.
//! The enriched message names the directory so the mismatch is
//! immediately visible.

#![cfg(unix)]

use anyhow::{ensure, Result};
use serde_json::json;
use tempfile::TempDir;

use crate::common;
use common::{error_and_message, expected_socket_fragment, initialized_mcp};

#[test]
fn issue_151_top_offenders_error_names_socket_path_when_lsp_absent() -> Result<()> {
    let workspace = TempDir::new()?;
    // Intentionally do NOT spawn an LSP. The socket file is absent.
    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "duplicates", "arguments": { "offset": 0, "limit": 5, "detail": "summary" } }),
    )?;
    let (_error, message) = error_and_message(&response)?;

    let socket_fragment = expected_socket_fragment(workspace.path())?;
    ensure!(
        message.contains(&socket_fragment),
        "error must name the exact socket path so users hit by --root mismatch can diagnose ([Deslop#151]): {message}"
    );
    ensure!(
        message.contains("--root"),
        "error must mention --root so the next debugging step is obvious: {message}"
    );
    Ok(())
}
