//! [LIVE-OPEN-BUFFERS] An editor that stops reading the server's output
//! must not freeze every other reader. The session lock is released before
//! a `deslop/reportChanged` is written, so the IPC socket keeps answering
//! while the editor's pipe is full.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::Path,
    process::ChildStdin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};

use super::{
    common::{notification, whole_buffer_change, workspace_file_uri, write_frame},
    harness::fixture_socket_ready,
    ANALYSIS_TIMEOUT,
};

const FIXTURE: &str = "csharp-small";
const EDITED_FILE: &str = "Alpha.cs";
const REPORT_GET: &str = "report/get";
/// Edits written without reading one byte back: their notifications
/// overflow any pipe buffer many times over.
const FLOOD: usize = 20_000;
/// Consecutive IPC answers with no edit consumed in between: the server
/// has stopped reading the editor, so its output pipe is full.
const STALLED_ANSWERS: usize = 200;
/// The stall must be observed well before the IPC answers run out.
const MAX_ANSWERS: usize = 100_000;
const FIRST_EDIT_VERSION: usize = 2;

/// One `report/get` over the IPC socket, failing if no answer arrives.
fn ipc_report(socket: &Path, id: usize) -> Result<Value> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(ANALYSIS_TIMEOUT))?;
    let mut payload = serde_json::to_vec(
        &json!({"jsonrpc": "2.0", "id": id, "method": REPORT_GET, "params": {}}),
    )?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    let mut line = String::new();
    let _read = BufReader::new(&stream).read_line(&mut line).map_err(|error| {
        anyhow!("the IPC report read number {id} got no answer while the editor was not reading: {error}")
    })?;
    let answer: Value = serde_json::from_str(line.trim())?;
    ensure!(
        answer
            .pointer("/result/clusters")
            .is_some_and(Value::is_array),
        "IPC answer {id} is not a report: {line}"
    );
    Ok(answer)
}

/// Writes `FLOOD` whole-buffer edits, counting each one the server accepted.
fn flood_edits(mut stdin: ChildStdin, uri: &str, text: &str, written: &AtomicUsize) {
    for version in FIRST_EDIT_VERSION..FIRST_EDIT_VERSION.saturating_add(FLOOD) {
        let Ok(version_number) = i64::try_from(version) else {
            return;
        };
        let Ok(frame) = whole_buffer_change(uri, version_number, text) else {
            return;
        };
        if write_frame(&mut stdin, &frame).is_err() {
            return;
        }
        let _previous = written.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn an_editor_that_stops_reading_does_not_block_ipc_report_reads() -> Result<()> {
    let (workspace, guard, mut stdin, stdout, socket) = fixture_socket_ready(FIXTURE)?;
    let uri = workspace_file_uri(workspace.path(), EDITED_FILE)?;
    let text = fs::read_to_string(workspace.path().join(EDITED_FILE))?;
    let opened = notification(
        "textDocument/didOpen",
        &json!({"textDocument": {"uri": uri.as_str(), "languageId": "csharp", "version": 1, "text": text}}),
    )?;
    write_frame(&mut stdin, &opened)?;
    let written = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&written);
    let writer = thread::spawn(move || flood_edits(stdin, uri.as_str(), &text, &counter));

    let mut unchanged = 0_usize;
    let mut last_written = 0_usize;
    let mut answers = 0_usize;
    while unchanged < STALLED_ANSWERS {
        answers = answers.saturating_add(1);
        ensure!(answers < MAX_ANSWERS, "the editor pipe never filled");
        let _report = ipc_report(&socket, answers)?;
        let now_written = written.load(Ordering::SeqCst);
        ensure!(
            now_written < FLOOD,
            "every edit was consumed: the editor pipe never filled"
        );
        unchanged = if now_written > 0 && now_written == last_written {
            unchanged.saturating_add(1)
        } else {
            0
        };
        last_written = now_written;
    }
    let after_stall = ipc_report(&socket, answers.saturating_add(1))?;
    ensure!(
        after_stall
            .pointer("/result/files_analysed")
            .and_then(Value::as_u64)
            == Some(2),
        "the report served while the editor is stalled still covers both files: {after_stall}"
    );
    drop(guard);
    drop(stdout);
    writer
        .join()
        .map_err(|_panic| anyhow!("the edit writer panicked"))?;
    Ok(())
}
