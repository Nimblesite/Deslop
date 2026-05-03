//! End-to-end coverage for the hover panel the **human** reads.
//!
//! Every assertion here is framed as what the human on the other side
//! of VS Code must see. Raw numeric signal scores, academic clone-
//! taxonomy labels, and machine-readable breakdowns belong on dedicated
//! agent surfaces (`deslop/reportGet`, diagnostic `data`, Copy-for-AI
//! commands) — not in the tooltip body.
//!
//! Drives the real `deslop-lsp` binary over stdio. No mocked transport,
//! per `CLAUDE.md` ("Testing any UI/Extension with a fake LSP/MCP =
//! ILLEGAL").

mod common;

use std::{path::Path, thread, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{call, copy_fixture, handshake, notification, spawn_lsp, take_io, write_frame};

const HOVER: &str = "textDocument/hover";
const REPORT_GET: &str = "deslop/reportGet";

/// Audience: HUMAN. Issue #31 / redesign at the LSP hover layer.
///
/// The human hover card is intentionally minimal: one bold title
/// (clone verdict + action sentence + occurrence count). The occurrence
/// list and signal scores are agent-only content and must NOT appear in
/// the human panel — they belong in `deslop/reportGet`, diagnostic
/// `data`, and Copy-for-AI commands.
#[test]
fn hover_body_is_compact_title_only_for_humans() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    open_fixture_files(&mut stdin, workspace.path(), &["Alpha.cs", "Beta.cs"])?;

    let cursor = cursor_inside_clone(&mut stdin, &mut stdout, &alpha)?;
    let response = call(
        &mut stdin,
        &mut stdout,
        HOVER,
        &json!({
            "textDocument": { "uri": file_uri(&alpha)? },
            "position": cursor,
        }),
    )?;
    let markdown = hover_markdown(&response)?;

    let sub_bullets = top_level_sub_bullets(&markdown);
    assert!(
        sub_bullets.is_empty(),
        "human hover must have no sub-bullets — occurrence list is agent-only. Got: {markdown}"
    );
    assert!(
        markdown.contains("occurrences"),
        "human hover must still state the total occurrence count. Got: {markdown}"
    );
    assert!(
        !markdown.contains("structural"),
        "human hover must not expose raw signal scores. Got: {markdown}"
    );

    let _ = child.kill();
    Ok(())
}

/// Audience: HUMAN. Issue #33.
///
/// When a physical clone fingerprints both as an exact subtree and as
/// a wider sibling window — which happens whenever a duplicated block
/// is long enough for the two-pass detector — the pipeline produces
/// multiple clusters covering overlapping byte ranges in the same
/// file. The hover panel is the human's single-glance surface, so it
/// must consolidate these into one card per physical clone. Two cards
/// stacked on one region forces the reader to compare two breakdowns
/// of the same duplication.
#[test]
fn hover_surfaces_one_card_even_when_multiple_clusters_overlap_the_cursor() -> Result<()> {
    let workspace = copy_fixture("csharp-unrelated-xunit-tests")?;
    let endpoint = workspace.path().join("EndpointWorkflowTests.cs");
    // EndpointWorkflowTests exposes a broad request/response workflow
    // clone plus narrower nested clones under the documented
    // min_nodes=30 default, so the test does not rely on removed startup
    // config.
    let mut child = spawn_lsp(workspace.path())?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    open_fixture_files(
        &mut stdin,
        workspace.path(),
        &[
            "EndpointWorkflowTests.cs",
            "GenerateEndpointTests.cs",
            "ProgramConfigTests.cs",
        ],
    )?;

    let (cursor, overlap_count) =
        cursor_inside_overlapping_clusters(&mut stdin, &mut stdout, &endpoint)?;
    assert!(
        overlap_count >= 2,
        "fixture must reproduce the overlapping-cluster case so the test is meaningful; got overlap={overlap_count}"
    );

    let response = call(
        &mut stdin,
        &mut stdout,
        HOVER,
        &json!({
            "textDocument": { "uri": file_uri(&endpoint)? },
            "position": cursor,
        }),
    )?;
    let markdown = hover_markdown(&response)?;

    let card_headlines = count_card_headlines(&markdown);
    assert_eq!(
        card_headlines, 1,
        "human hover must render exactly one card per physical clone even when \
         {overlap_count} clusters overlap the cursor; got {card_headlines} headlines in: {markdown}"
    );

    let _ = child.kill();
    Ok(())
}

/// Opens the listed `.cs` files over `didOpen` so the scheduler begins
/// analysis immediately.
fn open_fixture_files(
    stdin: &mut std::process::ChildStdin,
    root: &Path,
    names: &[&str],
) -> Result<()> {
    for name in names {
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
                        "text": text,
                    }
                }),
            )?,
        )?;
    }
    Ok(())
}

/// Polls `deslop/reportGet` until a cluster references `target`, then
/// returns a cursor just inside that occurrence.
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

/// Polls `deslop/reportGet` until at least two cluster occurrences in
/// `target` overlap on a byte. Returns a cursor inside that overlap
/// region along with the observed overlap count.
fn cursor_inside_overlapping_clusters(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
    target: &Path,
) -> Result<(Value, usize)> {
    for _ in 0..60 {
        let response = call(stdin, stdout, REPORT_GET, &json!({}))?;
        if let Some(found) = overlap_cursor_from_report(&response, target) {
            return Ok(found);
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err(anyhow!(
        "no overlapping clusters found in {} in 30s",
        target.display()
    ))
}

/// Returns a cursor at the first byte of the first occurrence matching
/// `target`, or `None` when no cluster yet references the file.
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

/// Returns a cursor + overlap count when at least two occurrences in
/// `target` overlap on some byte across different clusters.
fn overlap_cursor_from_report(response: &Value, target: &Path) -> Option<(Value, usize)> {
    let clusters = response.pointer("/result/clusters")?.as_array()?;
    let target_name = target.file_name()?;
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for cluster in clusters {
        let Some(occurrences) = cluster.get("occurrences").and_then(Value::as_array) else {
            continue;
        };
        for occurrence in occurrences {
            let Some(path) = occurrence.get("path").and_then(Value::as_str) else {
                continue;
            };
            if Path::new(path).file_name() != Some(target_name) {
                continue;
            }
            let Some(start) = occurrence.get("start_byte").and_then(Value::as_u64) else {
                continue;
            };
            let Some(end) = occurrence.get("end_byte").and_then(Value::as_u64) else {
                continue;
            };
            if end <= start {
                continue;
            }
            ranges.push((
                usize::try_from(start).unwrap_or(usize::MAX),
                usize::try_from(end).unwrap_or(usize::MAX),
            ));
        }
    }
    let mut best: Option<(usize, usize)> = None;
    for (index, (start_i, end_i)) in ranges.iter().enumerate() {
        let probe = start_i.saturating_add(end_i.saturating_sub(*start_i) / 2);
        let overlap = ranges
            .iter()
            .enumerate()
            .filter(|(other_index, (start_j, end_j))| {
                *other_index != index && probe >= *start_j && probe < *end_j
            })
            .count()
            .saturating_add(1);
        if overlap > best.map_or(0, |(_, count)| count) {
            best = Some((probe, overlap));
        }
    }
    let (byte, overlap) = best?;
    if overlap < 2 {
        return None;
    }
    let body = std::fs::read_to_string(target).ok()?;
    let (line, character) = byte_position(&body, byte);
    Some((json!({ "line": line, "character": character }), overlap))
}

/// Extracts the markdown payload from a `textDocument/hover` response.
fn hover_markdown(response: &Value) -> Result<String> {
    let value = response
        .pointer("/result/contents/value")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("hover response missing markdown value: {response}"))?;
    Ok(value.to_owned())
}

/// Returns the text of every two-space-indented top-level sub-bullet
/// under the hover's outer bullet. For the human card the list must be
/// empty — occurrence lists and signal scores are agent-only content.
fn top_level_sub_bullets(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| line.strip_prefix("  - "))
        .map(|rest| rest.split(':').next().unwrap_or(rest).trim().to_owned())
        .map(|head| {
            if head == "Occurrences" {
                "Occurrences:".to_owned()
            } else {
                head
            }
        })
        .collect()
}

/// Counts distinct bold bucket headlines in the hover markdown. One
/// card produces exactly one headline; overlapping clusters each add a
/// headline containing the bucket's plain-title label.
fn count_card_headlines(markdown: &str) -> usize {
    markdown
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start_matches(|character: char| {
                character.is_whitespace() || character == '-'
            });
            trimmed.starts_with("**")
                && (trimmed.contains("Identical code")
                    || trimmed.contains("Nearly identical code")
                    || trimmed.contains("Loosely similar code")
                    || trimmed.contains("Same behavior"))
        })
        .count()
}

/// Mirrors `position::position_for_byte` without the tower-lsp
/// dependency.
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
