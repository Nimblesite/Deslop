//! [LIVE-OPEN-BUFFERS] [VSIX-STATE-DIRTY] Every live report answer names the
//! editor version of each open buffer it analysed, read under the lock that
//! produced it. The editor releases an unsaved file's occurrences on exactly
//! that version, so a report reflecting an edit must carry the edit's
//! version itself: a separate notification can reach the editor after the
//! report it describes. Alternates the clone out of and back into the buffer
//! with report requests pipelined behind each edit, and checks every answer.

use std::{
    fs,
    io::BufReader,
    process::{ChildStdin, ChildStdout},
    time::Instant,
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tower_lsp::lsp_types::Url;

use super::{
    byte_field, cluster_spanning, did_open, has_cluster, occurrence_in, string_field, DID_CHANGE,
    EDITED_FILE, FIXTURE, FIXTURE_FILE_COUNT, PEER_FILE,
};
use crate::common::{
    at, call, handshake, notification, read_frame,
    reports::{assert_initialize_contract, report_clusters, REPORT_TIMEOUT},
    request, spawn_lsp_on_fixture_guarded, wait_for_report_matching, workspace_file_uri,
    write_frame,
};

const REPORT_GET: &str = "deslop/reportGet";
const REPORT_DELTA: &str = "deslop/reportDelta";
/// Buffer versions after `didOpen`'s version 1; each flips the clone.
const FIRST_EDIT_VERSION: i32 = 2;
/// Edits per run: every one is a fresh chance for a report to outrun it.
const EDIT_ROUNDS: i32 = 32;
/// `reportGet` requests pipelined behind each edit.
const BURST: usize = 16;

/// One edit and what a report reflecting it must show.
struct Edit<'a> {
    cluster_id: &'a str,
    clone_present: bool,
    version: i32,
}

#[test]
fn every_report_answer_names_the_buffer_version_it_analysed() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(FIXTURE)?;
    assert_initialize_contract(&handshake(&mut stdin, &mut stdout)?);
    let initial = wait_for_report_matching(&mut stdin, &mut stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(FIXTURE_FILE_COUNT)
            && report_clusters(report).is_ok_and(|clusters| !clusters.is_empty())
    })?;
    assert_eq!(
        initial.get("analysed_buffers"),
        Some(&json!([])),
        "with no buffer open the live report names none"
    );
    let cluster = cluster_spanning(&initial, EDITED_FILE, PEER_FILE)?;
    let cluster_id = string_field(&cluster, "id")?;
    let occurrence = occurrence_in(&cluster, EDITED_FILE)?;
    let span = usize::try_from(byte_field(&occurrence, "start_byte")?)?
        ..usize::try_from(byte_field(&occurrence, "end_byte")?)?;
    let disk_text = fs::read_to_string(workspace.path().join(EDITED_FILE))?;
    let mut without_clone = disk_text.clone();
    without_clone.replace_range(span, "");
    let uri = workspace_file_uri(workspace.path(), EDITED_FILE)?;
    write_frame(&mut stdin, &did_open(&uri, &disk_text)?)?;

    for version in FIRST_EDIT_VERSION..FIRST_EDIT_VERSION + EDIT_ROUNDS {
        let clone_present = version % 2 != 0;
        let text = if clone_present {
            &disk_text
        } else {
            &without_clone
        };
        write_frame(&mut stdin, &whole_buffer(&uri, text, version)?)?;
        let edit = Edit {
            cluster_id: &cluster_id,
            clone_present,
            version,
        };
        watch_until_reflected(&mut stdin, &mut stdout, &edit)?;
        assert_delta_names(&mut stdin, &mut stdout, version)?;
    }
    Ok(())
}

/// Pipelines bursts of `reportGet` behind the edit until an answer shows it.
fn watch_until_reflected(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    edit: &Edit<'_>,
) -> Result<()> {
    let started = Instant::now();
    while started.elapsed() < REPORT_TIMEOUT {
        if burst(stdin, stdout, edit)? {
            return Ok(());
        }
    }
    Err(anyhow!(
        "the edit to version {} never reached a report",
        edit.version
    ))
}

/// One pipelined burst; true once an answer reflected the edit. Every
/// answer that reflects it must name its version.
fn burst(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    edit: &Edit<'_>,
) -> Result<bool> {
    for _ in 0..BURST {
        write_frame(stdin, &request(REPORT_GET, &json!({}))?.1)?;
    }
    let mut reflected = false;
    for _ in 0..BURST {
        let report = next_result(stdout)?;
        if has_cluster(&report, edit.cluster_id) == edit.clone_present {
            assert_eq!(
                named_versions(&report),
                vec![edit.version],
                "a report reflecting buffer version {} must name that version",
                edit.version
            );
            reflected = true;
        }
    }
    Ok(reflected)
}

/// The next response's `result`, skipping server-initiated frames.
fn next_result(stdout: &mut BufReader<ChildStdout>) -> Result<Value> {
    loop {
        let frame = read_frame(stdout)?;
        if frame.get("method").is_none() {
            return frame
                .get("result")
                .cloned()
                .ok_or_else(|| anyhow!("{frame}"));
        }
    }
}

/// A delta spanning the whole session names the same version.
fn assert_delta_names(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    version: i32,
) -> Result<()> {
    let response = call(
        stdin,
        stdout,
        REPORT_DELTA,
        &json!({ "since_generation": 0 }),
    )?;
    let delta = response
        .get("result")
        .ok_or_else(|| anyhow!("{response}"))?;
    assert_eq!(
        named_versions(delta),
        vec![version],
        "the delta names version {version}"
    );
    Ok(())
}

/// The versions an answer's `analysed_buffers` names for the edited file.
fn named_versions(answer: &Value) -> Vec<i32> {
    answer
        .get("analysed_buffers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|buffer| {
            at(buffer, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(EDITED_FILE))
        })
        .filter_map(|buffer| buffer.get("version").and_then(Value::as_i64))
        .filter_map(|version| i32::try_from(version).ok())
        .collect()
}

/// A whole-buffer `textDocument/didChange` to `text` at `version`.
fn whole_buffer(uri: &Url, text: &str, version: i32) -> Result<String> {
    notification(
        DID_CHANGE,
        &json!({
            "textDocument": { "uri": uri.as_str(), "version": version },
            "contentChanges": [{ "text": text }]
        }),
    )
}
