//! End-to-end coverage for issue #35: when the Ollama embedding provider
//! is unreachable, `deslop-lsp` must stay alive and serve a clean protocol —
//! VS Code observed a crash-loop because the backend called
//! `std::process::exit(1)` on provider connect failure.
//!
//! Audience: HUMAN. Since #175 the LSP no longer accepts embedding startup
//! CLI flags (`--embeddings*` are rejected as legacy); the editor configures
//! the provider *after* `initialize` via the `deslop/embeddingSetModel`
//! request — exactly what the VS Code client's `syncEmbeddingSettingsToLsp`
//! sends. The #35 invariant is unchanged: pointing the running LSP at an
//! unreachable provider must degrade gracefully — a clean JSON-RPC reply and
//! a live process — never a crash that loops the editor's restart logic.

use crate::common;

use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use common::{call, request, spawn_lsp_on_fixture, write_frame};
use serde_json::json;

/// Loopback port nothing should listen on. Connection attempts fail with
/// ECONNREFUSED instantly, reproducing the unreachable provider scenario
/// without waiting on DNS.
const UNREACHABLE_ENDPOINT: &str = "http://127.0.0.1:1";

/// How long we give the LSP to run past the point where it would have
/// crashed on the unreachable-provider path. 3 seconds is generous.
const LIVENESS_WINDOW: Duration = Duration::from_secs(3);

/// Builds the `deslop/embeddingSetModel` request that points the LSP at the
/// unreachable Ollama endpoint — the supported, settings-driven path.
fn unreachable_set_model() -> Result<(i64, String)> {
    request(
        "deslop/embeddingSetModel",
        &json!({
            "provider_id": "ollama",
            "model_id": "nomic-embed-text",
            "endpoint": UNREACHABLE_ENDPOINT,
        }),
    )
}

/// Audience: HUMAN. Issue #35. Pointing the running LSP at an unreachable
/// Ollama endpoint must not kill the process. Positive invariant: the child
/// is still running after the liveness window. Liveness is checked directly
/// on the child (never a blocking read) so a crash can never hang the harness.
#[test]
fn lsp_survives_when_configured_ollama_endpoint_is_unreachable() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = common::handshake(&mut stdin, &mut stdout)?;

    let (_set_id, set_model) = unreachable_set_model()?;
    write_frame(&mut stdin, &set_model)?;

    let deadline = Instant::now() + LIVENESS_WINDOW;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            let _ = child.kill();
            return Err(anyhow!(
                "deslop-lsp exited with status {status:?} — the LSP must stay \
                 alive when the configured Ollama endpoint is unreachable",
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }

    // Keep stdin alive until the test finishes so the kept-open handle is
    // what ends the liveness window, not an EOF-triggered clean shutdown.
    drop(stdin);
    let _ = child.kill();
    Ok(())
}

/// Audience: HUMAN. Issue #35. Beyond merely surviving, the LSP must keep
/// *serving* after the unreachable provider is configured: the
/// `embeddingSetModel` request itself returns a clean JSON-RPC reply (the
/// provider failure is reported in-band, not as a transport crash). A reply
/// at all proves the server processed the request and is still answering.
#[test]
fn lsp_survives_when_required_ollama_endpoint_is_unreachable() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = common::handshake(&mut stdin, &mut stdout)?;

    let reply = call(
        &mut stdin,
        &mut stdout,
        "deslop/embeddingSetModel",
        &json!({
            "provider_id": "ollama",
            "model_id": "nomic-embed-text",
            "endpoint": UNREACHABLE_ENDPOINT,
        }),
    )
    .map_err(|error| {
        anyhow!(
            "deslop-lsp did not reply to embeddingSetModel against an \
             unreachable endpoint — a crashed server closes stdout: {error}"
        )
    })?;
    assert!(
        reply.get("result").is_some() || reply.get("error").is_some(),
        "embeddingSetModel must return an in-band JSON-RPC reply (result or \
         error), never crash the transport: {reply}"
    );

    // The server is still answering: a follow-up request also gets a reply.
    let config = call(&mut stdin, &mut stdout, "deslop/sessionConfig", &json!({}))?;
    assert!(
        config.get("result").is_some(),
        "LSP must keep serving after an unreachable embeddingSetModel: {config}"
    );

    let _ = child.kill();
    Ok(())
}
