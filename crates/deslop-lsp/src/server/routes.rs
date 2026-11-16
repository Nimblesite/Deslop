//! The methods Deslop answers, registered on the `lspkit-server` dispatcher
//! ([LSP-CAPABILITIES], [LSP-CUSTOM-METHODS]).

use std::{future::Future, sync::Arc};

use lsp_types::{
    notification::{
        DidChangeConfiguration, DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument,
        DidOpenTextDocument, Initialized, Notification,
    },
    request::{
        CodeActionRequest, CodeActionResolveRequest, CodeLensRequest, DocumentDiagnosticRequest,
        ExecuteCommand, Initialize, Request, Shutdown,
    },
};
use lspkit_server::{CancellationToken, Dispatcher, HandlerResult};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    backend::LspBackend,
    custom_methods as custom,
    jsonrpc::{internal_error, invalid_params, request_cancelled, LspResult},
};

/// Registers `$handler` for `$method`: params are read as the handler's
/// parameter type and a cancelled request stops waiting on its work. The
/// `read` form is for handlers that answer from a snapshot without waiting
/// and cannot fail; `open` is `initialize`, which can.
macro_rules! route {
    ($dispatcher:expr, $backend:expr, $method:expr, $handler:path) => {
        route!(@register $dispatcher, $backend, $method, |backend, params| {
            $handler(backend, parse(params)?).await
        })
    };
    (read $dispatcher:expr, $backend:expr, $method:expr, $handler:path) => {
        route!(@register $dispatcher, $backend, $method, |backend, params| {
            Ok($handler(backend, &parse(params)?))
        })
    };
    (open $dispatcher:expr, $backend:expr, $method:expr, $handler:path) => {
        route!(@register $dispatcher, $backend, $method, |backend, params| {
            $handler(backend, &parse(params)?)
        })
    };
    (@register $dispatcher:expr, $backend:expr, $method:expr, |$held:ident, $params:ident| $body:block) => {{
        let backend = Arc::clone($backend);
        $dispatcher.register($method, move |$params: Value, cancel: CancellationToken| {
            let backend = Arc::clone(&backend);
            async move {
                let $held: &LspBackend = &backend;
                answer(cancel, async $body).await
            }
        });
    }};
}

/// Requests answered from the published report snapshot. They never wait
/// for an analysis pass, so they do not wait for the edits read before them
/// either ([LSP-NON-INTERFERENCE-NONBLOCKING]).
const SNAPSHOT_READS: [&str; 3] = [
    DocumentDiagnosticRequest::METHOD,
    CodeActionRequest::METHOD,
    CodeLensRequest::METHOD,
];

/// Whether `method` is answered from the snapshot without waiting on the session.
pub(super) fn reads_snapshot(method: &str) -> bool {
    SNAPSHOT_READS.contains(&method)
}

/// Registers every request Deslop answers.
pub(super) fn register(dispatcher: &Dispatcher, backend: &Arc<LspBackend>) {
    register_standard(dispatcher, backend);
    register_reports(dispatcher, backend);
    register_session(dispatcher, backend);
}

/// The base-protocol requests ([LSP-CAPABILITIES]).
fn register_standard(dispatcher: &Dispatcher, backend: &Arc<LspBackend>) {
    use LspBackend as Backend;
    route!(open dispatcher, backend, Initialize::METHOD, Backend::initialize);
    route!(read dispatcher, backend, DocumentDiagnosticRequest::METHOD, Backend::diagnostic);
    route!(read dispatcher, backend, CodeActionRequest::METHOD, Backend::code_action);
    route!(read dispatcher, backend, CodeLensRequest::METHOD, Backend::code_lens);
    route!(
        dispatcher,
        backend,
        CodeActionResolveRequest::METHOD,
        Backend::code_action_resolve
    );
    route!(
        dispatcher,
        backend,
        ExecuteCommand::METHOD,
        Backend::execute_command
    );
    // `shutdown` has nothing to flush: admission already stopped new work.
    dispatcher.register(Shutdown::METHOD, |_params, _cancel| {
        std::future::ready(HandlerResult::Ok(Value::Null))
    });
}

/// The `deslop/*` report reads ([LSP-CUSTOM-METHODS]).
fn register_reports(dispatcher: &Dispatcher, backend: &Arc<LspBackend>) {
    route!(dispatcher, backend, custom::REPORT_GET, custom::report_get);
    route!(
        dispatcher,
        backend,
        custom::REPORT_DELTA,
        custom::report_delta
    );
    route!(
        dispatcher,
        backend,
        custom::REPORT_FOR_FILE,
        custom::report_for_file
    );
    route!(
        dispatcher,
        backend,
        custom::REPORT_FOR_RANGE,
        custom::report_for_range
    );
    route!(
        dispatcher,
        backend,
        custom::CLUSTER_BY_ID,
        custom::cluster_by_id
    );
    route!(
        dispatcher,
        backend,
        custom::PAIR_COMPARE,
        custom::pair_compare
    );
    route!(
        dispatcher,
        backend,
        custom::FIND_SIMILAR,
        custom::find_similar
    );
    route!(
        dispatcher,
        backend,
        custom::REPORT_SCHEMA_DOC,
        custom::report_schema_doc
    );
    route!(
        dispatcher,
        backend,
        custom::VIRTUAL_DOCUMENT,
        custom::virtual_document
    );
}

/// The `deslop/*` session controls ([LSP-CUSTOM-METHODS]).
fn register_session(dispatcher: &Dispatcher, backend: &Arc<LspBackend>) {
    route!(
        dispatcher,
        backend,
        custom::LIST_MODELS,
        custom::embedding_list_models
    );
    route!(
        dispatcher,
        backend,
        custom::SET_MODEL,
        custom::embedding_set_model
    );
    route!(
        dispatcher,
        backend,
        custom::SESSION_CONFIG,
        custom::session_config
    );
    route!(dispatcher, backend, custom::CPU_REPORT, custom::cpu_report);
}

/// Runs the handler for one notification; an unknown method is ignored, as
/// the base protocol requires.
pub(super) async fn notify(backend: &LspBackend, method: &str, params: Value) {
    match method {
        Initialized::METHOD => backend.initialized().await,
        DidChangeConfiguration::METHOD => {
            deliver(method, params, |p| backend.did_change_configuration(p)).await;
        }
        DidChangeWatchedFiles::METHOD => {
            deliver(method, params, |p| backend.did_change_watched_files(p)).await;
        }
        DidOpenTextDocument::METHOD => deliver(method, params, |p| backend.did_open(p)).await,
        DidChangeTextDocument::METHOD => deliver(method, params, |p| backend.did_change(p)).await,
        DidCloseTextDocument::METHOD => deliver(method, params, |p| backend.did_close(p)).await,
        _ => tracing::debug!(method, "unhandled notification ignored"),
    }
}

/// Reads a notification's params and hands them to `handler`; params that
/// do not fit are logged, since a notification has no reply to fail.
async fn deliver<P, F, Fut>(method: &str, params: Value, handler: F)
where
    P: DeserializeOwned,
    F: FnOnce(P) -> Fut,
    Fut: Future<Output = ()>,
{
    match serde_json::from_value(params) {
        Ok(params) => handler(params).await,
        Err(error) => tracing::warn!(method, %error, "notification params not understood"),
    }
}

/// Reads request params as the handler's parameter type.
fn parse<P: DeserializeOwned>(params: Value) -> LspResult<P> {
    serde_json::from_value(params).map_err(|error| invalid_params(error.to_string()))
}

/// Awaits `work` unless the request is cancelled first, and shapes the
/// outcome for the dispatcher.
pub(super) async fn answer<R, W>(cancel: CancellationToken, work: W) -> HandlerResult
where
    R: Serialize,
    W: Future<Output = LspResult<R>>,
{
    let outcome = tokio::select! {
        biased;
        () = cancel.cancelled() => Err(request_cancelled()),
        outcome = work => outcome,
    };
    let serialised = outcome
        .and_then(|value| serde_json::to_value(value).map_err(|e| internal_error(e.to_string())));
    match serialised {
        Ok(value) => HandlerResult::Ok(value),
        Err(error) => HandlerResult::Err(error),
    }
}
