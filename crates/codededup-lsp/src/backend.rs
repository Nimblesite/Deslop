//! `tower-lsp` backend wiring `CodeDedup` live analysis into the
//! Language Server Protocol ([LSP-CAPABILITIES]).
//!
//! Keeps the protocol surface narrow: `initialize`, `initialized`,
//! `shutdown`, `textDocument/didChange`, `textDocument/diagnostic`,
//! and the custom `codededup/*` namespace ([LSP-CUSTOM-METHODS]).

use std::{path::PathBuf, sync::Arc};

use codededup_core::{
    embedding::StubProvider,
    live::{AnalysisSession, LiveApi, LiveService},
};
use tokio::sync::Mutex;
use tower_lsp::{
    jsonrpc::Result as LspResult,
    lsp_types::{
        DidChangeTextDocumentParams, DocumentDiagnosticParams, DocumentDiagnosticReport,
        DocumentDiagnosticReportResult, FullDocumentDiagnosticReport, InitializeParams,
        InitializeResult, InitializedParams, MessageType, RelatedFullDocumentDiagnosticReport,
        ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    },
    Client, LanguageServer, LspService, Server,
};

use crate::{custom_methods, diagnostics};

/// User-visible server name advertised in `initialize`.
pub const SERVER_NAME: &str = "codededup-lsp";

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
    /// Propagates [`codededup_core::live::LiveError`] when the
    /// underlying session cannot initialise.
    pub fn new_with_stub(
        client: Client,
        workspace_root: PathBuf,
        min_nodes: u32,
    ) -> Result<Self, codededup_core::live::LiveError> {
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
                version: Some(codededup_core::version().to_owned()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "codededup-lsp initialised")
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
        let items = diagnostics::build_for_file(&file_report);
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
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}

/// Aborts the process with a structured diagnostic when the backend
/// cannot construct. The editor surfaces this through the standard
/// "server crashed" UX.
fn report_init_failure(error: &codededup_core::live::LiveError) -> ! {
    tracing::error!(%error, "codededup-lsp backend failed to initialise");
    std::process::exit(1)
}
