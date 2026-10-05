//! [LIVE-OPEN-BUFFERS] The report follows the editor's open buffer, not the
//! bytes on disk. Drives the real binary over stdio with the notifications
//! VS Code sends: open a file, delete a clone in the unsaved buffer, and the
//! clone leaves the report and every range query; restore it, and it
//! returns; close the buffer without saving, and the disk is the truth
//! again.

use std::{
    fs,
    io::BufReader,
    path::Path,
    process::{ChildStdin, ChildStdout},
};

use anyhow::{anyhow, Result};
use deslop_core::refactor::position::line_col_for_byte;
use serde_json::{json, Value};
use tower_lsp::lsp_types::Url;

use crate::common::{
    at, call, handshake, notification,
    reports::{
        assert_initialize_contract, assert_report_shell, cluster_paths, has_suffix,
        occurrence_paths, report_clusters, REPORT_TIMEOUT,
    },
    spawn_lsp_on_fixture_guarded, wait_for_report_matching, workspace_file_uri, write_frame,
};

#[path = "open_buffers/embedding.rs"]
mod embedding;

/// The two-file C# workspace whose one clone spans both files.
const FIXTURE: &str = "csharp-small";
const FIXTURE_FILE_COUNT: u64 = 2;
const EDITED_FILE: &str = "Alpha.cs";
const PEER_FILE: &str = "Beta.cs";
const LANGUAGE_ID: &str = "csharp";
const DID_OPEN: &str = "textDocument/didOpen";
const DID_CHANGE: &str = "textDocument/didChange";
const DID_CLOSE: &str = "textDocument/didClose";
const REPORT_FOR_RANGE: &str = "deslop/reportForRange";
/// Replacement text that removes the edited span outright.
const DELETED: &str = "";
/// One byte inside the deleted clone, used to query the range it covered.
const INSIDE_OCCURRENCE_BYTE: u64 = 1;

#[test]
fn unsaved_buffer_edits_drive_the_report_and_range_queries() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(FIXTURE)?;
    assert_initialize_contract(&handshake(&mut stdin, &mut stdout)?);
    let initial = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(FIXTURE_FILE_COUNT)
            && report_clusters(report).is_ok_and(|clusters| !clusters.is_empty())
    })?;
    assert_report_shell(&initial, FIXTURE_FILE_COUNT);
    let cluster = cluster_spanning(&initial, EDITED_FILE, PEER_FILE)?;
    let cluster_id = string_field(&cluster, "id")?;
    let occurrence = occurrence_in(&cluster, EDITED_FILE)?;
    let start_byte = byte_field(&occurrence, "start_byte")?;
    let end_byte = byte_field(&occurrence, "end_byte")?;
    let file = workspace.path().join(EDITED_FILE);
    let disk_text = fs::read_to_string(&file)?;
    let uri = workspace_file_uri(workspace.path(), EDITED_FILE)?;

    write_frame(&mut stdin, &did_open(&uri, &disk_text)?)?;
    let deletion = ranged_change(&uri, &disk_text, start_byte, end_byte, DELETED)?;
    write_frame(&mut stdin, &deletion)?;
    let without = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        !has_cluster(report, &cluster_id)
    })?;
    assert!(
        !has_suffix(&occurrence_paths(&without)?, EDITED_FILE),
        "no clone may be reported in the file whose copy was deleted: {without}"
    );
    assert_eq!(
        at(&without, "files_analysed"),
        FIXTURE_FILE_COUNT,
        "the edited file is still analysed, it just holds no clone"
    );
    assert_eq!(
        fs::read_to_string(&file)?,
        disk_text,
        "the disk is untouched; the deletion lives only in the buffer"
    );
    let hits = report_for_range(&mut stdin, &mut stdout, &file, start_byte)?;
    assert!(
        hits.is_empty(),
        "a range inside the deleted clone must report nothing: {hits:?}"
    );

    write_frame(&mut stdin, &whole_change(&uri, &disk_text)?)?;
    let restored = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        has_cluster(report, &cluster_id)
    })?;
    assert_eq!(
        at(&restored, "clusters"),
        at(&initial, "clusters"),
        "restoring the buffer text reproduces the initial clusters exactly"
    );
    let hits = report_for_range(&mut stdin, &mut stdout, &file, start_byte)?;
    assert!(
        hits.iter().any(|hit| at(hit, "id") == cluster_id.as_str()),
        "the restored clone answers the range query again: {hits:?}"
    );

    write_frame(&mut stdin, &deletion)?;
    let _without_again =
        wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
            !has_cluster(report, &cluster_id)
        })?;
    write_frame(&mut stdin, &did_close(&uri)?)?;
    let after_close =
        wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
            has_cluster(report, &cluster_id)
        })?;
    assert_eq!(
        at(&after_close, "clusters"),
        at(&initial, "clusters"),
        "closing an unsaved buffer makes the disk the truth again"
    );
    Ok(())
}

/// The `textDocument/didOpen` VS Code sends for `uri` holding `text`.
fn did_open(uri: &Url, text: &str) -> Result<String> {
    notification(
        DID_OPEN,
        &json!({
            "textDocument": {
                "uri": uri.as_str(),
                "languageId": LANGUAGE_ID,
                "version": 1,
                "text": text
            }
        }),
    )
}

/// An incremental `textDocument/didChange` replacing the byte span
/// `start_byte..end_byte` of `text` with `replacement`.
fn ranged_change(
    uri: &Url,
    text: &str,
    start_byte: u64,
    end_byte: u64,
    replacement: &str,
) -> Result<String> {
    let start = line_col_for_byte(text, usize::try_from(start_byte)?);
    let end = line_col_for_byte(text, usize::try_from(end_byte)?);
    notification(
        DID_CHANGE,
        &json!({
            "textDocument": { "uri": uri.as_str(), "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": start.line, "character": start.character },
                    "end": { "line": end.line, "character": end.character }
                },
                "text": replacement
            }]
        }),
    )
}

/// A `textDocument/didChange` without a range: the whole buffer becomes
/// `text`.
fn whole_change(uri: &Url, text: &str) -> Result<String> {
    notification(
        DID_CHANGE,
        &json!({
            "textDocument": { "uri": uri.as_str(), "version": 3 },
            "contentChanges": [{ "text": text }]
        }),
    )
}

/// The `textDocument/didClose` for `uri`.
fn did_close(uri: &Url) -> Result<String> {
    notification(
        DID_CLOSE,
        &json!({ "textDocument": { "uri": uri.as_str() } }),
    )
}

/// Queries `deslop/reportForRange` one byte inside the clone that began
/// at `start_byte` in `file`.
fn report_for_range(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    file: &Path,
    start_byte: u64,
) -> Result<Vec<Value>> {
    let probe = start_byte.saturating_add(INSIDE_OCCURRENCE_BYTE);
    let response = call(
        stdin,
        stdout,
        REPORT_FOR_RANGE,
        &json!({ "path": file, "start_byte": probe, "end_byte": probe }),
    )?;
    response
        .get("result")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow!("reportForRange returned no cluster array: {response}"))
}

/// True when `report` lists a cluster with `cluster_id`.
fn has_cluster(report: &Value, cluster_id: &str) -> bool {
    report_clusters(report).is_ok_and(|clusters| {
        clusters
            .iter()
            .any(|cluster| at(cluster, "id") == cluster_id)
    })
}

/// The cluster whose occurrences cover both `first` and `second`.
fn cluster_spanning(report: &Value, first: &str, second: &str) -> Result<Value> {
    report_clusters(report)?
        .iter()
        .find(|cluster| {
            let paths = cluster_paths(cluster);
            has_suffix(&paths, first) && has_suffix(&paths, second)
        })
        .cloned()
        .ok_or_else(|| anyhow!("no cluster spans {first} and {second}: {report}"))
}

/// The occurrence of `cluster` that lives in `file`.
fn occurrence_in(cluster: &Value, file: &str) -> Result<Value> {
    at(cluster, "occurrences")
        .as_array()
        .into_iter()
        .flatten()
        .find(|occurrence| {
            at(occurrence, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(file))
        })
        .cloned()
        .ok_or_else(|| anyhow!("cluster has no occurrence in {file}: {cluster}"))
}

/// A required string field of `value`.
fn string_field(value: &Value, field: &str) -> Result<String> {
    at(value, field)
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{field} is not a string: {value}"))
}

/// A required byte-offset field of `value`.
fn byte_field(value: &Value, field: &str) -> Result<u64> {
    at(value, field)
        .as_u64()
        .ok_or_else(|| anyhow!("{field} is not a byte offset: {value}"))
}
