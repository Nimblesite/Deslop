//! [LANG-CAND-OSPREY-UNPARSED] A file that cannot be analysed is skipped,
//! never fatal. Drives the real `deslop-lsp` binary over stdio: the server
//! boots with a malformed `.ospml` on disk, keeps serving, returns the file
//! to the report the moment it parses again, and follows the editor's open
//! buffer instead of the broken disk bytes.

use std::{
    collections::BTreeSet,
    fs,
    io::BufReader,
    path::Path,
    process::{ChildStdin, ChildStdout},
    sync::mpsc::{self, Receiver},
    time::Instant,
};

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Map, Value};

use crate::{
    common::{
        at, copy_fixture, notification, read_frame,
        reports::{
            assert_initialize_contract, assert_report_shell, assert_report_shell_without_clusters,
            cluster_paths, report_clusters, REPORT_TIMEOUT,
        },
        spawn_lsp_guarded, whole_buffer_change, workspace_file_uri, write_frame, LspGuard,
    },
    notifications::{recv_method, request_response, FrameResult},
};

/// Two good copies plus a third identical copy, so one broken file still
/// leaves a duplicate pair.
const FIXTURE: &str = "osprey-ml";
const BROKEN_FILE: &str = "alpha.ospml";
const PEER_FILE: &str = "beta.ospml";
const THIRD_COPY: &str = "gamma.ospml";
const ML_LANGUAGE: &str = "osprey-ml";
const MALFORMED_ML: &str = "describe value =\n    if value > then\n        value +\n";
const ML_PARSE_ERROR: &str = "tree-sitter failed to produce a parse tree for osprey-ml";
const DEEP_FILE: &str = "deep.py";
const DEEP_NESTING: usize = 700;
const DEEP_REASON: &str = "python source nests deeper than the 500-level AST depth limit";
const TWO_FILES: u64 = 2;
const THREE_FILES: u64 = 3;
const ZERO_FILES: u64 = 0;
const ONE_CLUSTER: usize = 1;
const ZERO_CLUSTERS: usize = 0;
const FIRST_VERSION: i64 = 1;
const SECOND_VERSION: i64 = 2;
const THIRD_VERSION: i64 = 3;
const INITIALIZE: &str = "initialize";
const REPORT_GET: &str = "deslop/reportGet";
const REPORT_DELTA: &str = "deslop/reportDelta";
const REPORT_CHANGED: &str = "deslop/reportChanged";
const DID_OPEN: &str = "textDocument/didOpen";
const DID_CLOSE: &str = "textDocument/didClose";
const RESULT: &str = "result";
const FILES_ANALYSED: &str = "files_analysed";
const FILES_SKIPPED: &str = "files_skipped";
const CLUSTERS: &str = "clusters";
const PATH: &str = "path";
const REASON: &str = "reason";
const SINCE_GENERATION: &str = "since_generation";
const ZERO_GENERATION: u64 = 0;
const DELTA_GENERATION: &str = "/result/to_generation";
const NOTIFIED_GENERATION: &str = "/params/generation";

/// Decoded protocol frames shared by the live helpers.
type Frames = Receiver<FrameResult>;

/// Reads frames on a background thread so waits stay bounded.
fn spawn_frame_reader(mut stdout: BufReader<ChildStdout>) -> Frames {
    let (tx, rx) = mpsc::channel();
    let _reader = std::thread::spawn(move || loop {
        let frame = read_frame(&mut stdout).map_err(|error| error.to_string());
        if tx.send(frame).is_err() {
            break;
        }
    });
    rx
}

/// The `initialize` params the real client sends.
fn initialize_params() -> Value {
    json!({
        "processId": null,
        "rootUri": null,
        "capabilities": {}
    })
}

/// Spawns the real server, completes the handshake, asserts the initialize
/// contract, and returns the first `deslop/reportGet` response frame.
fn spawn_live(root: &Path) -> Result<(LspGuard, ChildStdin, Frames, Value)> {
    let (guard, mut stdin, stdout) = spawn_lsp_guarded(root)?;
    let frames = spawn_frame_reader(stdout);
    let init = request_response(&mut stdin, &frames, INITIALIZE, &initialize_params())?;
    assert_initialize_contract(&init);
    let initial = request_response(&mut stdin, &frames, REPORT_GET, &json!({}))?;
    Ok((guard, stdin, frames, initial))
}

/// Requests the current live report and returns its `result` object.
fn current_report(stdin: &mut ChildStdin, frames: &Frames) -> Result<Value> {
    let frame = request_response(stdin, frames, REPORT_GET, &json!({}))?;
    ensure!(
        frame.get("error").is_none(),
        "reportGet returned an error: {frame}"
    );
    Ok(at(&frame, RESULT).clone())
}

/// Reads the current published generation without requesting a scan.
fn generation(stdin: &mut ChildStdin, frames: &Frames) -> Result<u64> {
    let params = Value::Object(Map::from_iter([(
        SINCE_GENERATION.to_owned(),
        ZERO_GENERATION.into(),
    )]));
    number(
        &request_response(stdin, frames, REPORT_DELTA, &params)?,
        DELTA_GENERATION,
    )
}

/// Reads a required number from a JSON pointer.
fn number(value: &Value, pointer: &str) -> Result<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("missing number {pointer}: {value}"))
}

/// Awaits a strictly newer `deslop/reportChanged` notification.
fn changed_after(frames: &Frames, previous: u64) -> Result<u64> {
    let deadline = Instant::now()
        .checked_add(REPORT_TIMEOUT)
        .ok_or_else(|| anyhow!("deadline overflow"))?;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let frame = recv_method(frames, REPORT_CHANGED, remaining)?;
        let next = number(&frame, NOTIFIED_GENERATION)?;
        if next > previous {
            return Ok(next);
        }
    }
}

/// The report's `files_skipped` entries as `(path, reason)` in report order.
fn skipped_entries(report: &Value) -> Result<Vec<(String, String)>> {
    at(report, FILES_SKIPPED)
        .as_array()
        .ok_or_else(|| anyhow!("report carries no files_skipped array: {report}"))?
        .iter()
        .map(|entry| {
            let path = at(entry, PATH)
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("skipped entry has no path: {entry}"))?;
            let reason = at(entry, REASON)
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("skipped entry has no reason: {entry}"))?;
            Ok((path, reason))
        })
        .collect()
}

/// Asserts `files_skipped` equals `expected` exactly, in order.
fn assert_skipped(report: &Value, expected: &[(&str, &str)]) -> Result<()> {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(path, reason)| ((*path).to_owned(), (*reason).to_owned()))
        .collect();
    assert_eq!(skipped_entries(report)?, expected, "{report:#}");
    Ok(())
}

/// Asserts the exact analysed count, skip list, cluster count and — when a
/// cluster is expected — its occurrence paths.
fn assert_state(
    report: &Value,
    analysed: u64,
    skipped: &[(&str, &str)],
    occurrences: &BTreeSet<String>,
) -> Result<()> {
    assert_eq!(at(report, FILES_ANALYSED), analysed, "{report:#}");
    assert_skipped(report, skipped)?;
    let clusters = report_clusters(report)?;
    if occurrences.is_empty() {
        assert_eq!(clusters.len(), ZERO_CLUSTERS, "{report:#}");
        return Ok(());
    }
    assert_eq!(clusters.len(), ONE_CLUSTER, "{report:#}");
    let cluster = clusters
        .first()
        .ok_or_else(|| anyhow!("expected one cluster: {report}"))?;
    assert_eq!(cluster_paths(cluster), *occurrences, "{report:#}");
    Ok(())
}

/// A set of workspace-relative occurrence paths.
fn paths(files: &[&str]) -> BTreeSet<String> {
    files.iter().map(|file| (*file).to_owned()).collect()
}

/// Writes `text` to `file` on disk and settles on the next report.
fn save_and_settle(
    stdin: &mut ChildStdin,
    frames: &Frames,
    root: &Path,
    previous: u64,
    file: &str,
    text: &str,
) -> Result<(Value, u64)> {
    fs::write(root.join(file), text)?;
    let next = changed_after(frames, previous)?;
    let report = current_report(stdin, frames)?;
    Ok((report, next))
}

/// Deletes `file` from disk and settles on the next report.
fn remove_and_settle(
    stdin: &mut ChildStdin,
    frames: &Frames,
    root: &Path,
    previous: u64,
    file: &str,
) -> Result<(Value, u64)> {
    fs::remove_file(root.join(file))?;
    let next = changed_after(frames, previous)?;
    let report = current_report(stdin, frames)?;
    Ok((report, next))
}

/// Sends one editor notification and settles on the next report.
fn buffer_and_settle(
    stdin: &mut ChildStdin,
    frames: &Frames,
    previous: u64,
    frame: &str,
) -> Result<(Value, u64)> {
    write_frame(stdin, frame)?;
    let next = changed_after(frames, previous)?;
    let report = current_report(stdin, frames)?;
    Ok((report, next))
}

/// The `textDocument/didOpen` VS Code sends for `uri` holding `text`.
fn did_open(uri: &str, text: &str) -> Result<String> {
    notification(
        DID_OPEN,
        &json!({
            "textDocument": {
                "uri": uri,
                "languageId": ML_LANGUAGE,
                "version": FIRST_VERSION,
                "text": text
            }
        }),
    )
}

/// The `textDocument/didClose` for `uri`.
fn did_close(uri: &str) -> Result<String> {
    notification(DID_CLOSE, &json!({ "textDocument": { "uri": uri } }))
}

/// Copies the fixture, adds a third identical copy, and breaks `alpha.ospml`.
fn broken_workspace() -> Result<(tempfile::TempDir, String)> {
    let workspace = copy_fixture(FIXTURE)?;
    let valid = fs::read_to_string(workspace.path().join(PEER_FILE))?;
    fs::write(workspace.path().join(THIRD_COPY), &valid)?;
    fs::write(workspace.path().join(BROKEN_FILE), MALFORMED_ML)?;
    Ok((workspace, valid))
}

/// [LANG-CAND-OSPREY-UNPARSED] One broken `.ospml` never kills the live
/// server, and every later fix, re-break, editor edit and deletion is
/// reflected exactly in `deslop/reportGet`.
#[test]
fn broken_osprey_ml_file_is_skipped_without_killing_the_live_server() -> Result<()> {
    let (workspace, valid) = broken_workspace()?;
    let root = workspace.path();
    let uri = workspace_file_uri(root, BROKEN_FILE)?.to_string();
    let two_good = paths(&[PEER_FILE, THIRD_COPY]);
    let all_three = paths(&[BROKEN_FILE, PEER_FILE, THIRD_COPY]);
    let alpha_skipped = [(BROKEN_FILE, ML_PARSE_ERROR)];

    let (_guard, mut stdin, frames, initial_frame) = spawn_live(root)?;
    let initial = at(&initial_frame, RESULT).clone();
    assert_report_shell(&initial, TWO_FILES);
    assert_state(&initial, TWO_FILES, &alpha_skipped, &two_good)?;
    let mut previous = generation(&mut stdin, &frames)?;

    let (fixed, next) = save_and_settle(&mut stdin, &frames, root, previous, BROKEN_FILE, &valid)?;
    assert_state(&fixed, THREE_FILES, &[], &all_three)?;
    previous = next;

    let (broken, next) = save_and_settle(
        &mut stdin,
        &frames,
        root,
        previous,
        BROKEN_FILE,
        MALFORMED_ML,
    )?;
    assert_state(&broken, TWO_FILES, &alpha_skipped, &two_good)?;
    previous = next;

    // Re-saving the same broken text is a no-op pass: the server must stay
    // alive and the report must not move.
    fs::write(root.join(BROKEN_FILE), MALFORMED_ML)?;
    let same = current_report(&mut stdin, &frames)?;
    assert_state(&same, TWO_FILES, &alpha_skipped, &two_good)?;
    assert_eq!(
        at(&same, CLUSTERS),
        at(&broken, CLUSTERS),
        "re-saving identical broken bytes must leave the report unchanged"
    );

    let (opened, next) =
        buffer_and_settle(&mut stdin, &frames, previous, &did_open(&uri, &valid)?)?;
    assert_state(&opened, THREE_FILES, &[], &all_three)?;
    previous = next;

    let (edited_broken, next) = buffer_and_settle(
        &mut stdin,
        &frames,
        previous,
        &whole_buffer_change(&uri, SECOND_VERSION, MALFORMED_ML)?,
    )?;
    assert_state(&edited_broken, TWO_FILES, &alpha_skipped, &two_good)?;
    previous = next;

    let (edited_valid, next) = buffer_and_settle(
        &mut stdin,
        &frames,
        previous,
        &whole_buffer_change(&uri, THIRD_VERSION, &valid)?,
    )?;
    assert_state(&edited_valid, THREE_FILES, &[], &all_three)?;
    previous = next;

    let (closed, next) = buffer_and_settle(&mut stdin, &frames, previous, &did_close(&uri)?)?;
    assert_state(&closed, TWO_FILES, &alpha_skipped, &two_good)?;
    previous = next;

    let (deleted, _next) = remove_and_settle(&mut stdin, &frames, root, previous, BROKEN_FILE)?;
    assert_state(&deleted, TWO_FILES, &[], &two_good)?;
    Ok(())
}

/// [LANG-CAND-OSPREY-UNPARSED] A repository of nothing but broken ML still
/// boots and reports zero analysed files, zero clusters, and both skips.
#[test]
fn a_repository_of_only_malformed_osprey_ml_still_boots() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    for file in [BROKEN_FILE, PEER_FILE] {
        fs::write(workspace.path().join(file), MALFORMED_ML)?;
    }
    let both_skipped = [(BROKEN_FILE, ML_PARSE_ERROR), (PEER_FILE, ML_PARSE_ERROR)];
    let no_clusters = BTreeSet::new();

    let (_guard, mut stdin, frames, initial_frame) = spawn_live(workspace.path())?;
    let initial = at(&initial_frame, RESULT).clone();
    assert_report_shell_without_clusters(&initial, ZERO_FILES);
    assert_state(&initial, ZERO_FILES, &both_skipped, &no_clusters)?;
    let again = current_report(&mut stdin, &frames)?;
    assert_state(&again, ZERO_FILES, &both_skipped, &no_clusters)?;
    Ok(())
}

/// [LANG-CAND-OSPREY-UNPARSED] A file nested past the AST depth limit is
/// skipped, named with the depth reason, and the remaining pair still clusters.
#[test]
fn a_too_deep_file_is_skipped_and_the_server_boots() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    let deep = format!(
        "x = {}1{}\n",
        "[".repeat(DEEP_NESTING),
        "]".repeat(DEEP_NESTING)
    );
    fs::write(workspace.path().join(DEEP_FILE), deep)?;
    let two_good = paths(&[BROKEN_FILE, PEER_FILE]);
    let deep_skipped = [(DEEP_FILE, DEEP_REASON)];

    let (_guard, mut stdin, frames, initial_frame) = spawn_live(workspace.path())?;
    let initial = at(&initial_frame, RESULT).clone();
    assert_report_shell(&initial, TWO_FILES);
    assert_state(&initial, TWO_FILES, &deep_skipped, &two_good)?;
    let again = current_report(&mut stdin, &frames)?;
    assert_state(&again, TWO_FILES, &deep_skipped, &two_good)?;
    Ok(())
}
