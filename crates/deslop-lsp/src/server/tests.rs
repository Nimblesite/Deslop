//! [LSP-LIFECYCLE] The endings the serve loop must recognise, and what a
//! session holds once its work is over ([LSPKIT-ADOPT-LEAKS]).
//!
//! The integration pin lives in `tests/lifecycle.rs`, which drives the real
//! binary. It caught the original defect only on a slow instrumented runner,
//! where the analysis pass outlived the client. These isolate the mechanism
//! so the contract does not depend on how fast the machine underneath it is.

use std::{future::pending, sync::Arc};

use lspkit_server::{jsonrpc::read_message, Message, MessageWriter, RequestId};
use serde_json::{json, Value};
use tokio::{
    io::{duplex, split, BufReader, DuplexStream, ReadHalf, WriteHalf},
    task::JoinHandle,
    time::{timeout, Duration},
};

use super::{
    connect, routes, stdio, ClientSocket, ServeEnd, Session, EXIT_METHOD, SHUTDOWN_METHOD,
};
use crate::{backend::LspBackend, client::Phase, jsonrpc::LspResult};

/// Long enough that reaching it means "never", short enough to fail fast.
/// Nothing here waits on real work, so a correct implementation resolves
/// immediately and never spends this.
const NEVER: Duration = Duration::from_secs(5);
/// Room for every frame a test exchanges, so no write waits on a reader.
const PIPE_BYTES: usize = 1 << 16;
/// A request whose handler never finishes: the unbounded analysis pass.
const ENDLESS_METHOD: &str = "test/endless";
const MIN_NODES: u32 = 30;
const FIRST_ID: i64 = 1;
const SECOND_ID: i64 = 2;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The editor's end of a served connection.
struct Editor {
    /// Frames the server wrote.
    reader: BufReader<ReadHalf<DuplexStream>>,
    /// Where the editor writes; dropping it closes the server's stdin.
    writer: MessageWriter<WriteHalf<DuplexStream>>,
    /// The serve loop under test.
    serving: JoinHandle<ServeEnd>,
    /// Keeps the fixture workspace alive.
    _workspace: tempfile::TempDir,
}

/// A session over an empty workspace, already initialised, with one extra
/// route that never answers.
fn session() -> TestResult<(Session, ClientSocket, tempfile::TempDir)> {
    let workspace = tempfile::tempdir()?;
    let root = workspace.path().to_path_buf();
    let (session, socket) =
        connect(move |client| LspBackend::new_with_defaults(client, root, MIN_NODES))?;
    session.phase.set(Phase::Initialized);
    session
        .dispatcher
        .register(ENDLESS_METHOD, |_params, cancel| {
            routes::answer(cancel, pending::<LspResult<Value>>())
        });
    Ok((session, socket, workspace))
}

/// Serves a fresh session over an in-memory pipe.
fn editor() -> TestResult<Editor> {
    let (session, socket, workspace) = session()?;
    let (ours, theirs) = duplex(PIPE_BYTES);
    let (server_reader, server_writer) = split(theirs);
    let (reader, writer) = split(ours);
    let serving = tokio::spawn(stdio::serve(session, socket, server_reader, server_writer));
    Ok(Editor {
        reader: BufReader::new(reader),
        writer: MessageWriter::new(writer),
        serving,
        _workspace: workspace,
    })
}

impl Editor {
    /// Sends a request whose handler never finishes.
    async fn start_endless_work(&self) -> TestResult {
        let request = Message::request(RequestId::Number(FIRST_ID), ENDLESS_METHOD, Value::Null);
        Ok(self.writer.write_message(&request).await?)
    }

    /// Sends a notification with no params.
    async fn notify(&self, method: &str) -> TestResult {
        Ok(self
            .writer
            .write_message(&Message::notification(method, Value::Null))
            .await?)
    }

    /// How the serve loop ended, failing if it has not ended.
    async fn ending(self) -> TestResult<ServeEnd> {
        Ok(timeout(NEVER, self.serving).await??)
    }
}

#[tokio::test]
async fn a_closed_stream_ends_the_session_while_work_is_still_running() -> TestResult {
    let editor = editor()?;
    editor.start_endless_work().await?;
    let Editor {
        reader,
        writer,
        serving,
        _workspace,
    } = editor;
    drop((reader, writer));

    let end = timeout(NEVER, serving).await;

    assert_eq!(
        end??,
        ServeEnd::ClientVanished,
        "a client that closed stdin has gone: the loop must end at once rather \
         than wait on the analysis pass nobody will collect, and closing stdin \
         is not an `exit`"
    );
    Ok(())
}

#[tokio::test]
async fn traffic_before_the_stream_closes_is_answered() -> TestResult {
    let mut editor = editor()?;
    let request = Message::request(RequestId::Number(SECOND_ID), SHUTDOWN_METHOD, Value::Null);
    editor.writer.write_message(&request).await?;

    let answer = timeout(NEVER, read_message(&mut editor.reader)).await??;

    assert_eq!(answer.id, Some(RequestId::Number(SECOND_ID)));
    assert!(
        answer.method.is_none(),
        "the frame is a response: {answer:?}"
    );
    assert!(answer.error.is_none(), "shutdown must not fail: {answer:?}");
    assert!(
        !editor.serving.is_finished(),
        "a client that is still there has not gone: shutdown keeps the \
         process running until `exit`"
    );
    Ok(())
}

#[tokio::test]
async fn exit_ends_the_loop_while_work_is_still_running() -> TestResult {
    let editor = editor()?;
    editor.start_endless_work().await?;
    editor.notify(EXIT_METHOD).await?;

    assert_eq!(
        editor.ending().await?,
        ServeEnd::ExitWithoutShutdown,
        "the loop must end on `exit` alone — waiting for a pass with no \
         client left to read it is the leak this contract forbids"
    );
    Ok(())
}

#[tokio::test]
async fn exit_after_shutdown_is_the_orderly_ending() -> TestResult {
    let mut editor = editor()?;
    let request = Message::request(RequestId::Number(FIRST_ID), SHUTDOWN_METHOD, Value::Null);
    editor.writer.write_message(&request).await?;
    let _answer = timeout(NEVER, read_message(&mut editor.reader)).await??;
    editor.notify(EXIT_METHOD).await?;

    assert_eq!(editor.ending().await?, ServeEnd::ExitAfterShutdown);
    Ok(())
}

#[tokio::test]
async fn a_cancelled_request_is_answered_as_cancelled() -> TestResult {
    let mut editor = editor()?;
    editor.start_endless_work().await?;
    let cancel = Message::notification("$/cancelRequest", json!({ "id": FIRST_ID }));
    editor.writer.write_message(&cancel).await?;

    let answer = timeout(NEVER, read_message(&mut editor.reader)).await??;

    assert_eq!(answer.id, Some(RequestId::Number(FIRST_ID)));
    assert_eq!(
        answer.error.map(|error| error.code),
        Some(crate::jsonrpc::REQUEST_CANCELLED),
        "a cancelled request must stop waiting on its work and say so"
    );
    Ok(())
}

#[tokio::test]
async fn a_request_before_initialize_is_refused_and_one_after_shutdown_is_invalid() -> TestResult {
    let (session, _socket, _workspace) = session()?;
    session.phase.set(Phase::Uninitialized);
    let early = Message::request(RequestId::Number(FIRST_ID), ENDLESS_METHOD, Value::Null);

    let refused = session.call(early).await.and_then(|answer| answer.error);

    assert_eq!(
        refused.map(|error| error.code),
        Some(crate::jsonrpc::SERVER_NOT_INITIALIZED)
    );
    session.phase.set(Phase::ShutDown);
    let late = Message::request(RequestId::Number(SECOND_ID), ENDLESS_METHOD, Value::Null);
    let invalid = session.call(late).await.and_then(|answer| answer.error);
    assert_eq!(
        invalid.map(|error| error.code),
        Some(crate::jsonrpc::INVALID_REQUEST)
    );
    Ok(())
}

#[tokio::test]
async fn an_unknown_request_is_method_not_found() -> TestResult {
    let (session, _socket, _workspace) = session()?;
    let unknown = Message::request(RequestId::Number(FIRST_ID), "test/unknown", Value::Null);

    let answer = session.call(unknown).await.and_then(|answer| answer.error);

    assert_eq!(
        answer.map(|error| error.code),
        Some(crate::jsonrpc::METHOD_NOT_FOUND)
    );
    Ok(())
}

#[tokio::test]
async fn dropping_the_session_releases_the_backend() -> TestResult {
    let (session, socket, _workspace) = session()?;
    let backend = Arc::downgrade(&session.backend);
    assert!(backend.upgrade().is_some(), "the session holds its backend");

    drop(session);
    drop(socket);

    assert!(
        backend.upgrade().is_none(),
        "the routes hold the backend and the session holds the routes; \
         nothing else may, or a closed session's analysis state is never freed"
    );
    Ok(())
}
