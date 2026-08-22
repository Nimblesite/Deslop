//! E2E coverage for [LSP-EDITOR-SURFACES] `deslop/virtualDocument`.
//!
//! Drives the real binary over stdio. Proves that the method renders
//! the canonical markdown for the three documented URI shapes —
//! `deslop://schema`, `deslop://report`, `deslop://cluster/<id>` — and
//! returns a structured JSON-RPC error on malformed input.


use std::{path::Path, thread, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{call, handshake, notification, spawn_lsp_on_fixture, write_frame};

const VIRTUAL_DOCUMENT: &str = "deslop/virtualDocument";
const REPORT_GET: &str = "deslop/reportGet";

#[test]
fn virtual_document_schema_returns_non_empty_markdown() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    let response = call(
        &mut stdin,
        &mut stdout,
        VIRTUAL_DOCUMENT,
        &json!({ "uri": "deslop://schema" }),
    )?;
    let body = result_string(&response)?;
    assert!(!body.is_empty(), "schema markdown must not be empty");
    assert!(
        body.to_ascii_lowercase().contains("schema")
            || body.contains("# ")
            || body.contains("Deslop"),
        "expected markdown-ish schema body; got: {body}"
    );
    let _ = child.kill();
    Ok(())
}

#[test]
fn virtual_document_report_returns_canonical_text() -> Result<()> {
    let (workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    open_fixture_files(&mut stdin, workspace.path())?;

    let response = call(
        &mut stdin,
        &mut stdout,
        VIRTUAL_DOCUMENT,
        &json!({ "uri": "deslop://report" }),
    )?;
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
        body.contains("\nrepo: ") && body.contains("\n-- action hints --\n"),
        "expected canonical report sections from render_text; got: {body}"
    );
    let _ = child.kill();
    Ok(())
}

#[test]
fn virtual_document_cluster_returns_cluster_markdown() -> Result<()> {
    let (workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    open_fixture_files(&mut stdin, workspace.path())?;
    let cluster_id = wait_for_first_cluster(&mut stdin, &mut stdout)?;

    let uri = format!("deslop://cluster/{cluster_id}");
    let response = call(
        &mut stdin,
        &mut stdout,
        VIRTUAL_DOCUMENT,
        &json!({ "uri": uri }),
    )?;
    let body = result_string(&response)?;
    assert!(
        body.contains(&cluster_id),
        "cluster markdown must embed its id; got: {body}"
    );
    assert!(
        body.contains(':') && (body.contains(".cs") || body.contains("bytes")),
        "cluster markdown must carry occurrence locations; got: {body}"
    );
    let _ = child.kill();
    Ok(())
}

#[test]
fn virtual_document_rejects_malformed_uri_with_invalid_params() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    let response = call(
        &mut stdin,
        &mut stdout,
        VIRTUAL_DOCUMENT,
        &json!({ "uri": "http://not-a-deslop-uri" }),
    )?;
    let error_code = response
        .get("error")
        .and_then(|err| err.get("code"))
        .and_then(Value::as_i64);
    assert_eq!(
        error_code,
        Some(-32_602),
        "malformed uri must return JSON-RPC invalid params; got: {response}"
    );
    let _ = child.kill();
    Ok(())
}

#[test]
fn virtual_document_rejects_unknown_cluster_id() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    let response = call(
        &mut stdin,
        &mut stdout,
        VIRTUAL_DOCUMENT,
        &json!({ "uri": "deslop://cluster/does-not-exist" }),
    )?;
    assert!(
        response.get("error").is_some(),
        "unknown cluster id must surface an error, not a fallback string: {response}"
    );
    let _ = child.kill();
    Ok(())
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
fn wait_for_first_cluster(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
) -> Result<String> {
    for _ in 0..60 {
        let response = call(stdin, stdout, REPORT_GET, &json!({}))?;
        if let Some(id) = first_cluster_id(&response) {
            return Ok(id);
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err(anyhow!("no cluster appeared in 30s"))
}

fn first_cluster_id(response: &Value) -> Option<String> {
    response
        .pointer("/result/clusters/0/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
}
