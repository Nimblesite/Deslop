//! LSP server bootstrap — stdio transport and request normalisation
//! middleware ([LSP-TRANSPORT]).
//!
//! Extracted from `backend` to keep that module under the 500-line
//! budget while isolating all tower-lsp wiring in one place.

use std::{
    path::PathBuf,
    task::{Context, Poll},
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
];

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
/// # Errors
///
/// Returns `Err` when the backend fails to construct.
pub async fn run_stdio(
    workspace_root: PathBuf,
    min_nodes: u32,
    embedding: LspEmbeddingConfig,
) -> anyhow::Result<()> {
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
    .custom_method(
        custom_methods::VIRTUAL_DOCUMENT,
        custom_methods::virtual_document,
    )
    .finish();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    Server::new(stdin, stdout, socket)
        .serve(NormaliseParams::new(service))
        .await;
    Ok(())
}
