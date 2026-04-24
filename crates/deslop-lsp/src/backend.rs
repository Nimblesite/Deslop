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
    embedding::{
        EmbeddingMode, EmbeddingProvider, OllamaProvider, StubProvider, DEFAULT_OLLAMA_ENDPOINT,
        DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
    },
    live::{AnalysisSession, EmbeddingProgress, EmbeddingProgressReporter, LiveApi, LiveService},
};
use tokio::sync::Mutex;
use tower::Service;
use tower_lsp::{
    jsonrpc::{Request, Response, Result as LspResult},
    lsp_types::{
        CodeLens, CodeLensOptions, CodeLensParams, DiagnosticOptions, DiagnosticServerCapabilities,
        DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
        DocumentDiagnosticReport, DocumentDiagnosticReportResult, FileEvent,
        FullDocumentDiagnosticReport, GotoDefinitionParams, GotoDefinitionResponse, Hover,
        HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
        InitializedParams, Location, MessageType, OneOf, Range,
        RelatedFullDocumentDiagnosticReport, ServerCapabilities, ServerInfo,
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

/// Method name for the `deslop/embeddingProgress` custom notification
/// pushed around a model swap ([VSIX-SESSION-PROGRESS]).
pub const EMBEDDING_PROGRESS: &str = "deslop/embeddingProgress";

/// [LSP-EMBEDDING-CONSENT] Embedding startup settings supplied by the client after the user
/// has explicitly selected a model. `Off` means no startup embedding
/// pass runs.
#[derive(Debug, Clone)]
pub struct LspEmbeddingConfig {
    /// Live embedding mode.
    pub mode: EmbeddingMode,
    /// Provider registry key.
    pub provider_id: String,
    /// Model id.
    pub model_id: String,
    /// Provider endpoint.
    pub endpoint: String,
}

impl Default for LspEmbeddingConfig {
    fn default() -> Self {
        Self {
            mode: EmbeddingMode::Off,
            provider_id: DEFAULT_PROVIDER_ID.to_owned(),
            model_id: DEFAULT_OLLAMA_MODEL.to_owned(),
            endpoint: DEFAULT_OLLAMA_ENDPOINT.to_owned(),
        }
    }
}

/// Builds the provider used when the LSP backend starts. For
/// `EmbeddingMode::Auto`, unreachable Ollama endpoints fall back to
/// [`StubProvider`] with a warning log so the editor keeps working
/// — embeddings are optional per issue #35. `EmbeddingMode::Required`
/// preserves hard-fail semantics because the user explicitly opted
/// into embeddings.
fn build_startup_provider(
    embedding: &LspEmbeddingConfig,
) -> Result<Arc<dyn EmbeddingProvider>, deslop_core::live::LiveError> {
    if matches!(embedding.mode, EmbeddingMode::Off) {
        return Ok(Arc::new(StubProvider::new()));
    }
    match embedding.provider_id.as_str() {
        STUB_PROVIDER_ID => Ok(Arc::new(StubProvider::new())),
        DEFAULT_PROVIDER_ID => connect_ollama_or_fallback(embedding),
        other => Err(deslop_core::live::LiveError::UnsupportedProvider {
            requested: other.to_owned(),
            registered: vec![STUB_PROVIDER_ID.to_owned(), DEFAULT_PROVIDER_ID.to_owned()],
        }),
    }
}

/// Connects Ollama with auto-mode fallback. On `Auto` + provider
/// unreachable we log a warning and return [`StubProvider`] so the
/// LSP keeps answering requests. On `Required` we propagate the
/// error so the editor surfaces "start ollama and retry".
fn connect_ollama_or_fallback(
    embedding: &LspEmbeddingConfig,
) -> Result<Arc<dyn EmbeddingProvider>, deslop_core::live::LiveError> {
    match connect_ollama_provider(embedding) {
        Ok(provider) => Ok(provider),
        Err(error) if matches!(embedding.mode, EmbeddingMode::Auto) => {
            tracing::warn!(
                %error,
                endpoint = %embedding.endpoint,
                model = %embedding.model_id,
                "ollama embedding provider unreachable; falling back to stub so the LSP stays alive"
            );
            Ok(Arc::new(StubProvider::new()))
        }
        Err(error) => Err(error),
    }
}

/// Connects to Ollama and maps provider errors into live errors.
fn connect_ollama_provider(
    embedding: &LspEmbeddingConfig,
) -> Result<Arc<dyn EmbeddingProvider>, deslop_core::live::LiveError> {
    let provider =
        OllamaProvider::connect(&embedding.endpoint, &embedding.model_id).map_err(|err| {
            deslop_core::live::LiveError::ProviderUnreachable {
                endpoint: embedding.endpoint.clone(),
                message: err.to_string(),
            }
        })?;
    Ok(Arc::new(provider))
}

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
        let embedding = LspEmbeddingConfig::default();
        Self::new_with_config(client, workspace_root, min_nodes, &embedding)
    }

    /// Constructs a backend with explicit embedding startup config.
    ///
    /// # Errors
    ///
    /// Propagates live-session startup or selected-provider errors.
    pub fn new_with_config(
        client: Client,
        workspace_root: PathBuf,
        min_nodes: u32,
        embedding: &LspEmbeddingConfig,
    ) -> Result<Self, deslop_core::live::LiveError> {
        let provider = build_startup_provider(embedding)?;
        let mut session = if matches!(embedding.mode, EmbeddingMode::Off) {
            AnalysisSession::new(workspace_root, min_nodes, false, None, provider)?
        } else {
            AnalysisSession::new_with_mode(
                workspace_root,
                min_nodes,
                false,
                None,
                provider,
                embedding.mode,
            )?
        };
        session.set_embedding_progress_reporter(Some(progress_reporter(&client)));
        let service = Arc::new(LiveService::new(Arc::new(Mutex::new(session))));
        Ok(Self { client, service })
    }

    /// Returns the LSP client handle. Exposed so request handlers can
    /// push notifications alongside their response.
    #[must_use]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the inner live service handle.
    #[must_use]
    pub fn service(&self) -> Arc<LiveService> {
        Arc::clone(&self.service)
    }

    /// Re-runs analysis for changed paths when VS Code sends file
    /// lifecycle notifications.
    async fn apply_changed_paths(&self, paths: &[PathBuf]) {
        if paths.is_empty() {
            return;
        }
        let session = self.service.session();
        let mut guard = session.lock().await;
        let _outcome = guard.apply_changes(paths);
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
                definition_provider: Some(OneOf::Left(true)),
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

    async fn did_change_configuration(&self, _params: DidChangeConfigurationParams) {}

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let paths = paths_from_file_events(&params.changes);
        self.apply_changed_paths(&paths).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Some(path) = url_to_path(&params.text_document.uri) {
            self.apply_changed_paths(&[path]).await;
        }
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {}

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(path) = url_to_path(&params.text_document.uri) else {
            return;
        };
        self.apply_changed_paths(&[path]).await;
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
        if clusters.is_empty() {
            return Ok(None);
        }
        let ranked = self.service.report_get().await;
        let workspace_root = self.service.session_config().await.workspace_root;
        Ok(hover::build_for_clusters_with_root(
            &clusters,
            &ranked.clusters,
            &workspace_root,
        ))
    }

    async fn code_lens(&self, params: CodeLensParams) -> LspResult<Option<Vec<CodeLens>>> {
        let Some(path) = url_to_path(&params.text_document.uri) else {
            return Ok(None);
        };
        let file_report = self.service.report_for_file(&path).await;
        Ok(Some(code_lens::build_for_file(&file_report)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let td_params = params.text_document_position_params;
        let Some(path) = url_to_path(&td_params.text_document.uri) else {
            return Ok(None);
        };
        let Ok(source) = std::fs::read_to_string(&path) else {
            return Ok(None);
        };
        let byte = position::byte_for_position(&source, td_params.position);
        let clusters = self
            .service
            .report_for_range(&path, byte, byte.saturating_add(1))
            .await;
        let Some(cluster) = clusters.into_iter().next() else {
            return Ok(None);
        };
        let workspace_root = self.service.session_config().await.workspace_root;
        let Some(canonical) = pick_canonical(&cluster.occurrences, &workspace_root, &path, byte)
        else {
            return Ok(None);
        };
        let absolute = absolute_path(&workspace_root, &canonical.path);
        let target_source = std::fs::read_to_string(&absolute).unwrap_or_default();
        let start = position::position_for_byte(&target_source, canonical.start_byte);
        let end = position::position_for_byte(&target_source, canonical.end_byte);
        let Ok(uri) = Url::from_file_path(&absolute) else {
            return Ok(None);
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: Range { start, end },
        })))
    }
}

/// Picks the occurrence the caller should jump to from a cursor at
/// `(cursor_path, cursor_byte)`. Prefers the first occurrence that is
/// NOT the one the cursor sits in; falls back to the first occurrence
/// overall when every member lives in the same byte range. Resolves
/// relative occurrence paths against `workspace_root` before comparing.
fn pick_canonical<'a>(
    occurrences: &'a [deslop_core::report::ReportOccurrence],
    workspace_root: &std::path::Path,
    cursor_path: &std::path::Path,
    cursor_byte: usize,
) -> Option<&'a deslop_core::report::ReportOccurrence> {
    occurrences
        .iter()
        .find(|occurrence| {
            let absolute = absolute_path(workspace_root, &occurrence.path);
            !(absolute == cursor_path
                && occurrence.start_byte <= cursor_byte
                && cursor_byte < occurrence.end_byte)
        })
        .or_else(|| occurrences.first())
}

/// Joins `path` onto `workspace_root` when `path` is relative. Returns
/// `path` unchanged when it is already absolute.
fn absolute_path(workspace_root: &std::path::Path, path: &std::path::Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

/// Translates an LSP `Url` into a filesystem path.
#[must_use]
pub fn url_to_path(url: &Url) -> Option<PathBuf> {
    url.to_file_path().ok()
}

/// Extracts filesystem paths from watched-file events.
fn paths_from_file_events(events: &[FileEvent]) -> Vec<PathBuf> {
    events
        .iter()
        .filter_map(|event| url_to_path(&event.uri))
        .collect()
}

/// Builds the progress callback that emits LSP notifications.
fn progress_reporter(client: &Client) -> EmbeddingProgressReporter {
    let client = client.clone();
    Arc::new(move |event: EmbeddingProgress| {
        let client = client.clone();
        let _join = tokio::spawn(async move {
            client
                .send_notification::<EmbeddingProgressNotification>(event)
                .await;
        });
    })
}

/// Type-only marker so `tower_lsp::Client::send_notification` can
/// dispatch our custom method.
#[derive(Debug)]
pub enum EmbeddingProgressNotification {}

impl tower_lsp::lsp_types::notification::Notification for EmbeddingProgressNotification {
    type Params = EmbeddingProgress;
    const METHOD: &'static str = EMBEDDING_PROGRESS;
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

/// Methods that accept an empty-object `params` payload and must also
/// accept a missing `params` field — some JSON-RPC clients omit it for
/// no-arg calls, and tower-lsp's router rejects the request with
/// `-32602 Missing params field` before the handler runs.
const NO_PARAM_METHODS: &[&str] = &[
    custom_methods::REPORT_GET,
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
