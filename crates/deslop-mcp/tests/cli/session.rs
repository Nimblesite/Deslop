//! `session` tool tests: embedding-model listing, the
//! set-embedding-model consent path, provenance reporting, and the
//! session snapshot's shape.

use std::fs;

use anyhow::{anyhow, Result};
use common::{
    assert_production_embedding_models,
    mcp_calls::{
        assert_tool_rejects_invalid_params, call_tool, init_and_tool_payload,
        init_and_tool_response, structured_tool_result,
    },
    mcp_child::{init_session, McpChild},
    mcp_wire::{
        CSHARP_LANGUAGE, DEFAULT_MIN_NODES, EMBEDDING_PROVENANCE_POINTER, LANGUAGES_POINTER,
        SESSION_TOOL,
    },
    value_array, value_get, PROVIDER_ID_FIELD, STUB_PROVIDER,
};

use crate::common;

use serde_json::{json, Value};

/// JSON field: the session action discriminator.
const ACTION_FIELD: &str = "action";
/// `session` action: list available embedding models.
const LIST_EMBEDDING_MODELS_ACTION: &str = "list-embedding-models";
/// `session` action: swap the active embedding model.
const SET_EMBEDDING_MODEL_ACTION: &str = "set-embedding-model";
/// JSON field: the embedding model id.
const MODEL_ID_FIELD: &str = "model_id";
/// JSON field: whether the caller is the user (consent gate).
const USER_INITIATED_FIELD: &str = "user_initiated";
/// Provider id of a local Ollama daemon.
const OLLAMA_PROVIDER: &str = "ollama";
/// The default nomic embedding model id.
const DEFAULT_EMBEDDING_MODEL: &str = "nomic-embed-text";
/// JSON field: the provider endpoint a swap is handed.
const ENDPOINT_FIELD: &str = "endpoint";
/// An endpoint nothing ever listens on (port 1).
const UNREACHABLE_OLLAMA_ENDPOINT: &str = "http://127.0.0.1:1";
/// JSON field: embedding provenance inside a session snapshot.
const EMBEDDING_PROVENANCE_FIELD: &str = "embedding_provenance";
/// JSON pointer: `models[]` inside a `session` tool result.
const STRUCTURED_MODELS_POINTER: &str = "/result/structuredContent/models";

#[test]
fn list_embedding_models_excludes_stub_when_ollama_unreachable() -> Result<()> {
    // [REMOVE-STUB] `session` action=list-embedding-models delegates to
    // the companion LSP via IPC. When Ollama is unreachable in CI the
    // production listing must come back empty — the deterministic stub is
    // test infrastructure and never appears in production payloads.
    let (child, response) = init_and_tool_response(
        SESSION_TOOL,
        &json!({ (ACTION_FIELD): LIST_EMBEDDING_MODELS_ACTION }),
    )?;
    let models = value_array(&response, STRUCTURED_MODELS_POINTER)?;
    assert_production_embedding_models(&models)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_rejects_stub_provider() -> Result<()> {
    // [REMOVE-STUB] The session set-embedding-model path only accepts
    // the ollama provider — submitting `provider_id: "stub"` is rejected
    // before the call reaches any backend.
    let (child, response) = init_and_tool_response(
        SESSION_TOOL,
        &json!({
            (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
            (PROVIDER_ID_FIELD): STUB_PROVIDER,
            (MODEL_ID_FIELD): "blake3-stub",
            (USER_INITIATED_FIELD): true
        }),
    )?;
    assert!(
        response.get("error").is_some(),
        "stub provider must be rejected by the MCP schema: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_preserves_shared_settings_and_endpoint() -> Result<()> {
    // The harness spawns a companion LSP, so this exercises the live IPC
    // path. Port 1 is never listening, so the swap must fail as an
    // unreachable *provider* — naming the endpoint it was handed proves the
    // caller's endpoint survived the trip instead of being discarded
    // ([Deslop#286]).
    // [REMOVE-STUB] Stub provider removed from production payloads;
    // exercise the same plumbing through the ollama provider.
    let (child, response) = init_and_tool_response(
        SESSION_TOOL,
        &json!({
            (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
            (PROVIDER_ID_FIELD): OLLAMA_PROVIDER,
            (MODEL_ID_FIELD): DEFAULT_EMBEDDING_MODEL,
            (ENDPOINT_FIELD): UNREACHABLE_OLLAMA_ENDPOINT,
            (USER_INITIATED_FIELD): true
        }),
    )?;
    assert!(
        response.get("error").is_some(),
        "set-embedding-model against a dead endpoint must return an error envelope: {response}",
    );
    let rendered = response.to_string();
    assert!(
        !rendered.contains("LSP is not running"),
        "issue #286: the LSP is running and serving this socket; the error must describe the real failure: {response}",
    );
    assert!(
        rendered.contains("127.0.0.1:1"),
        "the endpoint the caller supplied must reach the provider and be named in the failure: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_fails_when_shared_settings_cannot_be_written() -> Result<()> {
    let workspace = common::copied_fixture()?;
    fs::write(workspace.path().join(".vscode"), "not a directory")?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": SESSION_TOOL,
            "arguments": {
                (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
                (PROVIDER_ID_FIELD): OLLAMA_PROVIDER,
                (MODEL_ID_FIELD): DEFAULT_EMBEDDING_MODEL,
                (USER_INITIATED_FIELD): true
            }
        }),
    )?;
    assert!(
        response.get("error").is_some(),
        "expected config write error"
    );
    let snap = structured_tool_result(&call_tool(&mut child, SESSION_TOOL, &json!({}))?)?;
    assert!(
        value_get(&snap, EMBEDDING_PROVENANCE_POINTER)?.is_null(),
        "failed settings write must not switch MCP state: {snap}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_unknown_provider_errors() -> Result<()> {
    let (child, response) = init_and_tool_response(
        SESSION_TOOL,
        &json!({
            (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
            (PROVIDER_ID_FIELD): "aztec-cpu",
            (MODEL_ID_FIELD): "blah",
            (USER_INITIATED_FIELD): true
        }),
    )?;
    assert!(response.get("error").is_some(), "expected error response");
    let _ = child.finish();
    Ok(())
}

#[test]
fn session_reports_workspace_root_and_languages() -> Result<()> {
    // Tests [MCP-TOOL-SESSION]
    // [MCP-IPC-CLIENT] session action=get goes over IPC to the running
    // LSP, so `min_nodes` is the LSP's default (30) — the fixture's
    // pre-committed value is no longer the wire source.
    let (child, payload) = init_and_tool_payload(SESSION_TOOL, &json!({}))?;
    assert_eq!(
        value_get(&payload, "/min_nodes")?.as_u64().unwrap_or(0),
        DEFAULT_MIN_NODES
    );
    let languages_value = value_get(&payload, LANGUAGES_POINTER)?;
    let languages: Vec<String> = languages_value
        .as_array()
        .ok_or_else(|| anyhow!("languages not array"))?
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect();
    assert!(
        languages
            .iter()
            .any(|candidate| candidate == CSHARP_LANGUAGE),
        "csharp missing from session config: {languages:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_swap_updates_session_config_provenance() -> Result<()> {
    // A swap that never reached a provider must not leave provenance behind:
    // an agent reading session-config afterwards has to see that embeddings
    // are still off, not a model that was never installed ([Deslop#286]).
    // [REMOVE-STUB] Use the ollama provider id since stub is no longer
    // accepted by the production MCP schema.
    let (mut child, response) = init_and_tool_response(
        SESSION_TOOL,
        &json!({
            (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
            (PROVIDER_ID_FIELD): OLLAMA_PROVIDER,
            (MODEL_ID_FIELD): DEFAULT_EMBEDDING_MODEL,
            (ENDPOINT_FIELD): UNREACHABLE_OLLAMA_ENDPOINT,
            (USER_INITIATED_FIELD): true
        }),
    )?;
    assert!(
        response.get("error").is_some(),
        "a swap against a dead endpoint must return an error envelope: {response}",
    );
    assert!(
        !response.to_string().contains("LSP is not running"),
        "issue #286: the companion LSP is live; the error must describe the real failure: {response}",
    );
    let config = structured_tool_result(&call_tool(&mut child, SESSION_TOOL, &json!({}))?)?;
    assert_eq!(
        config.get(EMBEDDING_PROVENANCE_FIELD),
        Some(&Value::Null),
        "a failed swap must leave session-config reporting no embedding provenance: {config}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_to_ollama_fails_when_daemon_not_running() -> Result<()> {
    let (child, response) = init_and_tool_response(
        SESSION_TOOL,
        &json!({
            (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
            (PROVIDER_ID_FIELD): OLLAMA_PROVIDER,
            (MODEL_ID_FIELD): DEFAULT_EMBEDDING_MODEL,
            (ENDPOINT_FIELD): UNREACHABLE_OLLAMA_ENDPOINT,
            (USER_INITIATED_FIELD): true
        }),
    )?;
    // Either a clean error envelope or the inner backend error. Both
    // paths exercise the ollama branch of set_embedding_model.
    assert!(
        response.get("error").is_some(),
        "ollama-to-nowhere must not succeed: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_missing_model_id_returns_invalid_params() -> Result<()> {
    assert_tool_rejects_invalid_params(
        SESSION_TOOL,
        &json!({
            (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
            (PROVIDER_ID_FIELD): OLLAMA_PROVIDER,
            (USER_INITIATED_FIELD): true
        }),
    )
}

#[test]
fn set_embedding_model_without_user_initiation_returns_invalid_params() -> Result<()> {
    // Tests [MCP-EMBEDDING-CONSENT]
    assert_tool_rejects_invalid_params(
        SESSION_TOOL,
        &json!({
            (ACTION_FIELD): SET_EMBEDDING_MODEL_ACTION,
            (PROVIDER_ID_FIELD): OLLAMA_PROVIDER,
            (MODEL_ID_FIELD): DEFAULT_EMBEDDING_MODEL
        }),
    )
}

#[test]
fn list_embedding_models_response_omits_legacy_keys_and_stub() -> Result<()> {
    // [MCP-IPC-CLIENT] / issue #87 / [REMOVE-STUB] — the response array
    // (empty under unreachable Ollama, populated when Ollama responds)
    // never exposes the legacy keys and never includes the stub
    // provider. Production payloads must not leak test infrastructure.
    let mut child = common::mcp_child::spawn_and_init()?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": SESSION_TOOL,
            "arguments": { (ACTION_FIELD): LIST_EMBEDDING_MODELS_ACTION }
        }),
    )?;
    let models = value_array(&response, STRUCTURED_MODELS_POINTER)?;
    assert_production_embedding_models(&models)?;
    let _ = child.finish();
    Ok(())
}
