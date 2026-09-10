//! E2E coverage for [LSP-EDITOR-SURFACES] `deslop/virtualDocument`.
//!
//! Drives the real binary over stdio. Proves that the method renders
//! the canonical markdown for the three documented URI shapes —
//! `deslop://schema`, `deslop://report`, `deslop://cluster/<id>` — and
//! returns a structured JSON-RPC error on malformed input.

use std::{path::Path, thread, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{notification, session::FixtureSession, write_frame};

/// The two-file C# workspace every case in this suite is served from.
const FIXTURE: &str = "csharp-small";
const VIRTUAL_DOCUMENT: &str = "deslop/virtualDocument";
const REPORT_GET: &str = "deslop/reportGet";
/// How long the first analysis pass may take before the suite gives up
/// waiting for a cluster to appear.
const CLUSTER_POLL_ATTEMPTS: usize = 60;
const CLUSTER_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[test]
fn virtual_document_schema_returns_non_empty_markdown() -> Result<()> {
    let mut lsp = FixtureSession::open(FIXTURE)?;
    let response = virtual_document(&mut lsp, "deslop://schema")?;
    let body = result_string(&response)?;
    assert!(!body.is_empty(), "schema markdown must not be empty");
    assert!(
        body.to_ascii_lowercase().contains("schema")
            || body.contains("# ")
            || body.contains("Deslop"),
        "expected markdown-ish schema body; got: {body}"
    );
    drop(lsp);
    Ok(())
}

#[test]
fn virtual_document_report_returns_canonical_text() -> Result<()> {
    let mut lsp = FixtureSession::open(FIXTURE)?;
    open_fixture_files(&mut lsp.stdin, lsp.workspace.path())?;
    let response = virtual_document(&mut lsp, "deslop://report")?;
    let body = result_string(&response)?;
    assert!(!body.is_empty(), "report text must not be empty");
    assert!(
        body.lines().next().is_some_and(|line| {
            line.starts_with("deslop ")
                && line.contains(" file(s), ")
                && line.contains(" cluster(s), ")
                && line.contains(" hidden")
        }),
        "expected render_text summary header; got: {body}"
    );
    assert!(
        body.contains("\nrepo: ") && body.contains("\ncache: ") && body.contains("\n#1 ["),
        "expected canonical report sections from render_text; got: {body}"
    );
    // The mass-only cutover retired the action-hints block from the
    // rendered report; if it returns, the fat surface leaked back in.
    assert!(
        !body.contains("-- action hints --"),
        "retired action-hints section must not leak into render_text; got: {body}"
    );
    drop(lsp);
    Ok(())
}

#[test]
fn virtual_document_cluster_returns_cluster_markdown() -> Result<()> {
    let mut lsp = FixtureSession::open(FIXTURE)?;
    open_fixture_files(&mut lsp.stdin, lsp.workspace.path())?;
    let cluster_id = wait_for_first_cluster(&mut lsp)?;

    let uri = format!("deslop://cluster/{cluster_id}");
    let response = virtual_document(&mut lsp, &uri)?;
    let body = result_string(&response)?;
    assert!(
        body.contains(&cluster_id),
        "cluster markdown must embed its id; got: {body}"
    );
    assert!(
        body.contains(':') && (body.contains(".cs") || body.contains("bytes")),
        "cluster markdown must carry occurrence locations; got: {body}"
    );
    drop(lsp);
    Ok(())
}

#[test]
fn virtual_document_rejects_malformed_uri_with_invalid_params() -> Result<()> {
    let mut lsp = FixtureSession::open(FIXTURE)?;
    let response = virtual_document(&mut lsp, "http://not-a-deslop-uri")?;
    let error_code = response
        .get("error")
        .and_then(|err| err.get("code"))
        .and_then(Value::as_i64);
    assert_eq!(
        error_code,
        Some(-32_602),
        "malformed uri must return JSON-RPC invalid params; got: {response}"
    );
    drop(lsp);
    Ok(())
}

#[test]
fn virtual_document_rejects_unknown_cluster_id() -> Result<()> {
    let mut lsp = FixtureSession::open(FIXTURE)?;
    let response = virtual_document(&mut lsp, "deslop://cluster/does-not-exist")?;
    assert!(
        response.get("error").is_some(),
        "unknown cluster id must surface an error, not a fallback string: {response}"
    );
    drop(lsp);
    Ok(())
}

/// Sends `deslop/virtualDocument` for `uri` and returns the raw
/// JSON-RPC frame, error envelopes included — the callers assert on
/// both the rendered markdown and the structured error.
fn virtual_document(lsp: &mut FixtureSession, uri: &str) -> Result<Value> {
    lsp.call(VIRTUAL_DOCUMENT, &json!({ "uri": uri }))
}

/// Extracts the `result` string from a JSON-RPC response, surfacing the
/// full frame on error so failed tests show why they failed.
fn result_string(response: &Value) -> Result<String> {
    response
        .get("result")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("virtualDocument did not return a string: {response}"))
}

/// Opens the two fixture files so the LSP's debounce scheduler kicks
/// off analysis without waiting for a watcher event.
fn open_fixture_files(stdin: &mut std::process::ChildStdin, root: &Path) -> Result<()> {
    for name in ["Alpha.cs", "Beta.cs"] {
        let path = root.join(name);
        let uri = tower_lsp::lsp_types::Url::from_file_path(&path)
            .map_err(|()| anyhow!("fixture path not absolute: {}", path.display()))?;
        let text = std::fs::read_to_string(&path)?;
        write_frame(
            stdin,
            &notification(
                "textDocument/didOpen",
                &json!({
                    "textDocument": {
                        "uri": uri.as_str(),
                        "languageId": "csharp",
                        "version": 1,
                        "text": text
                    }
                }),
            )?,
        )?;
    }
    Ok(())
}

/// Polls `deslop/reportGet` until a cluster appears or the budget is spent.
fn wait_for_first_cluster(lsp: &mut FixtureSession) -> Result<String> {
    for _ in 0..CLUSTER_POLL_ATTEMPTS {
        let response = lsp.call(REPORT_GET, &json!({}))?;
        if let Some(id) = first_cluster_id(&response) {
            return Ok(id);
        }
        thread::sleep(CLUSTER_POLL_INTERVAL);
    }
    Err(anyhow!(
        "no cluster appeared in {CLUSTER_POLL_ATTEMPTS} polls at {CLUSTER_POLL_INTERVAL:?}"
    ))
}

fn first_cluster_id(response: &Value) -> Option<String> {
    response
        .pointer("/result/clusters/0/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
}
