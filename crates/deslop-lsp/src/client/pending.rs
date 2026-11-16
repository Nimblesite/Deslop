//! Requests the server sent that the editor has not answered
//! ([LSPKIT-ADOPT-LEAKS]).
//!
//! A wait that is abandoned — the handler was cancelled, the editor never
//! answered — removes its own entry, so an unanswered request holds no
//! allocation for the life of the process.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex, PoisonError, Weak,
    },
    task::{Context, Poll},
};

use lspkit_server::RequestId;
use serde_json::Value;
use tokio::sync::oneshot;

use crate::jsonrpc::{internal_error, LspResult};

/// First id handed out; ids only ever grow from here.
const FIRST_REQUEST_ID: i64 = 1;

/// The editor's answer to one request.
type Reply = LspResult<Value>;
/// Answer channels by request id.
type Waiting = Mutex<HashMap<i64, oneshot::Sender<Reply>>>;

/// Every request still waiting for the editor.
#[derive(Debug)]
pub(super) struct PendingReplies {
    /// Next id to hand out.
    next: AtomicI64,
    /// Channels of the requests in flight.
    waiting: Arc<Waiting>,
}

impl Default for PendingReplies {
    fn default() -> Self {
        Self {
            next: AtomicI64::new(FIRST_REQUEST_ID),
            waiting: Arc::default(),
        }
    }
}

impl PendingReplies {
    /// Registers a new request; the returned wait forgets it when dropped.
    pub(super) fn register(&self) -> (RequestId, PendingReply) {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = oneshot::channel();
        let _replaced = lock(&self.waiting).insert(id, sender);
        let reply = PendingReply {
            id,
            waiting: Arc::downgrade(&self.waiting),
            receiver,
        };
        (RequestId::Number(id), reply)
    }

    /// An id for a request whose answer nobody will wait for.
    pub(super) fn unawaited_id(&self) -> RequestId {
        RequestId::Number(self.next.fetch_add(1, Ordering::SeqCst))
    }

    /// Delivers `outcome` to the request `id` names, if one still waits.
    pub(super) fn resolve(&self, id: &RequestId, outcome: Reply) {
        let RequestId::Number(number) = id else {
            return;
        };
        let Some(sender) = lock(&self.waiting).remove(number) else {
            return unawaited(*number, &outcome);
        };
        if sender.send(outcome).is_err() {
            tracing::debug!(id = number, "request abandoned as its answer arrived");
        }
    }

    /// Requests in flight.
    pub(super) fn len(&self) -> usize {
        lock(&self.waiting).len()
    }
}

/// An answer nobody waits for is only worth a line when the editor refused.
fn unawaited(id: i64, outcome: &Reply) {
    if let Err(error) = outcome {
        tracing::warn!(id, code = error.code, message = %error.message, "editor refused a request");
    }
}

/// The table, recovered from a poisoned lock: it holds only channel ends.
fn lock(waiting: &Waiting) -> std::sync::MutexGuard<'_, HashMap<i64, oneshot::Sender<Reply>>> {
    waiting.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The wait for one answer.
#[derive(Debug)]
pub(super) struct PendingReply {
    /// Id of the request this wait belongs to.
    id: i64,
    /// The table to leave on drop; weak so a wait never keeps it alive.
    waiting: Weak<Waiting>,
    /// Where the answer arrives.
    receiver: oneshot::Receiver<Reply>,
}

impl Future for PendingReply {
    type Output = Reply;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.receiver).poll(context).map(|received| {
            received.unwrap_or_else(|_| Err(internal_error(super::CONNECTION_CLOSED)))
        })
    }
}

impl Drop for PendingReply {
    fn drop(&mut self) {
        if let Some(waiting) = self.waiting.upgrade() {
            let _removed = lock(&waiting).remove(&self.id);
        }
    }
}
