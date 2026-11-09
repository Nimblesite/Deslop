//! JSON-RPC protocol-level tests: the initialize handshake, capability
//! shape, lifecycle (stdin EOF), and malformed / unknown frame handling
//! ([MCP-CAPABILITIES], [MCP-WIRE-FRAMING]).

use std::time::Duration;

use anyhow::{anyhow, Result};
use common::{
    mcp_calls::{assert_error_code, finish_after_tools_list},
    mcp_child::{init_session, spawn_and_init, McpChild},
    mcp_wire::{ERROR_FIELD, ID_POINTER, MCP_PROGRAM_NAME},
    rpc::MCP_PROTOCOL_VERSION,
    value_get,
};
use serde_json::json;

use crate::common;

/// `jsonrpc` version every frame must carry.
const JSONRPC_VERSION: &str = "2.0";
/// JSON pointer: the advertised server name.
const SERVER_INFO_NAME_POINTER: &str = "/result/serverInfo/name";
/// How long the MCP may take to exit after stdin closes.
const SHUTDOWN_TIMEOUT_SECS: u64 = 5;
/// JSON-RPC parse-error code.
const PARSE_ERROR_CODE: i64 = -32_700;
/// JSON-RPC invalid-request code.
const INVALID_REQUEST_CODE: i64 = -32_600;
/// JSON-RPC method-not-found code.
const METHOD_NOT_FOUND_CODE: i64 = -32_601;

#[test]
fn initialize_returns_server_info_and_capabilities() -> Result<()> {
    let mut child = McpChild::spawn(common::fixture_root(), &[])?;
    let response = init_session(&mut child)?;
    assert_eq!(value_get(&response, "/jsonrpc")?, json!(JSONRPC_VERSION));
    assert_eq!(
        value_get(&response, "/result/protocolVersion")?,
        json!(MCP_PROTOCOL_VERSION)
    );
    assert_eq!(
        value_get(&response, SERVER_INFO_NAME_POINTER)?,
        json!(MCP_PROGRAM_NAME)
    );
    assert_eq!(
        value_get(&response, "/result/serverInfo/version")?,
        json!(common::mcp_calls::expected_version())
    );
    assert!(
        value_get(&response, "/result/capabilities/tools")?.is_object(),
        "tools capability missing: {response}"
    );
    assert!(
        value_get(&response, "/result/capabilities/resources")?.is_object(),
        "resources capability missing: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn exits_within_five_seconds_after_stdio_stdin_closes() -> Result<()> {
    let mut child = McpChild::spawn(common::fixture_root(), &[])?;
    assert!(child.pid() > 0, "mcp pid must be observable");
    assert!(
        child.try_wait()?.is_none(),
        "mcp must stay alive before stdin is closed"
    );
    let response = init_session(&mut child)?;
    assert_eq!(
        value_get(&response, SERVER_INFO_NAME_POINTER)?,
        json!(MCP_PROGRAM_NAME)
    );
    assert!(
        value_get(&response, "/result/capabilities/resources")?.is_object(),
        "resources capability missing: {response}"
    );
    let started = std::time::Instant::now();
    let status = child.close_stdin_and_wait(Duration::from_secs(SHUTDOWN_TIMEOUT_SECS))?;
    assert!(status.success(), "stdin EOF should exit cleanly: {status}");
    assert!(
        started.elapsed() < Duration::from_secs(SHUTDOWN_TIMEOUT_SECS),
        "stdin EOF must stop deslop-mcp within five seconds"
    );
    Ok(())
}

#[test]
fn initialize_capabilities_have_no_null_values() -> Result<()> {
    // Regression: a `prompts: null` / `logging: null` payload was rejected
    // by Claude Desktop's MCP picker with `expected: object, received:
    // null`. Capabilities the server does not implement must be omitted,
    // not nulled.
    let mut child = McpChild::spawn(common::fixture_root(), &[])?;
    let response = init_session(&mut child)?;
    let capabilities = value_get(&response, "/result/capabilities")?;
    let object = capabilities
        .as_object()
        .ok_or_else(|| anyhow!("capabilities not an object: {capabilities}"))?;
    for (key, value) in object {
        assert!(
            !value.is_null(),
            "capability {key:?} is null — must be omitted or set to an object instead"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn notifications_initialized_is_accepted_silently() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.notify("notifications/initialized", &json!({}))?;
    finish_after_tools_list(child)
}

#[test]
fn mcp_sends_empty_line_and_server_keeps_going() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.send_raw_line("")?;
    finish_after_tools_list(child)
}

#[test]
fn unknown_method_returns_method_not_found() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("completely/made-up", &json!({}))?;
    assert_error_code(&response, METHOD_NOT_FOUND_CODE)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn malformed_frame_returns_parse_error() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.send_raw_line("{this is not valid json")?;
    let response = child.read_frame()?;
    assert_error_code(&response, PARSE_ERROR_CODE)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn invalid_jsonrpc_version_returns_invalid_request() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.send_raw_line(r#"{"jsonrpc":"1.5","id":99,"method":"ping"}"#)?;
    let response = child.read_frame()?;
    assert_error_code(&response, INVALID_REQUEST_CODE)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn ping_method_returns_empty_object() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("ping", &json!({}))?;
    assert!(response.get(ERROR_FIELD).is_none());
    let _ = child.finish();
    Ok(())
}

#[test]
fn shutdown_method_returns_null_result() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("shutdown", &json!({}))?;
    assert_eq!(value_get(&response, "/result")?, json!(null));
    let _ = child.finish();
    Ok(())
}

#[test]
fn string_request_id_round_trips_through_dispatch() -> Result<()> {
    let mut child = spawn_and_init()?;
    // The harness only issues numeric ids; craft a raw frame with a
    // string id so we exercise RequestId::String on the wire.
    let frame = r#"{"jsonrpc":"2.0","id":"alpha","method":"tools/list"}"#;
    child.send_raw_line(frame)?;
    let response = child.read_frame()?;
    assert_eq!(value_get(&response, ID_POINTER)?, json!("alpha"));
    let _ = child.finish();
    Ok(())
}
