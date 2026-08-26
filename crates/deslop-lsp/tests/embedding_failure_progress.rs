//! Real-LSP regression for failed background embedding refreshes
//! ([LIVE-EMBEDDING-CONSENT], terminal-phase rule).
//!
//! Explicit model selection is a user-requested operation. A refresh that
//! produces no embeddings must emit terminal `failed` progress and leave
//! the last good report untouched — never commit an embeddings-off
//! snapshot and call it `complete`.
//!
//! A pass produces none in two ways, and each test below drives one:
//!
//! 1. **The provider rejects everything.** This assertion exposed a live
//!    false negative the moment GH #370's stderr deadlock stopped hiding
//!    it: the refresh announced `phase = "complete", done = 851` after
//!    the provider rejected all 851 subtrees.
//! 2. **The pass never runs.** A refresh runs under `EmbeddingMode::Auto`,
//!    which deliberately swallows a provider error and returns a report
//!    with no provenance at all — right for an automatic pass, wrong for
//!    a model the user chose.
//!
//! `run_embedding_refresh` (`deslop-core/src/live/embedding_refresh.rs`)
//! converts both into a typed `FailedEmbeddingRefresh` before any commit,
//! so the failure path emits the terminal `failed` event and the last
//! good report survives. Do not weaken these tests to make the tree
//! green.
//!
//! Both cases drive the same wire sequence and owe the same contract, so
//! [`drive_failed_refresh`] holds it once; a test supplies the mock
//! provider and the behaviour its diagnostics name.

use crate::common;

use std::{
    io::BufReader,
    process::{ChildStdin, ChildStdout},
    time::{Duration, Instant},
};

use crate::mock_ollama::{MockBehavior, MockOllama};
use anyhow::{anyhow, Result};
use common::{
    at, call, call_capturing, handshake, path as json_path, reports::assert_initialize_contract,
    spawn_lsp_guarded, wait_for_report_matching, POLL_INTERVAL,
};
use serde_json::{json, Value};

const REPORT_TIMEOUT: Duration = Duration::from_secs(20);
/// How long the refresh may go without emitting a single progress frame
/// before the pass is called stalled.
///
/// The budget is on *silence*, not on the whole pass. A wall-clock ceiling
/// measures the runner, not the server: the same refresh that finishes in
/// seconds on a laptop was still at 192 of 1300 subtrees after 20s on an
/// instrumented two-core CI runner, and the test failed for being slow rather
/// than for being wrong. Silence is the real defect this guards — gh #370 was
/// a refresh that hung for fourteen minutes — and a stall budget catches that
/// the moment it starts instead of after a fixed wait.
const PROGRESS_STALL_TIMEOUT: Duration = Duration::from_secs(20);
/// Longest the whole refresh may run, however busy it looks.
///
/// A stall budget alone is not a bound: a pass that emits one frame every
/// nineteen seconds resets the clock forever and never reaches a terminal
/// phase, so the test hangs until the CI job is killed and reports nothing
/// about the server. This backstop is deliberately far above any real pass —
/// both cases here finish in about twelve seconds in release — so it never
/// competes with [`PROGRESS_STALL_TIMEOUT`] for the diagnosis. It exists only
/// so a trickle is named rather than waited on.
const PROGRESS_TOTAL_CEILING: Duration = Duration::from_secs(600);
const SET_MODEL: &str = "deslop/embeddingSetModel";
const PROGRESS: &str = "deslop/embeddingProgress";
const PROVIDER_ID: &str = "ollama";
const MODEL_ID: &str = "nomic-embed-text";

#[test]
fn rejected_embedding_refresh_reports_failure_and_preserves_last_good_report() -> Result<()> {
    let server = MockOllama::spawn_with(MockBehavior::RejectAllEmbeds)?;
    let (selection, terminal) =
        drive_failed_refresh(&server, "a provider that rejects every embedding")?;

    assert!(
        selection.get("result").is_some(),
        "model selection has no result: {selection:#}"
    );
    assert_eq!(
        json_path(&terminal, &["params", "provider_id"]),
        PROVIDER_ID,
        "{terminal:#}"
    );
    assert_eq!(
        json_path(&terminal, &["params", "model_id"]),
        MODEL_ID,
        "{terminal:#}"
    );
    Ok(())
}

// [LIVE-EMBEDDING-CONSENT] The terminal phase follows *the embeddings the
// pass produced*, and a provider that goes away produces none.
//
// The rejecting-provider case above is only half the contract. Selecting a
// model builds the provider, which probes it — so an endpoint that is
// already down is refused at selection, with an error the user sees. The
// uncovered half is the provider that answers that probe and is gone when
// the background refresh runs: the machine slept, the container was
// recycled, the model was unloaded.
//
// A refresh runs under `EmbeddingMode::Auto`, and `run_embedding_pass`
// deliberately swallows a provider error in that mode — "continuing
// without Type-4 recall" — returning a report with **no**
// `embedding_provenance` at all. That is the right call for an automatic
// pass and the wrong one for a model the user explicitly selected: the
// refresh commits an embeddings-off snapshot over the last good report and
// announces `complete`, so every clone that needed the semantic axis
// disappears from a report claiming that axis ran. The same false negative
// as GH #370, reached through the unreachable door rather than the
// rejecting one, and invisible to the test above because that provider
// answers every request it is given.
#[test]
fn vanished_provider_refresh_reports_failure_and_preserves_last_good_report() -> Result<()> {
    let server = MockOllama::spawn_vanishing_after_handshake()?;
    let (_selection, _terminal) =
        drive_failed_refresh(&server, "a provider that vanished after the probe")?;
    Ok(())
}

/// Selects the model on `server`, then asserts the failure contract every
/// provider owes: the model queues, terminal progress is `failed` with no
/// work counted done and an explanation, and the last good report survives.
/// Returns the selection response and that terminal progress frame.
fn drive_failed_refresh(server: &MockOllama, behaviour: &str) -> Result<(Value, Value)> {
    let workspace = common::copy_fixture("ts-mixed-band")?;
    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(workspace.path())?;
    let before = await_last_good_report(&mut stdin, &mut stdout)?;
    let (selection, initial_frames) = select_model(&mut stdin, &mut stdout, server)?;
    assert!(
        selection.get("error").is_none(),
        "{behaviour} must still queue the model — the failure comes after \
         selection: {selection:#}"
    );

    let terminal = wait_for_terminal_progress(&mut stdin, &mut stdout, initial_frames)?;
    assert_failed_terminal(&terminal, behaviour);
    assert_last_good_report_survived(&mut stdin, &mut stdout, &before)?;
    Ok((selection, terminal))
}

/// Completes the handshake and waits for the cold pass to land the
/// embeddings-off report every failure case must leave untouched.
fn await_last_good_report(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
) -> Result<Value> {
    assert_initialize_contract(&handshake(stdin, stdout)?);
    let before = wait_for_report_matching(stdin, stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(5)
    })?;
    assert!(at(&before, "embedding_provenance").is_null(), "{before:#}");
    Ok(before)
}

/// Explicitly selects the mock model, returning the response together with
/// every frame the server emitted while answering it.
fn select_model(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    server: &MockOllama,
) -> Result<(Value, Vec<Value>)> {
    let model = json!({
        "provider_id": PROVIDER_ID,
        "model_id": MODEL_ID,
        "endpoint": server.endpoint(),
    });
    call_capturing(stdin, stdout, SET_MODEL, &model)
}

/// A refresh that produced no embeddings terminates `failed`, with no work
/// counted done and a message the user can act on.
fn assert_failed_terminal(terminal: &Value, behaviour: &str) {
    assert_eq!(
        json_path(terminal, &["params", "phase"]),
        "failed",
        "{behaviour} produced no embeddings, so an explicitly selected model \
         must terminate `failed`, never `complete`: {terminal:#}"
    );
    assert_eq!(
        json_path(terminal, &["params", "done"]),
        0,
        "failed work cannot be complete"
    );
    let message = json_path(terminal, &["params", "message"]).as_str();
    assert!(
        message.is_some_and(|text| !text.is_empty()),
        "failed progress must explain why {behaviour} produced no \
         embeddings: {terminal:#}"
    );
}

/// Re-reads the committed report and asserts the failed refresh left it
/// exactly as `before`.
fn assert_last_good_report_survived(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    before: &Value,
) -> Result<()> {
    let after = call(stdin, stdout, "deslop/reportGet", &json!({}))?;
    let after_report = after
        .get("result")
        .ok_or_else(|| anyhow!("reportGet returned no result: {after:#}"))?;
    assert_eq!(
        after_report, before,
        "a failed refresh replaced the last good report"
    );
    Ok(())
}

/// The `complete` or `failed` frame, once the refresh has emitted one.
fn terminal_progress(frames: &[Value]) -> Option<Value> {
    frames
        .iter()
        .find(|frame| {
            at(frame, "method") == PROGRESS
                && matches!(
                    json_path(frame, &["params", "phase"]).as_str(),
                    Some("complete" | "failed")
                )
        })
        .cloned()
}

/// Deadline `window` from now, saturating at now.
fn deadline_after(window: Duration) -> Instant {
    Instant::now()
        .checked_add(window)
        .unwrap_or_else(Instant::now)
}

/// Deadline [`PROGRESS_STALL_TIMEOUT`] from now.
fn stall_deadline() -> Instant {
    deadline_after(PROGRESS_STALL_TIMEOUT)
}

fn wait_for_terminal_progress(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    mut frames: Vec<Value>,
) -> Result<Value> {
    let mut deadline = stall_deadline();
    let giving_up = deadline_after(PROGRESS_TOTAL_CEILING);
    loop {
        if let Some(terminal) = terminal_progress(&frames) {
            return Ok(terminal);
        }
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "the embedding refresh emitted nothing for {PROGRESS_STALL_TIMEOUT:?} and \
                 never reached a terminal phase: {frames:#?}"
            ));
        }
        if Instant::now() >= giving_up {
            return Err(anyhow!(
                "the embedding refresh kept emitting for {PROGRESS_TOTAL_CEILING:?} without \
                 ever reaching a terminal phase: {frames:#?}"
            ));
        }
        let (response, emitted) = call_capturing(stdin, stdout, "deslop/reportGet", &json!({}))?;
        if response.get("result").is_none() {
            return Err(anyhow!(
                "reportGet returned no result while polling: {response:#}"
            ));
        }
        // A pass that is still emitting frames is still working, however slow
        // the machine underneath it is, so every frame restarts the clock and
        // only silence is left to run it down.
        if !emitted.is_empty() {
            deadline = stall_deadline();
        }
        frames.extend(emitted);
        std::thread::sleep(POLL_INTERVAL);
    }
}
