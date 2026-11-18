//! [LSP-LIFECYCLE] The endings the serve loop must recognise, and what a
//! session holds once its work is over ([LSPKIT-ADOPT-LEAKS]), and how the
//! read loop dispatches each frame ([LSP-TRANSPORT-DISPATCH]).
//!
//! The integration pin lives in `tests/lifecycle.rs`, which drives the real
//! binary. It caught the original defect only on a slow instrumented runner,
//! where the analysis pass outlived the client. These isolate the mechanism
//! so the contract does not depend on how fast the machine underneath it is.

use std::{future::pending, sync::Arc};

use lspkit_server::{jsonrpc::read_message, Message, MessageWriter, RequestId};
use serde_json::{json, Value};
use tokio::{
    io::{duplex, split, AsyncWrite, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf},
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
/// The file the editor opens while an analysis pass is in flight.
const EDITED_FILE: &str = "Edited.cs";
const EDITED_LANGUAGE: &str = "csharp";
const EDITED_TEXT: &str = "class Edited {}\n";
const FIRST_VERSION: i32 = 1;

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
    Ok(serve(session, socket, workspace))
}

/// Serves `session` over an in-memory pipe.
fn serve(session: Session, socket: ClientSocket, workspace: tempfile::TempDir) -> Editor {
    let (ours, theirs) = duplex(PIPE_BYTES);
    let (server_reader, server_writer) = split(theirs);
    let (reader, writer) = split(ours);
    let serving = tokio::spawn(stdio::serve(session, socket, server_reader, server_writer));
    Editor {
        reader: BufReader::new(reader),
        writer: MessageWriter::new(writer),
        serving,
        _workspace: workspace,
    }
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

#[tokio::test]
async fn a_request_waits_for_the_notifications_read_before_it() -> TestResult {
    let mut editor = editor()?;
    editor.notify("initialized").await?;
    let request = Message::request(RequestId::Number(SECOND_ID), SHUTDOWN_METHOD, Value::Null);
    editor.writer.write_message(&request).await?;

    let mut before_answer = Vec::new();
    let answer = loop {
        let frame = timeout(NEVER, read_message(&mut editor.reader)).await??;
        if frame.id == Some(RequestId::Number(SECOND_ID)) {
            break frame;
        }
        before_answer.push(frame.method.unwrap_or_default());
    };

    assert!(answer.error.is_none(), "shutdown must not fail: {answer:?}");
    assert!(
        before_answer
            .iter()
            .any(|method| method == "window/logMessage"),
        "everything the `initialized` notification pushed must be on the wire \
         before the answer to a request read after it — a report asked for \
         after an edit must be a report of that edit; saw {before_answer:?}"
    );
    Ok(())
}

/// [LSP-NON-INTERFERENCE-NONBLOCKING] The session mutex held here is the
/// analysis pass in flight; the opened file's own pass waits behind it.
#[tokio::test]
async fn an_editor_read_is_answered_while_an_edit_is_still_being_analysed() -> TestResult {
    let (session, socket, workspace) = session()?;
    let uri = crate::backend::path_to_uri(&workspace.path().join(EDITED_FILE))
        .ok_or("the edited file has a file URI")?;
    let analysis = session.backend().service().session();
    let pass_in_flight = analysis.lock().await;
    let mut editor = serve(session, socket, workspace);

    let opened = json!({ "textDocument": {
        "uri": uri, "languageId": EDITED_LANGUAGE, "version": FIRST_VERSION, "text": EDITED_TEXT,
    }});
    let open = Message::notification("textDocument/didOpen", opened);
    editor.writer.write_message(&open).await?;
    let lenses = Message::request(
        RequestId::Number(SECOND_ID),
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
    );
    editor.writer.write_message(&lenses).await?;

    let answered = timeout(NEVER, async {
        loop {
            match read_message(&mut editor.reader).await {
                Ok(frame) if frame.id == Some(RequestId::Number(SECOND_ID)) => break Some(frame),
                Ok(_) => {}
                Err(_) => break None,
            }
        }
    })
    .await;
    drop(pass_in_flight);

    let answer = answered
        .map_err(|_| {
            "code lenses read the published report snapshot, so they must be answered while \
             the analysis of an earlier edit still holds the session; they waited for it"
        })?
        .ok_or("the server closed the connection instead of answering")?;
    assert!(
        answer.error.is_none(),
        "code lenses must not fail: {answer:?}"
    );
    assert_eq!(
        answer.result,
        Some(json!([])),
        "an empty workspace has no clone badges to show"
    );
    Ok(())
}

/// Bytes of filler written per chunk while streaming an oversized body.
const FILLER_CHUNK: usize = 1 << 16;

/// One past the largest body the reader accepts ([JSONRPC-FRAME-LIMIT]).
const OVERSIZED_BODY: usize = lspkit_server::jsonrpc::MAX_FRAME_BYTES + 1;

/// Writes a frame claiming, and carrying, `OVERSIZED_BODY` bytes of filler.
async fn write_oversized_frame<W: AsyncWrite + Unpin>(writer: &mut W) -> TestResult {
    let header = format!("Content-Length: {OVERSIZED_BODY}\r\n\r\n");
    writer.write_all(header.as_bytes()).await?;
    let chunk = vec![b' '; FILLER_CHUNK];
    let mut left = OVERSIZED_BODY;
    while left > 0 {
        let take = left.min(FILLER_CHUNK);
        writer
            .write_all(chunk.get(..take).ok_or("chunk slice")?)
            .await?;
        left = left.saturating_sub(take);
    }
    Ok(())
}

/// Writes `message` with its `Content-Length` header.
async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, message: &Message) -> TestResult {
    let body = serde_json::to_vec(message)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    Ok(writer.write_all(&body).await?)
}

#[tokio::test]
async fn a_refused_oversized_frame_leaves_the_session_serving_the_next_request() -> TestResult {
    let (session, socket, _workspace) = session()?;
    let (ours, theirs) = duplex(PIPE_BYTES);
    let (server_reader, server_writer) = split(theirs);
    let (reader, mut writer) = split(ours);
    let serving = tokio::spawn(stdio::serve(session, socket, server_reader, server_writer));
    let shutdown = Message::request(RequestId::Number(SECOND_ID), SHUTDOWN_METHOD, Value::Null);

    write_oversized_frame(&mut writer).await?;
    write_frame(&mut writer, &shutdown).await?;
    let answer = timeout(NEVER, read_message(&mut BufReader::new(reader))).await?;

    let answer = answer.map_err(|error| {
        format!("one refused frame must not end the session, but the stream closed: {error}")
    })?;
    assert_eq!(
        answer.id,
        Some(RequestId::Number(SECOND_ID)),
        "the request after the refused frame is the one answered"
    );
    assert!(answer.error.is_none(), "shutdown must not fail: {answer:?}");
    assert!(
        !serving.is_finished(),
        "a client that sent one oversized frame is still there: the session keeps serving"
    );
    Ok(())
}
