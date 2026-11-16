//! The handle handlers use to talk back to the editor ([LSP-TRANSPORT]).
//!
//! Everything the server sends — notifications, its own requests, and the
//! answers to the editor's requests — goes through one bounded outbox, so an
//! editor that stops reading holds the server back instead of growing a queue
//! ([LIVE-OPEN-BUFFERS]).

use std::sync::Arc;

use lsp_types::{
    notification::{LogMessage, Notification, PublishDiagnostics, ShowMessage},
    request::{Request, ShowDocument, WorkspaceDiagnosticRefresh},
    Diagnostic, LogMessageParams, MessageType, PublishDiagnosticsParams, ShowDocumentParams,
    ShowMessageParams, Uri,
};
use lspkit_server::Message;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::jsonrpc::{internal_error, not_initialized, LspResult};

mod pending;
mod phase;

use pending::PendingReplies;
pub(crate) use phase::{Phase, PhaseCell};

/// Fault message for a request that has no editor left to answer it.
pub(crate) const CONNECTION_CLOSED: &str = "editor connection closed";

/// Frames the outbox holds before a sender waits for the editor to read.
pub const OUTBOX_CAPACITY: usize = 16;

/// Cloneable handle onto one editor connection.
#[derive(Debug, Clone)]
pub struct Client {
    /// Shared connection state.
    inner: Arc<ClientInner>,
}

/// State shared by every clone of a [`Client`].
#[derive(Debug)]
struct ClientInner {
    /// Bounded queue of frames waiting to be written to the editor.
    outbox: mpsc::Sender<Message>,
    /// Requests this server sent that the editor has not answered yet.
    replies: PendingReplies,
    /// Lifecycle phase, shared with the session that advances it.
    phase: Arc<PhaseCell>,
}

impl Client {
    /// A client writing to a fresh outbox; returns the outbox's reading end.
    pub(crate) fn open(phase: Arc<PhaseCell>) -> (Self, mpsc::Receiver<Message>) {
        let (outbox, frames) = mpsc::channel(OUTBOX_CAPACITY);
        let inner = ClientInner {
            outbox,
            replies: PendingReplies::default(),
            phase,
        };
        (
            Self {
                inner: Arc::new(inner),
            },
            frames,
        )
    }

    /// Queues one frame, waiting while the outbox is full. False when the
    /// outbox is closed: the editor has gone and the frame has no reader.
    pub(crate) async fn send(&self, message: Message) -> bool {
        let queued = self.inner.outbox.send(message).await.is_ok();
        if !queued {
            tracing::debug!("editor connection closed; outbound frame dropped");
        }
        queued
    }

    /// Sends notification `N` once the session is initialised; the base
    /// protocol forbids anything but messages and logs before that.
    pub async fn send_notification<N>(&self, params: N::Params)
    where
        N: Notification,
        N::Params: Serialize,
    {
        if self.inner.phase.get().accepts_work() {
            self.notify_unchecked::<N>(params).await;
        } else {
            tracing::debug!(method = N::METHOD, "notification dropped before initialize");
        }
    }

    /// Sends notification `N` in any lifecycle phase.
    async fn notify_unchecked<N>(&self, params: N::Params)
    where
        N: Notification,
        N::Params: Serialize,
    {
        match serde_json::to_value(params) {
            Ok(params) => {
                let _queued = self.send(Message::notification(N::METHOD, params)).await;
            }
            Err(error) => {
                tracing::error!(method = N::METHOD, %error, "notification not serialised");
            }
        }
    }

    /// `window/showMessage`.
    pub async fn show_message(&self, typ: MessageType, message: impl Into<String>) {
        let message = message.into();
        self.notify_unchecked::<ShowMessage>(ShowMessageParams { typ, message })
            .await;
    }

    /// `window/logMessage`.
    pub async fn log_message(&self, typ: MessageType, message: impl Into<String>) {
        let message = message.into();
        self.notify_unchecked::<LogMessage>(LogMessageParams { typ, message })
            .await;
    }

    /// `textDocument/publishDiagnostics`.
    pub async fn publish_diagnostics(
        &self,
        uri: Uri,
        diagnostics: Vec<Diagnostic>,
        version: Option<i32>,
    ) {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version,
        };
        self.send_notification::<PublishDiagnostics>(params).await;
    }

    /// `window/showDocument`; resolves to whether the editor showed it.
    ///
    /// # Errors
    ///
    /// Returns the editor's error, or a fault when the session is not
    /// initialised or the connection closed before the answer.
    pub async fn show_document(&self, params: ShowDocumentParams) -> LspResult<bool> {
        let shown = self.send_request::<ShowDocument>(params).await?;
        Ok(shown.success)
    }

    /// `workspace/diagnostic/refresh`.
    ///
    /// # Errors
    ///
    /// Returns the editor's error, or a fault when the session is not
    /// initialised or the connection closed before the answer.
    pub async fn workspace_diagnostic_refresh(&self) -> LspResult<()> {
        self.send_request::<WorkspaceDiagnosticRefresh>(()).await
    }

    /// Sends request `R` and waits for the editor's answer. Abandoning the
    /// wait forgets the request, so an unanswered one holds nothing
    /// ([LSPKIT-ADOPT-LEAKS]).
    async fn send_request<R>(&self, params: R::Params) -> LspResult<R::Result>
    where
        R: Request,
        R::Params: Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        if !self.inner.phase.get().accepts_work() {
            return Err(not_initialized());
        }
        let params = serde_json::to_value(params).map_err(|e| internal_error(e.to_string()))?;
        let (id, reply) = self.inner.replies.register();
        if !self.send(Message::request(id, R::METHOD, params)).await {
            return Err(internal_error(CONNECTION_CLOSED));
        }
        let value = reply.await?;
        serde_json::from_value(value).map_err(|e| internal_error(e.to_string()))
    }

    /// Hands the editor's answer to the request waiting for it. An answer
    /// nobody waits for is dropped.
    pub(crate) fn resolve(&self, message: Message) {
        let Some(id) = message.id else { return };
        let outcome = match message.error {
            Some(error) => Err(error),
            None => Ok(message.result.unwrap_or(Value::Null)),
        };
        self.inner.replies.resolve(&id, outcome);
    }

    /// Requests sent to the editor and still unanswered.
    #[must_use]
    pub fn pending_requests(&self) -> usize {
        self.inner.replies.len()
    }
}

#[cfg(test)]
mod tests;
