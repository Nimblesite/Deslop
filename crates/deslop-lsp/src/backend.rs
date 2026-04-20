//! `tower-lsp` backend wiring `Deslop` live analysis into the
//! Language Server Protocol ([LSP-CAPABILITIES]).
//!
//! Keeps the protocol surface narrow: `initialize`, `initialized`,
//! `shutdown`, `textDocument/didChange`, `textDocument/diagnostic`,
//! and the custom `deslop/*` namespace ([LSP-CUSTOM-METHODS]).

use std::{
    path::PathBuf,
    sync::Arc,
    task::{Context, Poll},
};

use deslop_core::{
    embedding::StubProvider,
    live::{AnalysisSession, LiveApi, LiveService},
};
use tokio::sync::Mutex;
use tower::Service;
use tower_lsp::{
    jsonrpc::{Request, Response, Result as LspResult},
    lsp_types::{
        CodeLens, CodeLensOptions, CodeLensParams, DiagnosticOptions, DiagnosticServerCapabilities,
        DidChangeTextDocumentParams, DocumentDiagnosticParams, DocumentDiagnosticReport,
        DocumentDiagnosticReportResult, FullDocumentDiagnosticReport, Hover, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
        MessageType, RelatedFullDocumentDiagnosticReport, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkDoneProgressOptions,
    },
    Client, ExitedError, LanguageServer, LspService, Server,
};

use crate::{code_lens, custom_methods, diagnostics, hover, position};

/// User-visible server name advertised in `initialize`.
pub const SERVER_NAME: &str = "deslop-lsp";

/// Diagnostic `source` + provider `identifier` surfaced to the client.
/// Must match the `source` field stamped by
/// [`crate::diagnostics::build_for_file`] so clients can filter by it.
pub const DIAGNOSTIC_SOURCE: &str = "deslop";

/// `tower-lsp` backend backed by a live [`LiveService`].
#[derive(Debug)]
pub struct LspBackend {
    /// LSP client handle for sending notifications back to the editor.
    client: Client,
    /// Shared live service.
    service: Arc<LiveService>,
}

impl LspBackend {
    /// Constructs a backend rooted at `workspace_root` using the stub
    /// embedding provider.
    ///
    /// # Errors
    ///
    /// Propagates [`deslop_core::live::LiveError`] when the
    /// underlying session cannot initialise.
    pub fn new_with_stub(
        client: Client,
        workspace_root: PathBuf,
        min_nodes: u32,
    ) -> Result<Self, deslop_core::live::LiveError> {
        let session = AnalysisSession::new(
            workspace_root,
            min_nodes,
            false,
            None,
            Arc::new(StubProvider::new()),
        )?;
        let service = Arc::new(LiveService::new(Arc::new(Mutex::new(session))));
        Ok(Self { client, service })
    }

    /// Returns the inner live service handle.
    #[must_use]
    pub fn service(&self) -> Arc<LiveService> {
        Arc::clone(&self.service)
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LspBackend {
    async fn initialize(&self, _params: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: SERVER_NAME.to_owned(),
                version: Some(deslop_core::version().to_owned()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some(DIAGNOSTIC_SOURCE.to_owned()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "deslop-lsp initialised")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(path) = url_to_path(&params.text_document.uri) else {
            return;
        };
        let session = self.service.session();
        let mut guard = session.lock().await;
        let _outcome = guard.apply_changes(&[path]);
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> LspResult<DocumentDiagnosticReportResult> {
        let path = url_to_path(&params.text_document.uri).unwrap_or_default();
        let file_report = self.service.report_for_file(&path).await;
        let global_weights = self.service.all_cluster_weights().await;
        let workspace_root = self.service.session_config().await.workspace_root;
        let items = diagnostics::build_for_file(&file_report, &global_weights, &workspace_root);
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            }),
        ))
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let Some(path) = url_to_path(&params.text_document_position_params.text_document.uri)
        else {
            return Ok(None);
        };
        let Ok(source) = std::fs::read_to_string(&path) else {
            return Ok(None);
        };
        let lsp_position = params.text_document_position_params.position;
        let byte = position::byte_for_position(&source, lsp_position);
        let clusters = self
            .service
            .report_for_range(&path, byte, byte.saturating_add(1))
            .await;
        let Some(cluster) = clusters.into_iter().next() else {
            return Ok(None);
        };
        Ok(Some(hover::build_for_cluster(&cluster)))
    }

    async fn code_lens(&self, params: CodeLensParams) -> LspResult<Option<Vec<CodeLens>>> {
        let Some(path) = url_to_path(&params.text_document.uri) else {
            return Ok(None);
        };
        let file_report = self.service.report_for_file(&path).await;
        Ok(Some(code_lens::build_for_file(&file_report)))
    }
}

/// Translates an LSP `Url` into a filesystem path.
#[must_use]
pub fn url_to_path(url: &Url) -> Option<PathBuf> {
    url.to_file_path().ok()
}

/// Boots the LSP server over stdio. Used by the binary entry point
/// and by E2E tests that drive the binary as a black box.
///
/// # Errors
///
/// Returns `Err` when the backend fails to construct.
pub async fn run_stdio(workspace_root: PathBuf, min_nodes: u32) -> anyhow::Result<()> {
    tracing::info!(
        workspace_root = %workspace_root.display(),
        exists = workspace_root.exists(),
        is_dir = workspace_root.is_dir(),
        min_nodes,
        "run_stdio booting backend",
    );
    let workspace_root_for_builder = workspace_root;
    let (service, socket) = LspService::build(move |client| {
        match LspBackend::new_with_stub(client, workspace_root_for_builder.clone(), min_nodes) {
            Ok(backend) => backend,
            Err(error) => report_init_failure(&error),
        }
    })
    .custom_method(custom_methods::REPORT_GET, custom_methods::report_get)
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
    .finish();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    Server::new(stdin, stdout, socket)
        .serve(NormaliseParams::new(service))
        .await;
    Ok(())
}

/// Methods that accept an empty-object `params` payload and must also
/// accept a missing `params` field — some JSON-RPC clients omit it for
/// no-arg calls, and tower-lsp's router rejects the request with
/// `-32602 Missing params field` before the handler runs.
const NO_PARAM_METHODS: &[&str] = &[
    custom_methods::REPORT_GET,
    custom_methods::LIST_MODELS,
    custom_methods::SESSION_CONFIG,
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
