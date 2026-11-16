//! The read loop of one framed connection ([LSP-TRANSPORT], [LSP-LIFECYCLE]).
//!
//! Reading never waits on analysis: a request runs on its own task and a
//! notification is queued for one ordered worker, so `exit`, a cancellation
//! or the end of the stream is seen the moment it arrives. Both queues are
//! bounded, so an editor that stops reading its own pipe stops being read
//! ([LIVE-OPEN-BUFFERS]).

use std::sync::Arc;

use lspkit_server::{jsonrpc::read_message, FramingError, Message, MessageWriter};
use serde_json::Value;
use tokio::{
    io::{AsyncRead, AsyncWrite, BufReader},
    sync::{mpsc, watch, Semaphore},
    task::JoinSet,
};

use super::{ClientSocket, ServeEnd, Session, EXIT_METHOD};
use crate::client::Phase;

/// The base-protocol notification that cancels a request in flight.
const CANCEL_METHOD: &str = "$/cancelRequest";

/// Notifications held for the ordered worker before reading waits.
const INBOUND_CAPACITY: usize = 16;

/// Requests answered at once; the rest wait their turn, still cancellable.
const REQUEST_CONCURRENCY: usize = 4;

/// One notification waiting for the ordered worker.
type Queued = (String, Value);

/// The loop's working state.
struct Link {
    /// The session being served.
    session: Arc<Session>,
    /// Queue feeding the ordered notification worker.
    notifications: mpsc::Sender<Queued>,
    /// Requests in flight; dropping the set aborts them.
    requests: JoinSet<()>,
    /// Bounds how many requests run at once.
    permits: Arc<Semaphore>,
    /// Notifications queued so far.
    queued: u64,
    /// Notifications the worker has finished, for requests to wait on.
    handled: watch::Receiver<u64>,
}

/// Serves `session` over `reader`/`writer` until the client says `exit` or
/// goes away. Work still running when it ends is dropped, not awaited: after
/// either ending nobody is left to read its result ([LSP-LIFECYCLE]).
pub(super) async fn serve<R, W>(
    session: Session,
    socket: ClientSocket,
    reader: R,
    writer: W,
) -> ServeEnd
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let session = Arc::new(session);
    let pump = tokio::spawn(pump(socket.frames, MessageWriter::new(writer)));
    let (notifications, queue) = mpsc::channel(INBOUND_CAPACITY);
    let (progress, handled) = watch::channel(0);
    let worker = tokio::spawn(run_notifications(Arc::clone(&session), queue, progress));
    let mut link = Link {
        session: Arc::clone(&session),
        notifications,
        requests: JoinSet::new(),
        permits: Arc::new(Semaphore::new(REQUEST_CONCURRENCY)),
        queued: 0,
        handled,
    };
    let end = link.read(BufReader::new(reader)).await;
    session.dispatcher.cancel_all();
    worker.abort();
    pump.abort();
    end
}

/// Writes every queued frame; stops for good once the editor's pipe fails.
async fn pump<W>(mut frames: mpsc::Receiver<Message>, writer: MessageWriter<W>)
where
    W: AsyncWrite + Unpin + Send,
{
    while let Some(frame) = frames.recv().await {
        if let Err(error) = writer.write_message(&frame).await {
            tracing::warn!(%error, "editor stopped accepting output");
            return;
        }
    }
}

/// Runs notifications one at a time, in the order they were read, and
/// publishes how many are done.
async fn run_notifications(
    session: Arc<Session>,
    mut queue: mpsc::Receiver<Queued>,
    progress: watch::Sender<u64>,
) {
    while let Some((method, params)) = queue.recv().await {
        super::routes::notify(&session.backend, &method, params).await;
        progress.send_modify(|handled| *handled = handled.saturating_add(1));
    }
}

impl Link {
    /// Reads frames until one ends the session or the stream closes.
    async fn read<R: AsyncRead + Unpin>(&mut self, mut reader: BufReader<R>) -> ServeEnd {
        loop {
            match read_message(&mut reader).await {
                Ok(message) => {
                    if let Some(end) = self.accept(message).await {
                        return end;
                    }
                }
                Err(error @ (FramingError::BadJson(_) | FramingError::BadHeader(..))) => {
                    tracing::warn!(%error, "malformed frame ignored");
                }
                Err(error) => return closed(&error),
            }
        }
    }

    /// Routes one frame; `Some` when it ends the session.
    async fn accept(&mut self, message: Message) -> Option<ServeEnd> {
        while self.requests.try_join_next().is_some() {}
        match (message.id.clone(), message.method.clone()) {
            (Some(id), Some(method)) => {
                let answer = self.session.answer(id, &method, message.params);
                let behind = self.notifications_before(&method);
                self.spawn(answer, behind);
            }
            (None, Some(method)) => return self.notification(method, message.params).await,
            (Some(_), None) => self.session.client.resolve(message),
            (None, None) => tracing::debug!("frame with neither id nor method ignored"),
        }
        None
    }

    /// How many notifications a request for `method` waits behind: all that
    /// were read before it, so a report asked for after an edit is a report
    /// of that edit; none for a read of the published snapshot, which must
    /// never wait on analysis ([LSP-NON-INTERFERENCE-NONBLOCKING]).
    fn notifications_before(&self, method: &str) -> u64 {
        if super::routes::reads_snapshot(method) {
            0
        } else {
            self.queued
        }
    }

    /// Runs one admitted request and queues its response, once `before`
    /// notifications have been handled.
    fn spawn(&mut self, answer: super::Answer, before: u64) {
        let permits = Arc::clone(&self.permits);
        let client = self.session.client.clone();
        let mut handled = self.handled.clone();
        let _task = self.requests.spawn(async move {
            let caught_up = handled.wait_for(|done| *done >= before).await.is_ok();
            tracing::trace!(caught_up, "request released behind earlier notifications");
            let _permit = permits.acquire_owned().await;
            let _queued = client.send(answer.await).await;
        });
    }

    /// Handles a notification; `Some` when it is `exit`.
    async fn notification(&mut self, method: String, params: Option<Value>) -> Option<ServeEnd> {
        let params = params.unwrap_or(Value::Null);
        match method.as_str() {
            EXIT_METHOD => return Some(self.exit()),
            CANCEL_METHOD => self
                .session
                .dispatcher
                .cancellation_handle()
                .cancel_request(&params),
            _ if !self.session.takes_notifications(&method) => (),
            _ => {
                super::routes::observe(&self.session.backend, &method, &params);
                self.queued = self.queued.saturating_add(1);
                if self.notifications.send((method, params)).await.is_err() {
                    tracing::error!("notification worker stopped");
                }
            }
        }
        None
    }

    /// [LSP-LIFECYCLE] `exit` is orderly only after `shutdown`.
    fn exit(&self) -> ServeEnd {
        if self.session.phase.get() == Phase::ShutDown {
            ServeEnd::ExitAfterShutdown
        } else {
            ServeEnd::ExitWithoutShutdown
        }
    }
}

/// [LSP-LIFECYCLE] A stream that ended or broke is a client that went away.
fn closed(error: &FramingError) -> ServeEnd {
    if !matches!(error, FramingError::Closed) {
        tracing::warn!(%error, "editor stream failed");
    }
    ServeEnd::ClientVanished
}
