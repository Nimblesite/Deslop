//! [MCP-IPC-TRANSPORT] An LSP of this build that answers with a JSON-RPC
//! `error` envelope is reported as a transport failure carrying the
//! engine's own message: never a result page, and never the other-build
//! refusal ([MCP-IPC-WIRE-MISMATCH]).

#![cfg(unix)]

use std::sync::Arc;

use anyhow::{ensure, Result};
use common::{
    initialized_mcp, refused_call_message,
    stub_lsp::{bind_stub_lsp, Reply, Stamp},
};
use serde_json::json;
use tempfile::TempDir;

use crate::common;

/// A server-defined JSON-RPC error code: the engine failed, the wire did not.
const ENGINE_ERROR_CODE: i64 = -32_000;
const ENGINE_ERROR_MESSAGE: &str = "analysis session unavailable";
const TRANSPORT_FAILURE: &str = "ipc transport failure: ipc rpc error:";
/// The remedy only the other-build refusal names.
const OTHER_BUILD_REMEDY: &str = "reinstall the Deslop VSIX";
const PAGE_LIMIT: u64 = 5;

#[test]
fn an_engine_error_envelope_is_reported_as_a_transport_failure() -> Result<()> {
    let workspace = TempDir::new()?;
    bind_stub_lsp(
        workspace.path(),
        Stamp::ThisBuild,
        Arc::new(|_method| Reply::Error {
            code: ENGINE_ERROR_CODE,
            message: ENGINE_ERROR_MESSAGE,
        }),
    )?;
    let mut mcp = initialized_mcp(workspace.path())?;
    for (tool, arguments) in [
        ("duplicates", json!({ "limit": PAGE_LIMIT })),
        ("cluster-by-id", json!({ "id": "0000000000000000" })),
    ] {
        let message = refused_call_message(&mut mcp, tool, &arguments)?;
        ensure!(
            message.contains(TRANSPORT_FAILURE),
            "{tool}: names the transport failure: {message}"
        );
        ensure!(
            message.contains(ENGINE_ERROR_MESSAGE),
            "{tool}: carries the engine's own message: {message}"
        );
        ensure!(
            !message.contains(OTHER_BUILD_REMEDY),
            "{tool}: a same-build failure is not the other-build refusal: {message}"
        );
    }
    Ok(())
}
