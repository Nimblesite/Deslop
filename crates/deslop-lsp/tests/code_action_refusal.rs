//! E2E coverage for surfacing refusals computed at `codeAction/resolve`
//! time (issue #282, [AUTOFIX-MERGE-CODE-ACTION]).
//!
//! VS Code ignores a `disabled` field attached during resolve — the LSP
//! spec only honours `disabled` on the `textDocument/codeAction`
//! response — so a plan that refuses at resolve time must ALSO be
//! announced with a `window/showMessage` warning, or the user's click
//! silently does nothing.

use crate::common;

use anyhow::{ensure, Context, Result};
use common::{
    call_capturing, code_action_params, handshake, rewrite_offer, spawn_lsp_on_fixture_guarded,
    wait_for_actions, workspace_file_uri,
};
use serde_json::{json, Value};

/// `window/showMessage` `MessageType::WARNING` wire value.
const WARNING: u64 = 2;

/// Resolving a refused merge must emit a `window/showMessage` warning
/// carrying the refusal reason — the `disabled` field alone is invisible
/// in VS Code, so without the message the click is a silent no-op
/// (issue #282).
#[test]
fn refused_resolve_surfaces_showmessage_warning() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) =
        spawn_lsp_on_fixture_guarded("csharp-merge-leafdrift")?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let uri = workspace_file_uri(workspace.path(), "DriftLimits.cs")?;
    let params = code_action_params(uri.as_str(), 4, 6);
    let actions = wait_for_actions(&mut stdin, &mut stdout, &params)?;
    let offer = rewrite_offer(&actions, "Merge duplicates into one parameterised helper")?;

    let (resolved, mut frames) =
        call_capturing(&mut stdin, &mut stdout, "codeAction/resolve", offer)?;
    let reason = resolved
        .pointer("/result/disabled/reason")
        .and_then(Value::as_str)
        .context("refusal reason present on the resolved action")?
        .to_owned();
    // Fence round-trip: anything the server sent while resolving is
    // flushed before this response, so the capture is deterministic.
    let (_fence, late_frames) =
        call_capturing(&mut stdin, &mut stdout, "deslop/reportGet", &json!({}))?;
    frames.extend(late_frames);

    let message = frames
        .iter()
        .find(|frame| {
            frame.pointer("/method").and_then(Value::as_str) == Some("window/showMessage")
        })
        .context("refusal must emit a window/showMessage warning (issue #282)")?;
    ensure!(
        message.pointer("/params/type").and_then(Value::as_u64) == Some(WARNING),
        "refusal message must be a warning, got {message}"
    );
    let text = message
        .pointer("/params/message")
        .and_then(Value::as_str)
        .context("warning carries text")?;
    ensure!(
        text.contains(&reason),
        "warning must include the refusal reason `{reason}`, got `{text}`"
    );
    Ok(())
}
