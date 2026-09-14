//! LSP server bootstrap — stdio transport and request normalisation
//! middleware ([LSP-TRANSPORT]).
//!
//! Extracted from `backend` to keep that module under the 500-line
//! budget while isolating all tower-lsp wiring in one place.

use std::{
    path::PathBuf,
    pin::Pin,
    process::ExitCode,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, ReadBuf},
    sync::Notify,
};
use tower::Service;
use tower_lsp::{
    jsonrpc::{Request, Response},
    ExitedError, LspService, Server,
};

use crate::{
    backend::{LspBackend, LspEmbeddingConfig},
    custom_methods,
};

/// Methods that accept an empty-object `params` payload and must also
/// accept a missing `params` field — some JSON-RPC clients omit it for
/// no-arg calls, and tower-lsp's router rejects the request with
/// `-32602 Missing params field` before the handler runs.
const NO_PARAM_METHODS: &[&str] = &[
    custom_methods::REPORT_GET,
    custom_methods::REPORT_DELTA,
    custom_methods::LIST_MODELS,
    custom_methods::SESSION_CONFIG,
    custom_methods::REPORT_SCHEMA_DOC,
    custom_methods::CPU_REPORT,
];

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
    #[must_use]
    pub fn exit_code(self) -> ExitCode {
        match self {
            Self::ExitAfterShutdown | Self::ClientVanished => ExitCode::SUCCESS,
            Self::ExitWithoutShutdown => ExitCode::from(1),
        }
    }
}

/// Lifecycle messages seen on the wire, shared between the middleware that
/// observes them and the caller that reports the ending.
#[derive(Debug, Clone, Default)]
struct Lifecycle {
    /// Set once the client has sent `shutdown`.
    shutdown: Arc<AtomicBool>,
    /// Set once the client has sent `exit`.
    exit: Arc<AtomicBool>,
    /// Raised the moment `exit` is seen, so the serve loop can end without
    /// waiting for a message the client is not going to send.
    exited: Arc<Notify>,
}

impl Lifecycle {
    /// Records `method` when it is one of the two lifecycle messages.
    fn observe(&self, method: &str) {
        match method {
            SHUTDOWN_METHOD => self.shutdown.store(true, Ordering::SeqCst),
            EXIT_METHOD => {
                self.exit.store(true, Ordering::SeqCst);
                self.exited.notify_one();
            }
            _ => (),
        }
    }

    /// How the serve loop ended, read once it has returned.
    fn end(&self) -> ServeEnd {
        match (
            self.exit.load(Ordering::SeqCst),
            self.shutdown.load(Ordering::SeqCst),
        ) {
            (true, true) => ServeEnd::ExitAfterShutdown,
            (true, false) => ServeEnd::ExitWithoutShutdown,
            (false, _) => ServeEnd::ClientVanished,
        }
    }
}

/// Service adapter that records the base-protocol lifecycle as it passes.
#[derive(Debug)]
struct WatchLifecycle<S> {
    /// Wrapped service that handles the message.
    inner: S,
    /// Shared state the caller reads after the serve loop ends.
    lifecycle: Lifecycle,
}

impl<S> Service<Request> for WatchLifecycle<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
{
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.lifecycle.observe(req.method());
        self.inner.call(req)
    }
}

/// Service adapter that injects an empty-object `params` value on
/// selected custom methods when the incoming request omitted it.
#[derive(Debug)]
struct NormaliseParams<S> {
    /// Wrapped service that receives the normalised request.
    inner: S,
}

impl<S> NormaliseParams<S> {
    /// Wraps `inner` so incoming requests are normalised before reaching it.
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Service<Request> for NormaliseParams<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
{
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let normalised = if req.params().is_none()
            && NO_PARAM_METHODS
                .iter()
                .any(|method| *method == req.method())
        {
            rebuild_with_empty_params(req)
        } else {
            req
        };
        self.inner.call(normalised)
    }
}

/// Rebuilds `req` with a `params: {}` payload. Preserves method and id.
fn rebuild_with_empty_params(req: Request) -> Request {
    let (method, id, _params) = req.into_parts();
    let mut builder = Request::build(method).params(serde_json::json!({}));
    if let Some(id) = id {
        builder = builder.id(id);
    }
    builder.finish()
}

/// Aborts the process with a structured diagnostic when the backend
/// cannot construct. The editor surfaces this through the standard
/// "server crashed" UX.
fn report_init_failure(error: &deslop_core::live::LiveError) -> ! {
    tracing::error!(%error, "deslop-lsp backend failed to initialise");
    std::process::exit(1)
}

/// Boots the LSP server over stdio. Used by the binary entry point
/// and by E2E tests that drive the binary as a black box.
///
/// Returns how the serve loop ended so the process can report the exit
/// code the base protocol requires ([LSP-LIFECYCLE]).
///
/// # Errors
///
/// Returns `Err` when the backend fails to construct.
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
    let workspace_root_for_builder = workspace_root;
    let (service, socket) = LspService::build(move |client| {
        match LspBackend::new_with_config(
            client,
            workspace_root_for_builder.clone(),
            min_nodes,
            &embedding,
            ipc_mode,
        ) {
            Ok(backend) => backend,
            Err(error) => report_init_failure(&error),
        }
    })
    .custom_method(custom_methods::REPORT_GET, custom_methods::report_get)
    .custom_method(custom_methods::REPORT_DELTA, custom_methods::report_delta)
    .custom_method(
        custom_methods::REPORT_FOR_FILE,
        custom_methods::report_for_file,
    )
    .custom_method(
        custom_methods::REPORT_FOR_RANGE,
        custom_methods::report_for_range,
    )
    .custom_method(custom_methods::CLUSTER_BY_ID, custom_methods::cluster_by_id)
    .custom_method(custom_methods::PAIR_COMPARE, custom_methods::pair_compare)
    .custom_method(custom_methods::FIND_SIMILAR, custom_methods::find_similar)
    .custom_method(
        custom_methods::LIST_MODELS,
        custom_methods::embedding_list_models,
    )
    .custom_method(
        custom_methods::SET_MODEL,
        custom_methods::embedding_set_model,
    )
    .custom_method(
        custom_methods::SESSION_CONFIG,
        custom_methods::session_config,
    )
    .custom_method(
        custom_methods::REPORT_SCHEMA_DOC,
        custom_methods::report_schema_doc,
    )
    .custom_method(custom_methods::CPU_REPORT, custom_methods::cpu_report)
    .custom_method(
        custom_methods::VIRTUAL_DOCUMENT,
        custom_methods::virtual_document,
    )
    .finish();
    let lifecycle = Lifecycle::default();
    let stdin = EndOnEof {
        inner: tokio::io::stdin(),
        ended: lifecycle.exited.clone(),
    };
    let stdout = tokio::io::stdout();
    let serving = Server::new(stdin, stdout, socket).serve(WatchLifecycle {
        inner: NormaliseParams::new(service),
        lifecycle: lifecycle.clone(),
    });
    serve_until_exit(serving, &lifecycle.exited).await;
    let end = lifecycle.end();
    tracing::info!(?end, "serve loop ended");
    Ok(end)
}

/// Serves until the transport ends or the client sends `exit`.
///
/// tower-lsp's read loop only notices that the server has exited when the
/// *next* message arrives, and after `exit` the base protocol tells the
/// client there is nothing left to send. Waiting on the transport alone
/// therefore parks forever on a client that did exactly what it was told,
/// and the process outlives every editor window that opened it
/// ([LSP-LIFECYCLE]).
///
/// `biased` keeps the transport first, so the `exit` message is fully
/// handled — tower-lsp cancels pending work and closes the client inside
/// the same synchronous `call` that raises this signal — before the loop
/// is allowed to end.
async fn serve_until_exit<F>(serving: F, exited: &Notify)
where
    F: std::future::Future<Output = ()>,
{
    tokio::pin!(serving);
    tokio::select! {
        biased;
        () = &mut serving => (),
        () = exited.notified() => (),
    }
}

/// A reader that raises `ended` the moment it reaches end of file.
///
/// [LSP-LIFECYCLE] Closing stdin is how a client that did not get to say
/// anything says goodbye — it crashed, or the editor window went away. The
/// base protocol treats that as `exit`, and so must this process.
///
/// tower-lsp's serve future does not resolve on end of file alone: it first
/// lets the work already in flight finish. After `exit` that is right, since
/// the client asked for an orderly stop and is still there to be answered.
/// After the client has vanished it is the leak this module exists to
/// prevent, because the work being waited on is unbounded — gh #370 was a
/// refresh that ran for fourteen minutes, and nobody was left to read it.
///
/// The failure is invisible on a fast machine, where the pass is finished
/// before the pipe closes, which is why it surfaced first as a CI timeout:
/// the instrumented two-core runner held stdout open for the full
/// two-minute ceiling. Raising the same signal `exit` raises ends the loop
/// at once, and leaves the ending — and therefore the process exit code —
/// classified as [`ServeEnd::ClientVanished`] rather than as an `exit`.
struct EndOnEof<R> {
    /// The reader being watched, normally the process's stdin.
    inner: R,
    /// Raised once, when `inner` first reports end of file.
    ended: Arc<Notify>,
}

impl<R: AsyncRead + Unpin> AsyncRead for EndOnEof<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buffer.filled().len();
        // Only a read that was *offered* room can report end of file by
        // filling none. A caller that polls with a full buffer gets a ready,
        // zero-filled read while the client is still very much there, and
        // treating that as the client going away kills a live session.
        let offered_room = buffer.remaining() > 0;
        let polled = Pin::new(&mut self.inner).poll_read(context, buffer);
        // A ready read that was offered room and filled none is end of file;
        // one that filled bytes is ordinary traffic, and an error is not an
        // ending this signal may claim.
        if offered_room && matches!(polled, Poll::Ready(Ok(()))) && buffer.filled().len() == before
        {
            self.ended.notify_one();
        }
        polled
    }
}

#[cfg(test)]
mod tests;
