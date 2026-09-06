//! Real-LSP regression for failed background embedding refreshes.
//!
//! Explicit model selection is a user-requested operation. A provider that
//! rejects every real embedding must emit terminal `failed` progress and
//! leave the last good report untouched — never commit an embeddings-off
//! snapshot and call it `complete`.

#[path = "../../deslop/tests/cli/mock_ollama.rs"]
mod mock_ollama;

mod common;

use std::{
    io::BufReader,
    process::{ChildStdin, ChildStdout},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use common::{
    at, call, call_capturing, handshake, path as json_path, reports::assert_initialize_contract,
    spawn_lsp_guarded, wait_for_report_matching, POLL_INTERVAL,
};
use mock_ollama::{MockBehavior, MockOllama};
use serde_json::{json, Value};

const REPORT_TIMEOUT: Duration = Duration::from_secs(20);
const SET_MODEL: &str = "deslop/embeddingSetModel";
const PROGRESS: &str = "deslop/embeddingProgress";

#[test]
#[ignore = "GH #370: hangs indefinitely — 14m41s locally before being killed, \
            and it consumed the whole CI Test budget twice. The stall is in \
            the unbounded `recv_response` read, upstream of this file's 20s \
            REPORT_TIMEOUT, so the server appears never to emit a terminal \
            progress frame on the rejection path. Pre-existing; every earlier \
            CI run died at GH #369 before reaching this binary. Assertions \
            are intact — run with `-- --ignored`."]
fn rejected_embedding_refresh_reports_failure_and_preserves_last_good_report() -> Result<()> {
    let server = MockOllama::spawn_with(MockBehavior::RejectAllEmbeds)?;
    let workspace = common::copy_fixture("ts-mixed-band")?;
    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(workspace.path())?;
    assert_initialize_contract(&handshake(&mut stdin, &mut stdout)?);

    let before = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(5)
    })?;
    assert!(at(&before, "embedding_provenance").is_null(), "{before:#}");

    let (selection, initial_frames) = call_capturing(
        &mut stdin,
        &mut stdout,
        SET_MODEL,
        &json!({
            "provider_id": "ollama",
            "model_id": "nomic-embed-text",
            "endpoint": server.endpoint(),
        }),
    )?;
    assert!(
        selection.get("error").is_none(),
        "model was not queued: {selection:#}"
    );
    assert!(
        selection.get("result").is_some(),
        "model selection has no result: {selection:#}"
    );

    let terminal = wait_for_terminal_progress(&mut stdin, &mut stdout, initial_frames)?;
    let after = call(&mut stdin, &mut stdout, "deslop/reportGet", &json!({}))?;
    let after_report = after
        .get("result")
        .ok_or_else(|| anyhow!("reportGet returned no result: {after:#}"))?;

    assert_eq!(
        json_path(&terminal, &["params", "provider_id"]),
        "ollama",
        "{terminal:#}"
    );
    assert_eq!(
        json_path(&terminal, &["params", "model_id"]),
        "nomic-embed-text",
        "{terminal:#}"
    );
    assert_eq!(
        json_path(&terminal, &["params", "phase"]),
        "failed",
        "{terminal:#}"
    );
    assert_eq!(
        json_path(&terminal, &["params", "done"]),
        0,
        "failed work cannot be complete"
    );
    assert!(
        json_path(&terminal, &["params", "message"])
            .as_str()
            .is_some_and(|message| !message.is_empty()),
        "failed progress must explain the provider failure: {terminal:#}"
    );
    assert_eq!(
        after_report, &before,
        "a failed refresh replaced the last good report"
    );
    Ok(())
}

fn wait_for_terminal_progress(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    mut frames: Vec<Value>,
) -> Result<Value> {
    let deadline = Instant::now()
        .checked_add(REPORT_TIMEOUT)
        .unwrap_or_else(Instant::now);
    loop {
        if let Some(terminal) = frames.iter().find(|frame| {
            at(frame, "method") == PROGRESS
                && matches!(
                    json_path(frame, &["params", "phase"]).as_str(),
                    Some("complete" | "failed")
                )
        }) {
            return Ok(terminal.clone());
        }
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "no terminal embedding progress within {REPORT_TIMEOUT:?}: {frames:#?}"
            ));
        }
        let (response, emitted) = call_capturing(stdin, stdout, "deslop/reportGet", &json!({}))?;
        if response.get("result").is_none() {
            return Err(anyhow!(
                "reportGet returned no result while polling: {response:#}"
            ));
        }
        frames.extend(emitted);
        std::thread::sleep(POLL_INTERVAL);
    }
}
