//! Startup-surface tests: the version contract the binary prints and
//! the flags it accepts or refuses before any workspace is opened
//! ([DEPLOY-VERSION-CONTRACT]).

use anyhow::Result;
use common::{
    mcp_calls::{
        assert_binary_refuses_to_start, assert_version_manifest, binary_stdout,
        init_and_tool_payload,
    },
    mcp_wire::{EMBEDDING_PROVENANCE_POINTER, SESSION_TOOL},
    value_get,
};
use serde_json::{json, Value};

use crate::common;

/// Flags the binary accepts before any workspace is opened.
const VERSION_FLAG: &str = "--version";
const EMBEDDINGS_FLAG: &str = "--embeddings";

/// [REMOVE-STUB] `session` reads provenance from the state file and
/// never contacts Ollama, so the key is present whatever Ollama's
/// state — that is how the editor detects the disabled state.
fn assert_session_reports_provenance(reason: &str) -> Result<()> {
    let (child, snapshot) = init_and_tool_payload(SESSION_TOOL, &json!({}))?;
    assert!(
        snapshot.get("embedding_provenance").is_some(),
        "{reason}: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn prints_exact_version_contract() -> Result<()> {
    let stdout = binary_stdout(&[VERSION_FLAG])?;
    assert_eq!(
        String::from_utf8_lossy(&stdout),
        format!("deslop-mcp {}\n", common::mcp_calls::expected_version())
    );
    Ok(())
}

#[test]
fn prints_json_version_contract() -> Result<()> {
    let stdout = binary_stdout(&[VERSION_FLAG, "--json"])?;
    let value: Value = serde_json::from_slice(&stdout)?;
    assert_version_manifest(&value, "deslop-mcp", "mcp");
    Ok(())
}

#[test]
fn binary_starts_with_default_embedding_config() -> Result<()> {
    // [REMOVE-STUB] StateFileBackend reads provenance from the state
    // file; with stub removed, provenance may be null when no real
    // provider is selected. The key must still be present.
    let (child, snapshot) = init_and_tool_payload(SESSION_TOOL, &json!({}))?;
    assert!(
        value_get(&snapshot, EMBEDDING_PROVENANCE_POINTER)?.is_object()
            || value_get(&snapshot, EMBEDDING_PROVENANCE_POINTER)?.is_null(),
        "session must return embedding_provenance field: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn binary_starts_without_ollama_returns_provenance_field() -> Result<()> {
    // [REMOVE-STUB] StateFileBackend reads provenance from the state
    // file; Ollama is not contacted. Production no longer falls back
    // to a stub provider when Ollama is unreachable, but the
    // provenance key must always be present so the editor can detect
    // the disabled state.
    assert_session_reports_provenance("session must include embedding_provenance key")
}

/// `[LSP-EMBEDDING-CONSENT]` Audience: HUMAN. Issue #35. `StateFileBackend` does
/// not contact Ollama — the server always starts and `session-config` responds.
#[test]
fn binary_survives_when_required_ollama_endpoint_is_unreachable() -> Result<()> {
    assert_session_reports_provenance("session must respond even when Ollama is not running")
}

#[test]
fn binary_rejects_invalid_embedding_mode_string() -> Result<()> {
    // Spawn the binary directly (bypassing the McpChild harness) so we
    // can assert on the non-zero exit status.
    assert_binary_refuses_to_start(
        &[EMBEDDINGS_FLAG, "nonsense"],
        "invalid --embeddings value must not succeed",
    )
}

#[test]
fn binary_rejects_unknown_embedding_provider_at_init() -> Result<()> {
    assert_binary_refuses_to_start(
        &[
            EMBEDDINGS_FLAG,
            "auto",
            "--embedding-provider",
            "zzz-not-real",
        ],
        "unknown embedding provider must exit non-zero",
    )
}
