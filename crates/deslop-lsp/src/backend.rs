//! `tower-lsp` backend wiring `Deslop` into LSP ([LSP-CAPABILITIES]).
use deslop_core::{
    embedding::{
        EmbeddingMode, EmbeddingProvider, NoopProvider, ProviderRegistry, RegistryError,
        DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID,
    },
    live::{
        read_report_snapshot, report_for_file_in, ChangeSummary, EmbeddingProgress,
        EmbeddingProgressReporter, LiveApi, LiveError, LiveService, ReportChangedNotification,
    },
    report::Report,
};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, RwLock},
};
use tokio::sync::Mutex;
use tower_lsp::{
    jsonrpc::Result as LspResult,
    lsp_types::{
        CodeAction, CodeActionKind, CodeActionOptions, CodeActionParams,
        CodeActionProviderCapability, CodeActionResponse, CodeLens, CodeLensOptions,
        CodeLensParams, DiagnosticOptions, DiagnosticServerCapabilities,
        DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
        DocumentDiagnosticReport, DocumentDiagnosticReportResult, ExecuteCommandParams,
        FullDocumentDiagnosticReport, InitializeParams, InitializeResult, InitializedParams,
        MessageType, RelatedFullDocumentDiagnosticReport, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkDoneProgressOptions,
    },
    Client, LanguageServer,
};

use crate::notifications::{EmbeddingProgressNotification, ReportChangedLspNotification};
use crate::observability::Observability;
use crate::{code_action, code_lens, commands, diagnostics};

/// User-visible server name advertised in `initialize`.
pub const SERVER_NAME: &str = "deslop-lsp";

/// Diagnostic `source` + provider `identifier` surfaced to clients.
pub const DIAGNOSTIC_SOURCE: &str = "deslop";

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

/// Resolves the startup `(provider, mode)` pair for the LSP backend.
///
/// For `EmbeddingMode::Off` the LSP installs a [`NoopProvider`] and
/// keeps mode `Off` — embeddings stay disabled until the user picks a
/// model. For `Auto` / `Required` we ask the production
/// [`ProviderRegistry`] for the requested provider. When the provider
/// is unreachable, we install [`NoopProvider`] and downgrade the mode
/// to `Off` so the editor keeps working without semantic recall — the
/// LSP must never crash-loop VS Code per. The log level
/// reflects intent: `error` when the user opted into `Required` and
/// `warn` when `Auto` silently degraded.
fn resolve_startup_provider(
    embedding: &LspEmbeddingConfig,
) -> Result<(Arc<dyn EmbeddingProvider>, EmbeddingMode), LiveError> {
    if matches!(embedding.mode, EmbeddingMode::Off) {
        return Ok((Arc::new(NoopProvider::new()), EmbeddingMode::Off));
    }
    let registry = ProviderRegistry::production();
    match registry.build(
        &embedding.provider_id,
        &embedding.model_id,
        Some(&embedding.endpoint),
    ) {
        Ok(provider) => Ok((provider, embedding.mode)),
        Err(RegistryError::Unsupported {
            requested,
            registered,
        }) => Err(LiveError::UnsupportedProvider {
            requested,
            registered,
        }),
        Err(RegistryError::Provider(provider_error)) => {
            log_provider_unreachable(embedding, &provider_error);
            Ok((Arc::new(NoopProvider::new()), EmbeddingMode::Off))
        }
    }
}

/// Emits the appropriate log when the configured provider is not
/// reachable. `Required` users opted in explicitly so the failure is
/// surfaced at `error`; `Auto` users get a `warn`.
fn log_provider_unreachable(
    embedding: &LspEmbeddingConfig,
    error: &deslop_core::embedding::ProviderError,
) {
    if matches!(embedding.mode, EmbeddingMode::Required) {
        tracing::error!(
            %error,
            endpoint = %embedding.endpoint,
            model = %embedding.model_id,
            "lsp_embedding_required_provider_unreachable",
        );
    } else {
        tracing::warn!(
            %error,
            endpoint = %embedding.endpoint,
            model = %embedding.model_id,
            "lsp_embedding_auto_provider_unreachable",
        );
    }
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
    /// CPU/work observability recorder for `deslop/cpuReport`.
    observability: Observability,
    /// Workspace root pinned at construction. Cached here so the
    /// additive editor reads (`diagnostic`, `code_lens`) never call
    /// `session_config()` — which would await the session mutex
    /// ([LSP-NON-INTERFERENCE]).
    workspace_root: PathBuf,
    /// Lock-free handle to the latest report. The additive editor reads
    /// answer from this snapshot without ever awaiting the session mutex
    /// held by an in-flight `apply_changes` pass, so Deslop can never
    /// freeze the editor ([LSP-NON-INTERFERENCE]).
    report_snapshot: Arc<RwLock<Arc<Report>>>,
    /// True while the cache-seed cold pass is still running. Read in
    /// `initialized()` to push the correct startup analysis state to a
    /// late-connecting editor, closing the race where the cold pass's
    /// `running`/`idle` broadcasts predate the VSIX notification
    /// handlers ([VSIX reactivity]).
    cold_pass_active: Arc<AtomicBool>,
}

impl LspBackend {
    /// Constructs a backend rooted at `workspace_root` with embeddings
    /// disabled. Used by callers that have not yet wired the
    /// embedding-config plumbing through the editor surface.
    ///
    /// # Errors
    ///
    /// Propagates [`deslop_core::live::LiveError`] when the
    /// underlying session cannot initialise.
    pub fn new_with_defaults(
        client: Client,
        workspace_root: PathBuf,
        min_nodes: u32,
    ) -> Result<Self, deslop_core::live::LiveError> {
        let embedding = LspEmbeddingConfig::default();
        Self::new_with_config(
            client,
            workspace_root,
            min_nodes,
            &embedding,
            deslop_core::live::transport::IpcMode::platform_default(),
        )
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
        ipc_mode: deslop_core::live::transport::IpcMode,
    ) -> Result<Self, deslop_core::live::LiveError> {
        let (provider, resolved_mode) = resolve_startup_provider(embedding)?;
        let observability = Observability::default();
        let (mut session, seeded_from_cache) = crate::cache_seed::open_session(
            workspace_root,
            min_nodes,
            true,
            None,
            Arc::clone(&provider),
            resolved_mode,
        )?;
        session.set_embedding_progress_reporter(Some(progress_reporter(&client)));
        // Capture the root and the lock-free report-snapshot handle
        // before moving the session into the Arc — the additive editor
        // reads use both without awaiting the session mutex
        // ([LSP-NON-INTERFERENCE]).
        let root = session.root().to_path_buf();
        let report_snapshot = session.report_snapshot_handle();
        let exclusion = session.exclusion_handle();
        let session = Arc::new(Mutex::new(session));
        let service = Arc::new(LiveService::new(Arc::clone(&session)));
        let (watcher, scheduler) =
            crate::file_watch::start(&root, None, Arc::clone(&session), exclusion, client.clone())?;
        let report_changed = scheduler.report_changed_sender();
        let ipc = crate::ipc::IpcServer::start(
            &root,
            ipc_mode,
            Arc::clone(&service),
            report_changed.clone(),
        )
        .map_err(|e| tracing::warn!(%e, "ipc_socket_start_failed"))
        .ok();
        // A seeded session serves a cached report while a background cold
        // pass runs; a fresh session has already finished its blocking
        // scan. `initialized()` reads this to report the right startup
        // state to the editor ([VSIX reactivity]).
        let cold_pass_active = Arc::new(AtomicBool::new(seeded_from_cache));
        if seeded_from_cache {
            crate::cache_seed::spawn_refresh(crate::cache_seed::RefreshTask {
                session: Arc::clone(&session),
                service: Arc::clone(&service),
                client: client.clone(),
                root: root.clone(),
                min_nodes,
                incremental: true,
                config_path: None,
                provider,
                mode: resolved_mode,
                report_changed,
                cold_pass_active: Arc::clone(&cold_pass_active),
            });
        }
        Ok(Self {
            client,
            service,
            _watcher: watcher,
            _scheduler: scheduler,
            _ipc: ipc,
            observability,
            workspace_root: root,
            report_snapshot,
            cold_pass_active,
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

    /// Returns the CPU/work observability recorder.
    #[must_use]
    pub fn observability(&self) -> &Observability {
        &self.observability
    }

    /// Returns the workspace root that occurrence paths resolve against.
    /// Command handlers pass it to the HTML renderer as the snippet
    /// scan root.
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
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
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
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

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "deslop-lsp initialised")
            .await;
        // Push the current analysis state now that the editor's
        // notification handlers are registered, so the panel reflects an
        // in-flight cold pass (or a settled scan) without a window reload
        // ([VSIX reactivity]).
        crate::cache_seed::push_initial_state(&self.client, &self.cold_pass_active).await;
        // [CI-DESLOP]: the threshold is a CLI-only gate; its sole
        // live-surface effect is one non-blocking warning when the budget
        // is smashed. Nothing else in the editor changes.
        crate::threshold_warning::push_threshold_warning(&self.client, &self.service).await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_change_configuration(&self, _params: DidChangeConfigurationParams) {}

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        self.observability
            .record_handler("did_change_watched_files");
        let paths = crate::navigation::paths_from_file_events(&params.changes);
        self.apply_changed_paths(&paths).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.observability.record_handler("did_open");
        if let Some(path) = url_to_path(&params.text_document.uri) {
            self.apply_changed_paths(&[path]).await;
        }
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {}

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.observability.record_handler("did_change");
        let Some(path) = url_to_path(&params.text_document.uri) else {
            return;
        };
        self.apply_changed_paths(&[path]).await;
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> LspResult<DocumentDiagnosticReportResult> {
        self.observability.record_handler("diagnostic");
        let path = url_to_path(&params.text_document.uri).unwrap_or_default();
        // [LSP-NON-INTERFERENCE-NONBLOCKING] Read the lock-free snapshot —
        // never the session mutex — so clone diagnostics can never block
        // behind an in-flight analysis pass.
        let report = read_report_snapshot(&self.report_snapshot);
        let file_report = report_for_file_in(&report, &path);
        let items = diagnostics::build_for_file(&file_report, &self.workspace_root);
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

    // [LSP-NON-INTERFERENCE] [LSP-HOVER] No `hover` or `goto_definition`
    // handler: Deslop never answers `textDocument/hover` or
    // `textDocument/definition`. Those belong to the editor's real
    // language server. The clone card is surfaced by the VSIX's own
    // additive `ClusterHoverProvider`; canonical navigation is offered
    // by the `deslop.jumpToNextOccurrence` code lens, the
    // `deslop.compareWithCanonical` command, and the Top Offenders tree.

    async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
        self.observability.record_handler("code_action");
        let Some(path) = url_to_path(&params.text_document.uri) else {
            return Ok(None);
        };
        // [AUTOFIX-EXTRACT-CODE-ACTION] The action is computed against
        // the same on-disk state the watcher-driven report describes;
        // a stale buffer makes the editor reject the apply and the
        // action is re-offered on the next compute cycle
        // ([AUTOFIX-EXTRACT-WORKSPACE-EDIT]).
        let Ok(source) = std::fs::read(&path) else {
            return Ok(None);
        };
        // [LSP-NON-INTERFERENCE-NONBLOCKING] Lock-free snapshot read —
        // never the session mutex.
        let report = read_report_snapshot(&self.report_snapshot);
        let actions = code_action::build_for_range(
            &report,
            &path,
            &params.text_document.uri,
            &source,
            params.range,
        );
        Ok((!actions.is_empty()).then_some(actions))
    }

    async fn code_action_resolve(&self, action: CodeAction) -> LspResult<CodeAction> {
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

    async fn code_lens(&self, params: CodeLensParams) -> LspResult<Option<Vec<CodeLens>>> {
        let Some(path) = url_to_path(&params.text_document.uri) else {
            return Ok(None);
        };
        // [LSP-NON-INTERFERENCE-NONBLOCKING] Lock-free snapshot read —
        // never the session mutex — so the additive badges never block the
        // editor.
        let report = read_report_snapshot(&self.report_snapshot);
        let file_report = report_for_file_in(&report, &path);
        Ok(Some(code_lens::build_for_file(&file_report)))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> LspResult<Option<serde_json::Value>> {
        commands::execute(self, params).await
    }
}

/// Translates an LSP `Url` into a filesystem path.
#[must_use]
pub fn url_to_path(url: &Url) -> Option<PathBuf> {
    url.to_file_path().ok()
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
