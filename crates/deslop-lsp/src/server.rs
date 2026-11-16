//! LSP session over `lspkit-server` — admission, dispatch and the stdio
//! bootstrap ([LSP-TRANSPORT], [LSP-LIFECYCLE]).
//!
//! `lspkit-server` supplies the framing, the dispatcher and request
//! cancellation; this module owns only what is Deslop's: which methods
//! exist, and which lifecycle phase admits them.

use std::{future::Future, path::PathBuf, pin::Pin, process::ExitCode, sync::Arc};

use lspkit_server::{DispatchError, Dispatcher, HandlerResult, Message, RequestId};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{
    backend::{LspBackend, LspEmbeddingConfig},
    client::{Client, Phase, PhaseCell},
    custom_methods,
    jsonrpc::{
        internal_error, invalid_request, method_not_found, not_initialized, Error, LspResult,
    },
};

mod routes;
mod stdio;

/// Methods that accept an empty-object `params` payload and must also
/// accept a missing `params` field — some JSON-RPC clients omit it for
/// no-arg calls.
const NO_PARAM_METHODS: &[&str] = &[
    custom_methods::REPORT_GET,
    custom_methods::REPORT_DELTA,
    custom_methods::LIST_MODELS,
    custom_methods::SESSION_CONFIG,
    custom_methods::REPORT_SCHEMA_DOC,
    custom_methods::CPU_REPORT,
];

/// The LSP base-protocol request that opens a session.
const INITIALIZE_METHOD: &str = "initialize";

/// The LSP base-protocol method that stops the server accepting work.
const SHUTDOWN_METHOD: &str = "shutdown";

/// The LSP base-protocol notification that ends the process.
const EXIT_METHOD: &str = "exit";

/// [LSP-LIFECYCLE] Why the serve loop ended. The base protocol fixes a
/// different process exit code for each, and editors read it: an orderly
/// teardown is a success, an `exit` with no `shutdown` before it is the
/// client tearing the session down out of order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeEnd {
    /// `exit` arrived after `shutdown` — the orderly teardown.
    ExitAfterShutdown,
    /// `exit` arrived with no preceding `shutdown`.
    ExitWithoutShutdown,
    /// The client went away without saying anything, closing stdin. Not a
    /// server fault, so not a failure.
    ClientVanished,
}

impl ServeEnd {
    /// The process exit code the base protocol requires for this ending.
    #[must_use = "the exit code is how the process reports this ending"]
    pub fn exit_code(self) -> ExitCode {
        match self {
            Self::ExitAfterShutdown | Self::ClientVanished => ExitCode::SUCCESS,
            Self::ExitWithoutShutdown => ExitCode::from(1),
        }
    }
}

/// The answer to one request, ready to be awaited off the read loop.
type Answer = Pin<Box<dyn Future<Output = Message> + Send>>;

/// One editor session: the backend, its routes and its lifecycle phase.
#[derive(Debug)]
pub struct Session {
    /// The analysis backend every route forwards to.
    backend: Arc<LspBackend>,
    /// Request routes and their cancellation registry.
    dispatcher: Dispatcher,
    /// Lifecycle phase, shared with the client handle.
    phase: Arc<PhaseCell>,
    /// Handle onto the editor.
    client: Client,
}

/// The editor's side of an in-process session: every frame the server
/// sends, and the way to answer the requests among them.
#[derive(Debug)]
pub struct ClientSocket {
    /// Frames the server queued for the editor.
    frames: mpsc::Receiver<Message>,
    /// Routes the editor's answers back to the waiting requests.
    client: Client,
}

impl ClientSocket {
    /// The next frame the server sends; `None` once the session is gone.
    pub async fn next(&mut self) -> Option<Message> {
        self.frames.recv().await
    }

    /// The next frame already queued, without waiting.
    pub fn try_next(&mut self) -> Option<Message> {
        self.frames.try_recv().ok()
    }

    /// Answers a request the server sent.
    pub fn answer(&self, id: RequestId, result: Value) {
        self.client.resolve(Message::response(id, result));
    }
}

/// Builds a session around the backend `build` constructs from its client
/// handle, and returns the editor's side of it.
///
/// # Errors
///
/// Returns `build`'s error when the backend cannot be constructed.
pub fn connect<F, E>(build: F) -> Result<(Session, ClientSocket), E>
where
    F: FnOnce(Client) -> Result<LspBackend, E>,
{
    let phase = Arc::new(PhaseCell::default());
    let (client, frames) = Client::open(Arc::clone(&phase));
    let backend = Arc::new(build(client.clone())?);
    let dispatcher = Dispatcher::new();
    routes::register(&dispatcher, &backend);
    let socket = ClientSocket {
        frames,
        client: client.clone(),
    };
    let session = Session {
        backend,
        dispatcher,
        phase,
        client,
    };
    Ok((session, socket))
}

impl Session {
    /// The backend this session serves.
    #[must_use]
    pub fn backend(&self) -> &LspBackend {
        &self.backend
    }

    /// Handles one inbound frame to completion: the response for a request,
    /// `None` for a notification or an answer to a server request.
    pub async fn call(&self, message: Message) -> Option<Message> {
        match (message.id.clone(), message.method.clone()) {
            (Some(id), Some(method)) => Some(self.answer(id, &method, message.params).await),
            (None, Some(method)) => {
                self.notify(&method, message.params.unwrap_or(Value::Null))
                    .await;
                None
            }
            (Some(_), None) => {
                self.client.resolve(message);
                None
            }
            (None, None) => None,
        }
    }

    /// Admits a request and registers its cancellation before returning, so
    /// a `$/cancelRequest` read next can never miss it.
    fn answer(&self, id: RequestId, method: &str, params: Option<Value>) -> Answer {
        let admitted = self.admit(method);
        let opening = admitted.is_ok() && method == INITIALIZE_METHOD;
        let prepared = admitted.and_then(|()| self.prepare(id.clone(), method, params));
        let phase = Arc::clone(&self.phase);
        Box::pin(async move {
            let result = match prepared {
                Ok(work) => work.await,
                Err(error) => HandlerResult::Err(error),
            };
            if opening {
                settle_initialize(&phase, &result);
            }
            response(id, result)
        })
    }

    /// [LSP-LIFECYCLE] Whether the current phase admits a request for `method`.
    fn admit(&self, method: &str) -> LspResult<()> {
        match self.phase.get() {
            Phase::Uninitialized if method == INITIALIZE_METHOD => self
                .phase
                .advance(Phase::Uninitialized, Phase::Initializing)
                .then_some(())
                .ok_or_else(invalid_request),
            Phase::Uninitialized | Phase::Initializing => Err(not_initialized()),
            Phase::Initialized if method == INITIALIZE_METHOD => Err(invalid_request()),
            Phase::Initialized => {
                if method == SHUTDOWN_METHOD {
                    self.phase.set(Phase::ShutDown);
                }
                Ok(())
            }
            Phase::ShutDown => Err(invalid_request()),
        }
    }

    /// Registers the request with the dispatcher and returns its work.
    fn prepare(
        &self,
        id: RequestId,
        method: &str,
        params: Option<Value>,
    ) -> LspResult<impl Future<Output = HandlerResult> + Send + 'static> {
        let params = params.unwrap_or_else(|| missing_params(method));
        self.dispatcher
            .prepare(id, method, params)
            .map_err(|error| dispatch_fault(&error))
    }

    /// Runs a notification's handler if the session takes notifications now.
    async fn notify(&self, method: &str, params: Value) {
        if self.takes_notifications(method) {
            routes::observe(&self.backend, method, &params);
            routes::notify(&self.backend, method, params).await;
        }
    }

    /// Whether a notification read now is to be handled. Outside an
    /// initialised session the base protocol says to drop it. Decided when
    /// the frame is read, not when its turn comes: a `shutdown` read after
    /// an edit must not make that edit vanish.
    fn takes_notifications(&self, method: &str) -> bool {
        let taken = self.phase.get() == Phase::Initialized;
        if !taken {
            tracing::debug!(
                method,
                "notification outside an initialised session dropped"
            );
        }
        taken
    }
}

/// The value a handler sees when the request carried no `params`.
fn missing_params(method: &str) -> Value {
    if NO_PARAM_METHODS.contains(&method) {
        Value::Object(serde_json::Map::new())
    } else {
        Value::Null
    }
}

/// `initialize` succeeded: the session works. It failed: the editor may retry.
fn settle_initialize(phase: &PhaseCell, result: &HandlerResult) {
    let next = if matches!(result, HandlerResult::Ok(_)) {
        Phase::Initialized
    } else {
        Phase::Uninitialized
    };
    phase.set(next);
}

/// The fault a request gets when the dispatcher could not schedule it.
fn dispatch_fault(error: &DispatchError) -> Error {
    match error {
        DispatchError::UnknownMethod(_) => method_not_found(),
        other => internal_error(other.to_string()),
    }
}

/// Shapes a handler outcome into the response frame for `id`.
fn response(id: RequestId, result: HandlerResult) -> Message {
    match result {
        HandlerResult::Ok(value) => Message::response(id, value),
        HandlerResult::Err(error) => Message::response_error(id, error),
        _ => Message::response_error(id, internal_error("unrecognised handler outcome")),
    }
}

/// Boots the LSP server over stdio. Used by the binary entry point
/// and by E2E tests that drive the binary as a black box.
///
/// Returns how the serve loop ended so the process can report the exit
/// code the base protocol requires ([LSP-LIFECYCLE]).
///
/// # Errors
///
/// Returns `Err` when the backend fails to construct; the editor surfaces
/// the failed process through its standard "server crashed" UX.
pub async fn run_stdio(
    workspace_root: PathBuf,
    min_nodes: u32,
    embedding: LspEmbeddingConfig,
    ipc_mode: deslop_core::live::transport::IpcMode,
) -> anyhow::Result<ServeEnd> {
    tracing::info!(
        workspace_root = %workspace_root.display(),
        exists = workspace_root.exists(),
        is_dir = workspace_root.is_dir(),
        min_nodes,
        "run_stdio booting backend",
    );
    let (session, socket) = connect(move |client| {
        LspBackend::new_with_config(client, workspace_root, min_nodes, &embedding, ipc_mode)
    })
    .inspect_err(|error| tracing::error!(%error, "deslop-lsp backend failed to initialise"))?;
    let end = stdio::serve(session, socket, tokio::io::stdin(), tokio::io::stdout()).await;
    tracing::info!(?end, "serve loop ended");
    Ok(end)
}

/// A client handle in an initialised session and the editor's side of it,
/// for tests that exercise server pushes without a backend.
#[cfg(test)]
pub(crate) fn loopback() -> (Client, ClientSocket) {
    let phase = Arc::new(PhaseCell::default());
    phase.set(Phase::Initialized);
    let (client, frames) = Client::open(phase);
    let socket = ClientSocket {
        frames,
        client: client.clone(),
    };
    (client, socket)
}

#[cfg(test)]
mod tests;
