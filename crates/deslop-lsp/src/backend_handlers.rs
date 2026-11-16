//! The base-protocol handlers of [`LspBackend`] ([LSP-CAPABILITIES]).

use deslop_core::live::{read_analysed_buffers, read_report_snapshot, report_for_file_in, LiveApi};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionParams, CodeActionProviderCapability,
    CodeActionResponse, CodeLens, CodeLensOptions, CodeLensParams, DiagnosticOptions,
    DiagnosticServerCapabilities, DidChangeConfigurationParams, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    ExecuteCommandParams, FullDocumentDiagnosticReport, InitializeParams, InitializeResult,
    MessageType, RelatedFullDocumentDiagnosticReport, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, WorkDoneProgressOptions,
};

use super::{uri_to_path, LspBackend, DIAGNOSTIC_SOURCE, SERVER_NAME};
use crate::{
    code_action, code_lens, commands,
    jsonrpc::{invalid_params, LspResult},
};

impl LspBackend {
    /// `initialize`: records the client and advertises the additive surfaces.
    ///
    /// # Errors
    ///
    /// Returns `Invalid params` when the client settings cannot be read.
    pub(crate) fn initialize(&self, params: &InitializeParams) -> LspResult<InitializeResult> {
        self.diagnostics
            .initialize(params)
            .map_err(|error| invalid_params(error.to_string()))?;
        crate::parent_process::start_monitor(params.process_id);
        Ok(InitializeResult {
            // [DEPLOY-PROTOCOL-VERSION] serverInfo.version must equal --version.
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
                // [LSP-NON-INTERFERENCE] Deslop advertises ONLY additive
                // surfaces — clone diagnostics, code-lens badges, and its
                // own `deslop.*` commands. It deliberately does NOT
                // advertise `hoverProvider`, `definitionProvider`,
                // `referencesProvider`, `completionProvider`, or any other
                // standard language-intelligence capability, so it can
                // never intercept, suppress, or slow the editor's own
                // Go To Definition / Hover / etc.
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                // [AUTOFIX-EXTRACT-CODE-ACTION] + [AUTOFIX-MERGE-CODE-ACTION]
                // The verbatim extract carries its edit inline; the
                // mechanical merge is offered lazily and resolved on
                // demand, so resolveProvider is advertised.
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::REFACTOR_EXTRACT,
                            CodeActionKind::REFACTOR_REWRITE,
                        ]),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(true),
                    },
                )),
                execute_command_provider: Some(commands::provider()),
                ..ServerCapabilities::default()
            },
        })
    }

    /// `initialized`: pushes the state the editor could not be sent earlier.
    pub(crate) async fn initialized(&self) {
        self.report_ipc_startup_error().await;
        self.client
            .log_message(MessageType::INFO, "deslop-lsp initialised")
            .await;
        // Push the current analysis state now that the editor's
        // notification handlers are registered, so the panel reflects an
        // in-flight cold pass (or a settled scan) without a window reload
        // ([VSIX reactivity]).
        crate::cache_seed::push_initial_state(&self.client, &self.startup_state).await;
        // [CI-DESLOP]: the threshold is a CLI-only gate; its sole
        // live-surface effect is one non-blocking warning when the budget
        // is smashed. Nothing else in the editor changes.
        crate::threshold_warning::push_threshold_warning(&self.client, &self.service).await;
        self.diagnostics.refresh(&self.client).await;
    }

    /// `workspace/didChangeConfiguration`.
    pub(crate) async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.diagnostics
            .update(&self.client, &params.settings)
            .await;
    }

    /// `workspace/didChangeWatchedFiles`.
    pub(crate) async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        self.observability
            .record_handler("did_change_watched_files");
        let paths = crate::navigation::paths_from_file_events(&params.changes);
        self.apply_changed_paths(&paths).await;
    }

    // [LIVE-OPEN-BUFFERS] From `didOpen` until `didClose` the editor's
    // text, not the file on disk, is what the session analyses for this
    // path. Every edit is applied to that text and re-analysed at once,
    // so the report never claims a clone the user has already deleted in
    // an unsaved buffer.
    /// `textDocument/didOpen`.
    pub(crate) async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.observability.record_handler("did_open");
        let Some(path) = uri_to_path(&params.text_document.uri) else {
            return;
        };
        let text = params.text_document.text;
        let version = params.text_document.version;
        self.apply_buffer_change(&path, version, |session| session.open_buffer(&path, text))
            .await;
    }

    /// `textDocument/didChange`.
    pub(crate) async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.observability.record_handler("did_change");
        let Some(path) = uri_to_path(&params.text_document.uri) else {
            return;
        };
        let edits = crate::position::buffer_edits(params.content_changes);
        let version = params.text_document.version;
        self.apply_buffer_change(&path, version, |session| session.edit_buffer(&path, &edits))
            .await;
    }

    /// `textDocument/didClose`.
    pub(crate) async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.observability.record_handler("did_close");
        let Some(path) = uri_to_path(&params.text_document.uri) else {
            return;
        };
        self.apply_session_change(|session| session.close_buffer(&path), None)
            .await;
    }

    /// `textDocument/diagnostic`.
    pub(crate) fn diagnostic(
        &self,
        params: &DocumentDiagnosticParams,
    ) -> DocumentDiagnosticReportResult {
        self.observability.record_handler("diagnostic");
        let path = uri_to_path(&params.text_document.uri).unwrap_or_default();
        // [LSP-NON-INTERFERENCE-NONBLOCKING] Read the lock-free snapshot —
        // never the session mutex — so clone diagnostics can never block
        // behind an in-flight analysis pass.
        let items = self.diagnostics.for_path(&path);
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            },
        ))
    }

    // [LSP-NON-INTERFERENCE] [LSP-HOVER] No `hover` or `goto_definition`
    // handler: Deslop never answers `textDocument/hover` or
    // `textDocument/definition`. Those belong to the editor's real
    // language server. The clone card is surfaced by the VSIX's own
    // additive `ClusterHoverProvider`; canonical navigation is offered
    // by the `deslop.jumpToNextOccurrence` code lens, the
    // `deslop.compareWithCanonical` command, and the Top Offenders tree.

    /// `textDocument/codeAction`.
    pub(crate) fn code_action(&self, params: &CodeActionParams) -> Option<CodeActionResponse> {
        self.observability.record_handler("code_action");
        let path = uri_to_path(&params.text_document.uri)?;
        // [LSP-NON-INTERFERENCE-NONBLOCKING] Lock-free snapshot read —
        // never the session mutex.
        let report = read_report_snapshot(&self.report_snapshot);
        // [AUTOFIX-EXTRACT-CODE-ACTION] [LIVE-OPEN-BUFFERS] The action is
        // computed against the text that report analysed: the open buffer
        // when the editor holds one, otherwise the file on disk.
        let source = read_analysed_buffers(&self.analysed_buffers)
            .read(&path)
            .ok()?;
        let actions = code_action::build_for_range(
            &report,
            &path,
            &params.text_document.uri,
            &source.bytes,
            params.range,
        );
        (!actions.is_empty()).then_some(actions)
    }

    /// `codeAction/resolve`.
    ///
    /// # Errors
    ///
    /// Never errors — fallible to match the request signature.
    pub(crate) async fn code_action_resolve(&self, action: CodeAction) -> LspResult<CodeAction> {
        self.observability.record_handler("code_action_resolve");
        // [AUTOFIX-MERGE-CODE-ACTION] step 2: only rewrite offers carry
        // a cluster id; anything else resolves to itself.
        let Some(cluster_id) = code_action::offered_cluster_id(&action) else {
            return Ok(action);
        };
        match self.service.merge_plan(&cluster_id).await {
            Ok(plan) => {
                let resolved = code_action::resolved_action(action, &plan);
                Ok(code_action::warn_if_refused(&self.client, resolved).await)
            }
            Err(error) => {
                tracing::warn!(%cluster_id, %error, "merge plan resolution failed");
                Ok(action)
            }
        }
    }

    /// `textDocument/codeLens`.
    pub(crate) fn code_lens(&self, params: &CodeLensParams) -> Option<Vec<CodeLens>> {
        let path = uri_to_path(&params.text_document.uri)?;
        // [LSP-NON-INTERFERENCE-NONBLOCKING] Lock-free snapshot read —
        // never the session mutex — so the additive badges never block the
        // editor.
        let report = read_report_snapshot(&self.report_snapshot);
        let file_report = report_for_file_in(&report, &path);
        Some(code_lens::build_for_file(
            &file_report,
            &report.metrics.total_includes,
        ))
    }

    /// `workspace/executeCommand`.
    ///
    /// # Errors
    ///
    /// Returns the command's fault ([LSP-COMMANDS]).
    pub(crate) async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> LspResult<Option<serde_json::Value>> {
        commands::execute(self, params).await
    }
}
