//! [SEVERITY-CONFIG] One live configuration for pull and workspace publication.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use deslop_core::{
    live::{read_report_snapshot, report_for_file_in, ReportChangedNotification},
    report::Report,
};
use tokio::sync::{broadcast, Mutex};
use tower_lsp::{
    lsp_types::{Diagnostic, InitializeParams, MessageType, Url},
    Client,
};

use crate::{
    diagnostic_settings::{DiagnosticScope, DiagnosticSettings},
    diagnostics,
};

/// Session-owned settings and the files whose pushed diagnostics need clearing.
#[derive(Debug)]
pub(crate) struct DiagnosticSession {
    /// Last valid editor configuration.
    settings: RwLock<DiagnosticSettings>,
    /// Latest committed analysis snapshot.
    report: Arc<RwLock<Arc<Report>>>,
    /// Root used to resolve reported paths.
    root: PathBuf,
    /// Files to clear after findings disappear or scope changes.
    published: Mutex<BTreeSet<PathBuf>>,
    /// Whether the client accepts workspace diagnostic refresh requests.
    refresh_supported: AtomicBool,
}

impl DiagnosticSession {
    /// Uses the analysis snapshot without ever acquiring the analysis mutex.
    pub(crate) fn new(root: PathBuf, report: Arc<RwLock<Arc<Report>>>) -> Arc<Self> {
        Arc::new(Self {
            settings: RwLock::new(DiagnosticSettings::default()),
            report,
            root,
            published: Mutex::new(BTreeSet::new()),
            refresh_supported: AtomicBool::new(false),
        })
    }

    /// Validates initialization settings before accepting the connection.
    pub(crate) fn initialize(&self, params: &InitializeParams) -> Result<(), serde_json::Error> {
        let refresh = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.diagnostic.as_ref())
            .and_then(|diagnostics| diagnostics.refresh_support)
            .unwrap_or(false);
        self.refresh_supported.store(refresh, Ordering::Relaxed);
        if let Some(options) = &params.initialization_options {
            self.configure(options)?;
        }
        Ok(())
    }

    /// Invalid updates leave the previous valid configuration intact.
    pub(crate) fn configure(&self, value: &serde_json::Value) -> Result<(), serde_json::Error> {
        let settings = DiagnosticSettings::from_settings(value)?;
        tracing::debug!(enabled = settings.enabled, "diagnostic settings changed");
        *self
            .settings
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = settings;
        Ok(())
    }

    /// Copies configuration without holding the lock during publication.
    fn settings(&self) -> DiagnosticSettings {
        self.settings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Builds a pull response from exactly the same resolver as push.
    pub(crate) fn for_path(&self, path: &Path) -> Vec<Diagnostic> {
        let report = read_report_snapshot(&self.report);
        self.for_report_path(&report, path, &self.settings())
    }

    /// Renders a single file from a fixed report and settings snapshot.
    fn for_report_path(
        &self,
        report: &Report,
        path: &Path,
        settings: &DiagnosticSettings,
    ) -> Vec<Diagnostic> {
        diagnostics::build_for_file(&report_for_file_in(report, path), &self.root, settings)
    }

    /// Refreshes publication without running analysis ([SEVERITY-CONFIG]).
    pub(crate) async fn refresh(&self, client: &Client) {
        self.publish_workspace(client).await;
        if self.refresh_supported.load(Ordering::Relaxed) {
            if let Err(error) = client.workspace_diagnostic_refresh().await {
                tracing::warn!(%error, "diagnostic refresh request failed");
            }
        }
    }

    /// Publishes current files and clears files previously published.
    async fn publish_workspace(&self, client: &Client) {
        let mut published = self.published.lock().await;
        let settings = self.settings();
        let report = read_report_snapshot(&self.report);
        let current = workspace_paths(&report, &settings);
        for path in current.union(&published) {
            let items = if current.contains(path) {
                self.for_report_path(&report, path, &settings)
            } else {
                Vec::new()
            };
            self.publish_path(client, path, items).await;
        }
        *published = current;
    }

    /// Sends diagnostics at an absolute file URI.
    async fn publish_path(&self, client: &Client, path: &Path, items: Vec<Diagnostic>) {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        if let Ok(uri) = Url::from_file_path(absolute) {
            client.publish_diagnostics(uri, items, None).await;
        } else {
            tracing::error!("diagnostic file could not be represented as a URI");
        }
    }

    /// Applies a settings notification and refreshes both publication modes.
    pub(crate) async fn update(&self, client: &Client, settings: &serde_json::Value) {
        match self.configure(settings) {
            Ok(()) => self.refresh(client).await,
            Err(error) => {
                client
                    .show_message(
                        MessageType::ERROR,
                        format!("Invalid Deslop diagnostic settings: {error}"),
                    )
                    .await;
            }
        }
    }

    /// Every completed watcher/cache pass refreshes the diagnostic surfaces.
    pub(crate) fn watch(
        self: &Arc<Self>,
        client: Client,
        mut reports: broadcast::Receiver<ReportChangedNotification>,
    ) {
        let session = Arc::clone(self);
        let _task = tokio::spawn(async move {
            while let Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) = reports.recv().await {
                session.refresh(&client).await;
            }
        });
    }
}

/// Files eligible for the configured workspace publication scope.
fn workspace_paths(report: &Report, settings: &DiagnosticSettings) -> BTreeSet<PathBuf> {
    if !settings.enabled || settings.scope != DiagnosticScope::Workspace {
        return BTreeSet::new();
    }
    report
        .clusters
        .iter()
        .flat_map(|cluster| &cluster.occurrences)
        .filter(|occurrence| !occurrence.hidden)
        .map(|occurrence| occurrence.path.clone())
        .collect()
}
