//! [LSPKIT-ADOPT-LEAKS] A request the server sends holds an entry only
//! while something waits for its answer.

use lsp_types::ShowDocumentParams;
use serde_json::json;
use tokio::time::{timeout, Duration};

use crate::server::loopback;

/// Long enough that reaching it means "never"; nothing here waits on work.
const NEVER: Duration = Duration::from_secs(5);
const DOCUMENT: &str = "deslop://report";

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The `window/showDocument` params every test here sends.
fn show() -> TestResult<ShowDocumentParams> {
    Ok(ShowDocumentParams {
        uri: DOCUMENT.parse()?,
        external: Some(false),
        take_focus: Some(true),
        selection: None,
    })
}

#[tokio::test]
async fn an_abandoned_request_leaves_nothing_behind() -> TestResult {
    let (client, mut socket) = loopback();
    let asking = client.clone();
    let params = show()?;
    let waiting = tokio::spawn(async move { asking.show_document(params).await });
    let request = timeout(NEVER, socket.next())
        .await?
        .ok_or("no request sent")?;
    assert_eq!(request.method.as_deref(), Some("window/showDocument"));
    assert_eq!(client.pending_requests(), 1, "the request is in flight");

    waiting.abort();
    let outcome = timeout(NEVER, waiting).await?;

    assert!(outcome.is_err(), "the wait was abandoned, not answered");
    assert_eq!(
        client.pending_requests(),
        0,
        "an editor that never answers must not leave an entry for the life \
         of the process"
    );
    let late = request.id.ok_or("request carried no id")?;
    socket.answer(late, json!({ "success": true }));
    assert_eq!(
        client.pending_requests(),
        0,
        "a late answer registers nothing"
    );
    Ok(())
}

#[tokio::test]
async fn an_answered_request_resolves_and_is_forgotten() -> TestResult {
    let (client, mut socket) = loopback();
    let asking = client.clone();
    let params = show()?;
    let waiting = tokio::spawn(async move { asking.show_document(params).await });
    let request = timeout(NEVER, socket.next())
        .await?
        .ok_or("no request sent")?;
    let id = request.id.ok_or("request carried no id")?;

    socket.answer(id, json!({ "success": true }));
    let shown = timeout(NEVER, waiting).await??;

    assert_eq!(
        shown.ok(),
        Some(true),
        "the editor's answer reaches the caller"
    );
    assert_eq!(client.pending_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn a_closed_connection_fails_the_wait_instead_of_parking_it() -> TestResult {
    let (client, socket) = loopback();
    drop(socket);

    let outcome = timeout(NEVER, client.show_document(show()?)).await?;

    assert_eq!(
        outcome.err().map(|error| error.code),
        Some(crate::jsonrpc::INTERNAL_ERROR),
        "with the connection gone the wait must end, not hold its task forever"
    );
    assert_eq!(
        client.pending_requests(),
        0,
        "a request that was never sent must not stay registered"
    );
    Ok(())
}
