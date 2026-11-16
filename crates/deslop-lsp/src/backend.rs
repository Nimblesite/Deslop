//! LSP backend wiring `Deslop` into LSP ([LSP-CAPABILITIES]).
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use deslop_core::{
    config::ConfigBaseline,
    live::{AnalysisState, EmbeddingProgress, EmbeddingProgressReporter, LiveService},
    report::Report,
    OpenBuffers,
};
use lsp_types::{MessageType, Uri};
use tokio::sync::Mutex;

use crate::{
    client::Client, diagnostic_session::DiagnosticSession,
    notifications::EmbeddingProgressNotification, observability::Observability,
};

/// User-visible server name advertised in `initialize`.
pub const SERVER_NAME: &str = "deslop-lsp";

/// Diagnostic `source` + provider `identifier` surfaced to clients.
pub const DIAGNOSTIC_SOURCE: &str = "deslop";

#[path = "backend_changes.rs"]
mod changes;
#[path = "backend_embedding.rs"]
mod embedding;
#[path = "backend_handlers.rs"]
mod handlers;
use embedding::resolve_startup_provider;
pub use embedding::LspEmbeddingConfig;

/// LSP backend backed by a live [`LiveService`].
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
    /// `None` when binding failed; the editor receives `ipc_startup_error`.
    _ipc: Option<crate::ipc::IpcServer>,
    /// [LSP-IPC] Retained until the editor can receive startup diagnostics.
    ipc_startup_error: Option<String>,
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
    /// Lock-free open buffers that report analysed ([LIVE-OPEN-BUFFERS]).
    analysed_buffers: Arc<RwLock<Arc<OpenBuffers>>>,
    /// Retains the cold pass state for `initialized()`, including refresh errors
    /// that predate the editor's notification handlers ([LIVE-CACHE-SEED-READINESS]).
    startup_state: Arc<Mutex<AnalysisState>>,
    /// Shared pull/push diagnostic configuration and publication state.
    diagnostics: Arc<DiagnosticSession>,
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
        // [LIVE-CONFIG-STARTUP] Before the session reads the config, so an
        // edit during the cold scan is replayed once the watcher attaches.
        let config_baseline = ConfigBaseline::capture(&workspace_root, None);
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
        let analysed_buffers = session.analysed_buffers_handle();
        let exclusion = session.exclusion_handle();
        let session = Arc::new(Mutex::new(session));
        let service = Arc::new(LiveService::new(Arc::clone(&session)));
        let (watcher, scheduler) = crate::file_watch::start(
            &root,
            &config_baseline,
            Arc::clone(&session),
            exclusion,
            client.clone(),
        )?;
        let report_changed = scheduler.report_changed_sender();
        let diagnostics = DiagnosticSession::new(root.clone(), Arc::clone(&report_snapshot));
        diagnostics.watch(client.clone(), scheduler.subscribe_report_changed());
        let ipc = crate::ipc::IpcServer::start(
            &root,
            ipc_mode,
            Arc::clone(&service),
            report_changed.clone(),
        )
        .inspect_err(|error| tracing::warn!(%error, "ipc_socket_start_failed"));
        let ipc_startup_error = ipc.as_ref().err().map(ToString::to_string);
        // A seeded session serves a cached report while a background cold
        // pass runs; a fresh session has already finished its blocking
        // scan. `initialized()` reads this to report the right startup
        // state to the editor ([VSIX reactivity]).
        let startup_state = crate::cache_seed::new_startup_state(seeded_from_cache);
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
                startup_state: Arc::clone(&startup_state),
            });
        }
        Ok(Self {
            client,
            service,
            _watcher: watcher,
            _scheduler: scheduler,
            _ipc: ipc.ok(),
            ipc_startup_error,
            observability,
            workspace_root: root,
            report_snapshot,
            analysed_buffers,
            startup_state,
            diagnostics,
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

    /// [LSP-IPC] Explain an unavailable MCP endpoint once handlers are installed.
    async fn report_ipc_startup_error(&self) {
        if let Some(error) = &self.ipc_startup_error {
            self.client
                .show_message(
                    MessageType::ERROR,
                    format!(
                        "Deslop MCP unavailable for {}: {error}",
                        self.workspace_root.display()
                    ),
                )
                .await;
        }
    }
}

/// Translates an LSP `file://` URI into a filesystem path.
#[must_use]
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    lspkit_server::uri_to_path(uri.as_str()).ok()
}

/// Translates an absolute filesystem path into an LSP `file://` URI.
#[must_use]
pub fn path_to_uri(path: &Path) -> Option<Uri> {
    lspkit_server::path_to_uri(path).ok()?.parse().ok()
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
