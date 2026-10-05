//! [LSP-CODE-LENS] The clone badge sits at the first line of every
//! occurrence in the requested file: one lens per occurrence, anchored
//! where that duplicate starts, so a reader scrolled to the clone sees it
//! ([VSIX-CODE-LENS]). Drives the real `deslop-lsp` binary over stdio.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{
    call, handshake, notification,
    reports::{report_clusters, wait_for_report},
    spawn_lsp_on_fixture_guarded, workspace_file_uri, write_frame,
};

const CODE_LENS: &str = "textDocument/codeLens";
const DIAGNOSTIC: &str = "textDocument/diagnostic";
const CONFIG_CHANGED: &str = "workspace/didChangeConfiguration";
const JUMP_COMMAND: &str = "deslop.jumpToNextOccurrence";
const JUMP_ACTION_SUFFIX: &str = " — jump to next";
/// Three near-identical tier methods in one file, none of them at line 1.
const FIXTURE: &str = "csharp-merge-defaults";
const FIXTURE_FILE: &str = "Tiers.cs";
/// One-based first lines of `ApplyBronze`, `ApplySilver` and `ApplyGold`,
/// as the engine reports them.
const OCCURRENCE_START_LINES: [u64; 3] = [3, 14, 25];
const LENS_TITLE_PREFIX: &str = "Nearly identical code × 3 — mass ";
const FIRST_COLUMN: u64 = 0;

/// Reads `pointer` from `value` as an unsigned integer.
fn number_at(value: &Value, pointer: &str) -> Result<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("{pointer} is not a number: {value}"))
}

/// Reads `pointer` from `value` as a string.
fn text_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{pointer} is not a string: {value}"))
}

#[test]
fn one_lens_per_occurrence_anchored_at_the_occurrence_first_line() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(FIXTURE)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let report = wait_for_report(&mut stdin, &mut stdout, |report| {
        report_clusters(report).is_ok_and(|clusters| !clusters.is_empty())
    })?;

    // Pin the fixture the lenses are projected from: one cluster, three
    // occurrences, each starting below line 1.
    let clusters = report_clusters(&report)?;
    let [cluster] = clusters.as_slice() else {
        return Err(anyhow!("expected exactly one cluster: {report}"));
    };
    let cluster_id = text_at(cluster, "/id")?;
    let occurrences = cluster
        .pointer("/occurrences")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("cluster carries occurrences: {cluster}"))?;
    let start_lines = occurrences
        .iter()
        .map(|occurrence| number_at(occurrence, "/start_line"))
        .collect::<Result<Vec<u64>>>()?;
    assert_eq!(
        start_lines, OCCURRENCE_START_LINES,
        "the three tier methods are the cluster, in file order: {cluster}"
    );

    let response = call(
        &mut stdin,
        &mut stdout,
        CODE_LENS,
        &json!({ "textDocument": { "uri": workspace_file_uri(workspace.path(), FIXTURE_FILE)? } }),
    )?;
    let lenses = response
        .pointer("/result")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("code lens result missing: {response}"))?;
    assert_eq!(
        lenses.len(),
        OCCURRENCE_START_LINES.len(),
        "one lens per occurrence in the file: {response}"
    );

    for (index, (lens, start_line)) in lenses.iter().zip(OCCURRENCE_START_LINES).enumerate() {
        let expected_line = start_line - 1;
        assert_eq!(
            number_at(lens, "/range/start/line")?,
            expected_line,
            "[LSP-CODE-LENS] lens {index} sits at the first line of its occurrence: {lens}"
        );
        assert_eq!(
            number_at(lens, "/range/end/line")?,
            expected_line,
            "the lens anchor is a single line: {lens}"
        );
        assert_eq!(
            number_at(lens, "/range/start/character")?,
            FIRST_COLUMN,
            "the lens anchors at column zero: {lens}"
        );
        assert_eq!(
            text_at(lens, "/command/command")?,
            JUMP_COMMAND,
            "the lens navigates via Deslop's own command: {lens}"
        );
        assert_eq!(
            lens.pointer("/command/arguments"),
            Some(&json!([cluster_id, index])),
            "the lens names its cluster and occurrence index: {lens}"
        );
        let title = text_at(lens, "/command/title")?;
        assert!(
            title.starts_with(LENS_TITLE_PREFIX) && title.ends_with(JUMP_ACTION_SUFFIX),
            "the lens states the clone kind, count and mass, then the jump action: {title}"
        );
    }
    Ok(())
}

/// [LSP-CODE-LENS] [LSP-DIAGNOSTICS] The lens and the diagnostic are two
/// views of one occurrence: same first line, same description. Diagnostics
/// are opt-in, so the pull is empty until the setting turns them on.
#[test]
fn lens_and_diagnostic_sit_on_the_same_line_and_say_the_same_thing() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(FIXTURE)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let _report = wait_for_report(&mut stdin, &mut stdout, |report| {
        report_clusters(report).is_ok_and(|clusters| !clusters.is_empty())
    })?;
    let document =
        json!({ "textDocument": { "uri": workspace_file_uri(workspace.path(), FIXTURE_FILE)? } });

    let before = call(&mut stdin, &mut stdout, DIAGNOSTIC, &document)?;
    assert_eq!(
        before
            .pointer("/result/items")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0),
        "diagnostics are off by default: {before}"
    );

    let settings = json!({ "settings": { "deslop": { "diagnostics": { "enabled": true } } } });
    write_frame(&mut stdin, &notification(CONFIG_CHANGED, &settings)?)?;
    let diagnostics = call(&mut stdin, &mut stdout, DIAGNOSTIC, &document)?;
    let mut items = diagnostics
        .pointer("/result/items")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow!("diagnostic items missing: {diagnostics}"))?;
    items.sort_by_key(|item| number_at(item, "/range/start/line").unwrap_or(u64::MAX));

    let lenses = call(&mut stdin, &mut stdout, CODE_LENS, &document)?;
    let lenses = lenses
        .pointer("/result")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow!("code lens result missing: {lenses}"))?;
    assert_eq!(
        items.len(),
        OCCURRENCE_START_LINES.len(),
        "one diagnostic per occurrence: {items:?}"
    );
    assert_eq!(
        lenses.len(),
        OCCURRENCE_START_LINES.len(),
        "one lens per occurrence: {lenses:?}"
    );

    for ((item, lens), start_line) in items.iter().zip(&lenses).zip(OCCURRENCE_START_LINES) {
        let lens_line = number_at(lens, "/range/start/line")?;
        assert_eq!(
            lens_line,
            start_line - 1,
            "the lens sits on the occurrence: {lens}"
        );
        assert_eq!(
            number_at(item, "/range/start/line")?,
            lens_line,
            "the diagnostic starts where the lens sits: {item}"
        );
        let expected_title = format!("{}{JUMP_ACTION_SUFFIX}", text_at(item, "/message")?);
        assert_eq!(
            text_at(lens, "/command/title")?,
            expected_title,
            "the lens says what the diagnostic says, plus the jump action: {lens}"
        );
        assert_eq!(
            item.pointer("/data/cluster_id"),
            lens.pointer("/command/arguments/0"),
            "both name the same cluster: {item}"
        );
    }
    Ok(())
}
