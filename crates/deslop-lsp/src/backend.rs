//! `tower-lsp` backend wiring `Deslop` live analysis into the
//! Language Server Protocol ([LSP-CAPABILITIES]).
//!
//! Keeps the protocol surface narrow: `initialize`, `initialized`,
//! `shutdown`, `textDocument/didChange`, `textDocument/diagnostic`,
//! and the custom `deslop/*` namespace ([LSP-CUSTOM-METHODS]).

use std::{path::PathBuf, sync::Arc};

use deslop_core::{
    embedding::{
        EmbeddingMode, EmbeddingProvider, StubProvider, DEFAULT_OLLAMA_ENDPOINT,
        DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID, STUB_PROVIDER_ID,
    },
    live::{
        AnalysisSession, ChangeSummary, EmbeddingProgress, EmbeddingProgressReporter, LiveApi,
        LiveError, LiveService, ReportChangedNotification,
    },
};
use tokio::sync::Mutex;
use tower_lsp::{
    jsonrpc::Result as LspResult,
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
    Client, LanguageServer,
};

use crate::{code_lens, diagnostics, hover, position};

/// User-visible server name advertised in `initialize`.
pub const SERVER_NAME: &str = "deslop-lsp";

/// Diagnostic `source` + provider `identifier` surfaced to the client.
/// Must match the `source` field stamped by
/// [`crate::diagnostics::build_for_file`] so clients can filter by it.
pub const DIAGNOSTIC_SOURCE: &str = "deslop";

/// Method name for the `deslop/embeddingProgress` custom notification
/// pushed around a model swap ([VSIX-SESSION-PROGRESS]).
pub const EMBEDDING_PROGRESS: &str = "deslop/embeddingProgress";

/// Method name for the `deslop/reportChanged` custom notification
/// pushed after an analysis generation changes.
pub const REPORT_CHANGED: &str = "deslop/reportChanged";

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
        DEFAULT_PROVIDER_ID => Ok(connect_ollama_or_fallback(embedding)),
        other => Err(deslop_core::live::LiveError::UnsupportedProvider {
            requested: other.to_owned(),
            registered: vec![STUB_PROVIDER_ID.to_owned(), DEFAULT_PROVIDER_ID.to_owned()],
        }),
    }
}

/// Connects Ollama, falling back to a stub via the shared core function.
/// Both `Auto` and `Required` survive — the LSP must never crash-loop
/// VS Code per issue #35. Log level differs: warn for Auto, error for Required.
fn connect_ollama_or_fallback(embedding: &LspEmbeddingConfig) -> Arc<dyn EmbeddingProvider> {
    deslop_core::embedding::connect_or_stub(
        embedding.mode,
        &embedding.endpoint,
        &embedding.model_id,
    )
}

/// `tower-lsp` backend backed by a live [`LiveService`].
///
/// The backend owns the [`LiveWatcher`] and [`Scheduler`] so that they
/// stay alive for the entire server session. Dropping either stops the
/// watcher loop and terminates background analysis.
#[derive(Debug)]
pub struct LspBackend {
    /// LSP client handle for sending notifications back to the editor.
    client: Client,
    /// Shared live service.
    service: Arc<LiveService>,
    /// Filesystem watcher kept alive here — dropping it stops the OS
    /// watch ([LIVE-WATCHER]).
    _watcher: deslop_core::live::LiveWatcher,
    /// Scheduler kept alive so its broadcast channels remain open
    /// ([LIVE-SCHEDULER]).
    _scheduler: deslop_core::live::Scheduler,
    /// IPC socket server that exposes `duplicates/findSimilar` and
    /// `embedding/listModels` to the MCP server ([LSP-IPC]).
    /// `None` when the socket could not be bound (non-fatal).
    _ipc: Option<crate::ipc::IpcServer>,
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
            AnalysisSession::new(workspace_root, min_nodes, true, None, provider)?
        } else {
            AnalysisSession::new_with_mode(
                workspace_root,
                min_nodes,
                true,
                None,
                provider,
                embedding.mode,
            )?
        };
        session.set_embedding_progress_reporter(Some(progress_reporter(&client)));
        // Capture the root before moving the session into the Arc.
        let root = session.root().to_path_buf();
        let session = Arc::new(Mutex::new(session));
        let service = Arc::new(LiveService::new(Arc::clone(&session)));
        let (watcher, scheduler) = crate::file_watch::start(&root, session, client.clone())?;
        let ipc = crate::ipc::IpcServer::start(&root, Arc::clone(&service))
            .map_err(|e| tracing::warn!(%e, "ipc_socket_start_failed"))
            .ok();
        Ok(Self {
            client,
            service,
            _watcher: watcher,
            _scheduler: scheduler,
            _ipc: ipc,
        })
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
        match self.changed_paths_notification(paths).await {
            Ok(Some(notification)) => {
                self.client
                    .send_notification::<ReportChangedLspNotification>(notification)
                    .await;
            }
            Ok(None) => {}
            Err(error) => tracing::error!(%error, "failed to apply changed paths"),
        }
    }

    /// Applies paths and converts a non-empty delta into an LSP notification.
    async fn changed_paths_notification(
        &self,
        paths: &[PathBuf],
    ) -> Result<Option<ReportChangedNotification>, LiveError> {
        let session = self.service.session();
        let (previous_generation, previous_report, delta) = {
            let mut guard = session.lock().await;
            let previous_generation = guard.generation();
            let previous_report = guard.report();
            let delta = guard.apply_changes(paths)?;
            (previous_generation, previous_report, delta)
        };
        self.service
            .remember_snapshot(previous_generation, previous_report)
            .await;
        if delta.is_empty() {
            return Ok(None);
        }
        Ok(Some(ReportChangedNotification {
            generation: delta.to_generation,
            summary: ChangeSummary::from_delta(&delta),
        }))
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
        let workspace_root = self.service.session_config().await.workspace_root;
        let items = diagnostics::build_for_file(&file_report, &workspace_root);
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

/// Type-only marker so `tower_lsp::Client::send_notification` can
/// dispatch `deslop/reportChanged`.
#[derive(Debug)]
pub enum ReportChangedLspNotification {}

impl tower_lsp::lsp_types::notification::Notification for ReportChangedLspNotification {
    type Params = ReportChangedNotification;
    const METHOD: &'static str = REPORT_CHANGED;
}

/// Method name for `deslop/analysisState` pushed by the scheduler
/// whenever a watcher-driven pass starts, finishes, or errors.
pub const ANALYSIS_STATE: &str = "deslop/analysisState";

/// Type-only marker so `tower_lsp::Client::send_notification` can
/// dispatch `deslop/analysisState`.
///
/// The payload is a plain JSON string (`"idle"`, `"running"`,
/// `"errored"`) matching the TypeScript `AnalysisState` union type in
/// the VSIX so the extension can do `state === "running"` directly.
#[derive(Debug)]
pub enum AnalysisStateLspNotification {}

impl tower_lsp::lsp_types::notification::Notification for AnalysisStateLspNotification {
    /// Plain string — VSIX checks `state === "running"` etc.
    type Params = String;
    const METHOD: &'static str = ANALYSIS_STATE;
}
