//! E2E coverage for [LSP-EDITOR-SURFACES] `textDocument/definition`.
//!
//! Proves that placing the cursor inside a clone range lands the user on
//! the canonical occurrence in a sibling file. Drives the real LSP
//! binary over stdio — no mocked transport.

mod common;

use std::{path::Path, thread, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{call, copy_fixture, handshake, notification, spawn_lsp, take_io, write_frame};

const REPORT_GET: &str = "deslop/reportGet";
const DEFINITION: &str = "textDocument/definition";

#[test]
fn definition_capability_is_advertised_on_initialize() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let init = handshake(&mut stdin, &mut stdout)?;

    let provider = init
        .pointer("/result/capabilities/definitionProvider")
        .ok_or_else(|| anyhow!("definitionProvider capability missing: {init}"))?;
    assert!(
        provider.is_boolean() || provider.is_object(),
        "definitionProvider must be advertised as bool or object; got: {provider}"
    );
    let _ = child.kill();
    Ok(())
}

#[test]
fn definition_jumps_to_canonical_occurrence_in_sibling_file() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    open_fixture_files(&mut stdin, workspace.path())?;

    let cursor = cursor_inside_clone(&mut stdin, &mut stdout, &alpha)?;
    let response = call(
        &mut stdin,
        &mut stdout,
        DEFINITION,
        &json!({
            "textDocument": { "uri": file_uri(&alpha)? },
            "position": cursor
        }),
    )?;

    let result = response
        .get("result")
        .ok_or_else(|| anyhow!("definition response missing result: {response}"))?;
    let target_uri = result
        .pointer("/uri")
        .or_else(|| result.pointer("/0/uri"))
        .or_else(|| result.pointer("/0/targetUri"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("no uri in definition response: {response}"))?;
    assert!(
        target_uri.contains("Beta.cs"),
        "expected canonical occurrence in Beta.cs; got: {target_uri}"
    );
    let _ = child.kill();
    Ok(())
}

#[test]
fn definition_returns_none_for_a_file_with_no_clone() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let lonely = workspace.path().join("Lonely.cs");
    std::fs::write(
        &lonely,
        "namespace Lonely { public class Unique { public int Seven() { return 7; } } }\n",
    )?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    open_fixture_files(&mut stdin, workspace.path())?;
    write_frame(
        &mut stdin,
        &notification(
            "textDocument/didOpen",
            &json!({
                "textDocument": {
                    "uri": file_uri(&lonely)?,
                    "languageId": "csharp",
                    "version": 1,
                    "text": std::fs::read_to_string(&lonely)?
                }
            }),
        )?,
    )?;

    let response = call(
        &mut stdin,
        &mut stdout,
        DEFINITION,
        &json!({
            "textDocument": { "uri": file_uri(&lonely)? },
            "position": { "line": 0, "character": 0 }
        }),
    )?;

    let is_null = response.get("result").is_some_and(|result| {
        result.is_null() || matches!(result.as_array(), Some(arr) if arr.is_empty())
    });
    assert!(
        is_null,
        "cursor in non-duplicate Lonely.cs should resolve to no definition; got: {response}"
    );
    let _ = child.kill();
    Ok(())
}

/// Opens the fixture files over `textDocument/didOpen` so the scheduler
/// begins analysis immediately.
fn open_fixture_files(stdin: &mut std::process::ChildStdin, root: &Path) -> Result<()> {
    for name in ["Alpha.cs", "Beta.cs"] {
        let path = root.join(name);
        let text = std::fs::read_to_string(&path)?;
        write_frame(
            stdin,
            &notification(
                "textDocument/didOpen",
                &json!({
                    "textDocument": {
                        "uri": file_uri(&path)?,
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

/// Waits for the first cluster whose occurrence is in `target` and
/// returns a line/character position inside that occurrence range.
fn cursor_inside_clone(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
    target: &Path,
) -> Result<Value> {
    for _ in 0..60 {
        let response = call(stdin, stdout, REPORT_GET, &json!({}))?;
        if let Some(cursor) = cursor_from_report(&response, target) {
            return Ok(cursor);
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err(anyhow!("no cluster covering {} in 30s", target.display()))
}

fn cursor_from_report(response: &Value, target: &Path) -> Option<Value> {
    let clusters = response.pointer("/result/clusters")?.as_array()?;
    let target_name = target.file_name()?;
    for cluster in clusters {
        let occurrences = cluster.get("occurrences")?.as_array()?;
        for occurrence in occurrences {
            let path = occurrence.get("path")?.as_str()?;
            if Path::new(path).file_name() != Some(target_name) {
                continue;
            }
            let start = occurrence.get("start_byte")?.as_u64()?;
            let end = occurrence.get("end_byte")?.as_u64()?;
            if end <= start {
                continue;
            }
            let body = std::fs::read_to_string(target).ok()?;
            let byte = usize::try_from(start).ok()?.saturating_add(1);
            let (line, character) = byte_position(&body, byte);
            return Some(json!({ "line": line, "character": character }));
        }
    }
    None
}

/// Mirrors `position::position_for_byte` without the tower-lsp
/// dependency so the harness stays lean.
fn byte_position(body: &str, byte: usize) -> (usize, usize) {
    let capped = byte.min(body.len());
    let prefix = body.as_bytes().get(..capped).unwrap_or(&[]);
    let line = count_newlines(prefix);
    let col = match prefix.iter().rposition(|b| *b == b'\n') {
        Some(nl) => capped.saturating_sub(nl).saturating_sub(1),
        None => capped,
    };
    (line, col)
}

fn count_newlines(bytes: &[u8]) -> usize {
    let mut count = 0_usize;
    for byte in bytes {
        if *byte == b'\n' {
            count = count.saturating_add(1);
        }
    }
    count
}

fn file_uri(path: &Path) -> Result<String> {
    tower_lsp::lsp_types::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|()| anyhow!("path not absolute: {}", path.display()))
}
