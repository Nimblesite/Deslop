//! `StateFileBackend` — [`McpBackend`] implementation that reads the
//! `.deslop-cache/live-report.json` file written by the LSP server.
//!
//! Implements [MCP-WHY-LIVE]: the MCP server no longer runs its own
//! analysis pipeline. It reads the single source of truth produced by
//! `deslop-lsp` and pushes change notifications whenever the file
//! changes on disk.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
};

use deslop_core::{report::ReportCluster, EmbeddingSpec, OllamaModelInfo, Report};
use notify::{recommended_watcher, EventHandler, RecursiveMode, Watcher as NotifyWatcher};
use tracing::{debug, warn};

use crate::{notify::push_report_changed, NotificationSender};

use super::{
    filters, BackendError, FindSimilarInput, FindSimilarOutput, McpBackend, SessionBackendConfig,
    SessionConfigSnapshot,
};

/// `McpBackend` implementation that reads the LSP-written state file.
///
/// Provides live clone data without running its own analysis pipeline.
/// The LSP server writes `.deslop-cache/live-report.json`; this backend
/// reads it on demand and watches for changes via `notify`.
pub struct StateFileBackend {
    /// Workspace root pinned at initialisation.
    root: PathBuf,
    /// Absolute path to the LSP-written state file.
    state_file: PathBuf,
    /// Monotonic generation counter, bumped on every reload.
    generation: Arc<AtomicU64>,
    /// Most-recently-loaded report. `None` until first successful load.
    cached: Arc<RwLock<Option<Arc<Report>>>>,
    /// Shared notification sender wired by [`McpServer::run`].
    sender: Arc<Mutex<Option<NotificationSender>>>,
}

impl std::fmt::Debug for StateFileBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StateFileBackend")
            .field("root", &self.root)
            .field("state_file", &self.state_file)
            .finish_non_exhaustive()
    }
}

impl StateFileBackend {
    /// Constructs the backend from `config`. Does not read the state
    /// file eagerly — the first tool call triggers the initial load.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::StateFileCorrupt`] only if the config
    /// path cannot be canonicalised; never on a missing LSP state file.
    pub fn initialise(config: SessionBackendConfig) -> Result<Self, BackendError> {
        let state_file = config.root.join(".deslop-cache").join("live-report.json");
        Ok(Self {
            root: config.root,
            state_file,
            generation: Arc::new(AtomicU64::new(0)),
            cached: Arc::new(RwLock::new(None)),
            sender: Arc::new(Mutex::new(None)),
        })
    }

    /// Returns the cached report, reloading from disk if the cache is
    /// empty.
    ///
    /// # Errors
    ///
    /// See [`Self::reload_cache`].
    fn cached_report(&self) -> Result<Arc<Report>, BackendError> {
        {
            let guard = self
                .cached
                .read()
                .map_err(|_| BackendError::MutexPoisoned)?;
            if let Some(report) = guard.as_ref() {
                return Ok(Arc::clone(report));
            }
        }
        self.reload_cache()
    }

    /// Reads and parses the LSP state file, updates the in-memory cache,
    /// and increments the generation counter.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::LspNotRunning`] when the file does not
    /// exist, and [`BackendError::StateFileCorrupt`] for all other I/O
    /// or parse failures.
    fn reload_cache(&self) -> Result<Arc<Report>, BackendError> {
        let bytes = std::fs::read(&self.state_file).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                BackendError::LspNotRunning
            } else {
                BackendError::StateFileCorrupt(err.to_string())
            }
        })?;
        let report: Report = serde_json::from_slice(&bytes)
            .map_err(|err| BackendError::StateFileCorrupt(err.to_string()))?;
        let shared = Arc::new(report);
        let mut guard = self
            .cached
            .write()
            .map_err(|_| BackendError::MutexPoisoned)?;
        *guard = Some(Arc::clone(&shared));
        let _ = self.generation.fetch_add(1, Ordering::Relaxed);
        Ok(shared)
    }

    /// Starts a background thread that watches the parent directory of
    /// the state file and calls [`reload_and_notify`] on any change.
    ///
    /// Silently returns if the watcher cannot be created — the backend
    /// still works by reloading on every tool call.
    fn spawn_file_watcher(&self) {
        let Some(watch_dir) = self.state_file.parent().map(Path::to_path_buf) else {
            return;
        };
        let state_file = self.state_file.clone();
        let cached = Arc::clone(&self.cached);
        let generation = Arc::clone(&self.generation);
        let sender = Arc::clone(&self.sender);
        let _thread = std::thread::spawn(move || {
            run_watcher(watch_dir, state_file, cached, generation, sender);
        });
    }
}

/// Derives the set of language ids present in a report by mapping
/// occurrence file extensions to known language ids.
fn languages_from_report(report: &Report) -> Vec<String> {
    let mut seen = BTreeSet::new();
    for cluster in &report.clusters {
        for occ in &cluster.occurrences {
            if let Some(lang) = extension_to_language(&occ.path) {
                let _inserted = seen.insert(lang);
            }
        }
    }
    seen.into_iter().map(str::to_owned).collect()
}

/// Maps a file extension to a canonical language id.
fn extension_to_language(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str())? {
        "cs" => Some("csharp"),
        "rs" => Some("rust"),
        "py" => Some("python"),
        _ => None,
    }
}

/// Entry point for the background watcher thread.
fn run_watcher(
    watch_dir: PathBuf,
    state_file: PathBuf,
    cached: Arc<RwLock<Option<Arc<Report>>>>,
    generation: Arc<AtomicU64>,
    sender: Arc<Mutex<Option<NotificationSender>>>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let handler = ChannelHandler { tx };
    let Ok(mut watcher) = recommended_watcher(handler) else {
        warn!("mcp_state_file_watcher_init_failed");
        return;
    };
    if watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .is_err()
    {
        warn!(dir = %watch_dir.display(), "mcp_state_file_watch_failed");
        return;
    }
    for _event in rx {
        reload_and_notify(&state_file, &cached, &generation, &sender);
    }
}

/// Reloads the state file and notifies the client if a sender is set.
fn reload_and_notify(
    state_file: &Path,
    cached: &RwLock<Option<Arc<Report>>>,
    generation: &AtomicU64,
    sender: &Mutex<Option<NotificationSender>>,
) {
    let bytes = match std::fs::read(state_file) {
        Ok(b) => b,
        Err(err) => {
            debug!(reason = %err, "mcp_state_file_reload_failed");
            return;
        }
    };
    let Ok(report) = serde_json::from_slice::<Report>(&bytes) else {
        warn!("mcp_state_file_parse_failed");
        return;
    };
    let shared = Arc::new(report);
    let Ok(mut guard) = cached.write() else {
        return;
    };
    *guard = Some(shared);
    let gen = generation.fetch_add(1, Ordering::Relaxed) + 1;
    let Ok(lock) = sender.lock() else {
        return;
    };
    if let Some(s) = lock.as_ref() {
        push_report_changed(s, gen);
    }
}

/// Bridges `notify` events into a std channel so the watcher thread
/// can block on `rx.recv()` without requiring an async runtime.
struct ChannelHandler {
    tx: std::sync::mpsc::Sender<()>,
}

impl EventHandler for ChannelHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        if event.is_ok() {
            let _ = self.tx.send(());
        }
    }
}

impl McpBackend for StateFileBackend {
    fn root(&self) -> &Path {
        &self.root
    }

    fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    fn report_get(&self) -> Result<Arc<Report>, BackendError> {
        self.cached_report()
    }

    fn report_for_file(&self, path: &Path) -> Result<Vec<ReportCluster>, BackendError> {
        let resolved = crate::safety::resolve_within_root(&self.root, path)?;
        let report = self.cached_report()?;
        Ok(filters::filter_clusters_by_path(
            &report, &resolved, &self.root,
        ))
    }

    fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Result<Vec<ReportCluster>, BackendError> {
        let resolved = crate::safety::resolve_within_root(&self.root, path)?;
        let report = self.cached_report()?;
        Ok(filters::filter_clusters_by_range(
            &report, &resolved, start_byte, end_byte, &self.root,
        ))
    }

    fn find_similar(
        &self,
        _input: FindSimilarInput<'_>,
        _top_n: usize,
    ) -> Result<FindSimilarOutput, BackendError> {
        Err(BackendError::LspNotRunning)
    }

    fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, BackendError> {
        let report = self.cached_report()?;
        report
            .clusters
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or_else(|| BackendError::UnknownCluster(id.to_owned()))
    }

    fn list_embedding_models(&self) -> Result<Vec<OllamaModelInfo>, BackendError> {
        Err(BackendError::LspNotRunning)
    }

    fn set_embedding_model(
        &self,
        _provider_id: &str,
        _model_id: &str,
        _endpoint: Option<&str>,
    ) -> Result<EmbeddingSpec, BackendError> {
        Err(BackendError::LspNotRunning)
    }

    fn session_config(&self) -> Result<SessionConfigSnapshot, BackendError> {
        let report = self.cached_report()?;
        let languages = languages_from_report(&report);
        Ok(SessionConfigSnapshot {
            root: self.root.clone(),
            min_nodes: report.min_nodes,
            languages,
            embedding_provenance: report.embedding_provenance.clone(),
            incremental: false,
            cumulative_cache_stats: report.cache_stats,
        })
    }

    fn mark_changed(&self, _paths: &[PathBuf]) -> Result<(), BackendError> {
        self.reload_cache().map(|_| ()).or_else(|err| {
            if matches!(err, BackendError::LspNotRunning) {
                Ok(())
            } else {
                Err(err)
            }
        })
    }

    fn set_notification_sender(&self, sender: NotificationSender) {
        let Ok(mut guard) = self.sender.lock() else {
            return;
        };
        *guard = Some(sender);
        drop(guard);
        self.spawn_file_watcher();
    }
}
