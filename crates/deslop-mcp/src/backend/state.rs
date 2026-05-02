//! `StateFileBackend` — [`McpBackend`] implementation that reads the
//! `.deslop-cache/live-report.json` file written by the LSP server.
//!
//! Implements [MCP-WHY-LIVE]: the MCP server no longer runs its own
//! analysis pipeline. It reads the single source of truth produced by
//! `deslop-lsp` and pushes change notifications whenever the file
//! changes on disk.

use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::SystemTime,
};

use deslop_core::{
    live::wire::{
        EmbeddingModelInfo as WireEmbeddingModelInfo, FindSimilarInput as WireFindSimilarInput,
        FindSimilarRequest,
    },
    report::ReportCluster,
    EmbeddingSpec, OllamaModelInfo, Report,
};
use notify::{recommended_watcher, EventHandler, RecursiveMode, Watcher as NotifyWatcher};
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::{notify::push_report_changed, NotificationSender};

use super::{
    filters, ipc::ipc_call, BackendError, FindSimilarInput, FindSimilarOutput, McpBackend,
    SessionBackendConfig, SessionConfigSnapshot,
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
    /// Absolute path to the LSP IPC socket. Used to delegate
    /// `find-similar` and `list-embedding-models` to the running LSP
    /// when the state file alone is not enough ([MCP-IPC-CLIENT]).
    ipc_socket: PathBuf,
    /// Monotonic generation counter, bumped on every reload.
    generation: Arc<AtomicU64>,
    /// Most-recently-loaded report. `None` until first successful load.
    cached: Arc<RwLock<Option<Arc<Report>>>>,
    /// State-file fingerprint loaded with `cached`.
    cached_stamp: Arc<RwLock<Option<StateFileStamp>>>,
    /// Shared notification sender wired by [`McpServer::run`].
    sender: Arc<Mutex<Option<NotificationSender>>>,
}

/// Fingerprint of the LSP-written state file, used by the watcher to
/// skip re-reads when neither the modification time nor the length
/// changed.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StateFileStamp {
    /// Last-modified time of the state file at load time.
    modified: SystemTime,
    /// Byte length of the state file at load time.
    len: u64,
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
        let cache_dir = config.root.join(".deslop-cache");
        let state_file = cache_dir.join("live-report.json");
        let ipc_socket = cache_dir.join("deslop.sock");
        Ok(Self {
            root: config.root,
            state_file,
            ipc_socket,
            generation: Arc::new(AtomicU64::new(0)),
            cached: Arc::new(RwLock::new(None)),
            cached_stamp: Arc::new(RwLock::new(None)),
            sender: Arc::new(Mutex::new(None)),
        })
    }

    /// Returns the cached report, reloading from disk when the state
    /// file changed since the cache was loaded.
    ///
    /// # Errors
    ///
    /// See [`Self::reload_cache`].
    fn cached_report(&self) -> Result<Arc<Report>, BackendError> {
        let stamp = state_file_stamp(&self.state_file)?;
        {
            let report_guard = self
                .cached
                .read()
                .map_err(|_| BackendError::MutexPoisoned)?;
            let stamp_guard = self
                .cached_stamp
                .read()
                .map_err(|_| BackendError::MutexPoisoned)?;
            if let (Some(report), Some(cached_stamp)) =
                (report_guard.as_ref(), stamp_guard.as_ref())
            {
                if cached_stamp == &stamp {
                    return Ok(Arc::clone(report));
                }
            }
        }
        self.reload_cache_with_stamp(stamp)
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
        let stamp = state_file_stamp(&self.state_file)?;
        self.reload_cache_with_stamp(stamp)
    }

    /// Variant of [`Self::reload_cache`] that takes a pre-computed
    /// [`StateFileStamp`] so the file watcher does not stat the file
    /// twice (once to detect the change, once to read it).
    fn reload_cache_with_stamp(&self, stamp: StateFileStamp) -> Result<Arc<Report>, BackendError> {
        let report = read_state_report(&self.state_file)?;
        let shared = Arc::new(report);
        let mut guard = self
            .cached
            .write()
            .map_err(|_| BackendError::MutexPoisoned)?;
        *guard = Some(Arc::clone(&shared));
        let mut stamp_guard = self
            .cached_stamp
            .write()
            .map_err(|_| BackendError::MutexPoisoned)?;
        *stamp_guard = Some(stamp);
        let _ = self.generation.fetch_add(1, Ordering::Relaxed);
        Ok(shared)
    }

    /// Asks the running LSP to execute `deslop.lsp.refreshReport`.
    ///
    /// Returns `Ok(false)` when the LSP socket is absent so MCP-only
    /// fixture tests and cache-only sessions keep the previous reload
    /// behaviour.
    fn request_lsp_refresh(&self) -> Result<bool, BackendError> {
        let result = match ipc_call(&self.ipc_socket, "deslop.lsp.refreshReport", &json!({})) {
            Ok(result) => result,
            Err(BackendError::LspNotRunning) => return Ok(false),
            Err(err) => return Err(err),
        };
        validate_refresh_result(&result)?;
        Ok(true)
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
        let cached_stamp = Arc::clone(&self.cached_stamp);
        let generation = Arc::clone(&self.generation);
        let sender = Arc::clone(&self.sender);
        let _thread = std::thread::spawn(move || {
            run_watcher(
                &watch_dir,
                &state_file,
                &cached,
                &cached_stamp,
                &generation,
                &sender,
            );
        });
    }
}

/// Returns the (mtime, length) stamp used to detect state-file changes.
fn state_file_stamp(state_file: &Path) -> Result<StateFileStamp, BackendError> {
    let metadata = fs::metadata(state_file).map_err(|err| map_state_file_io_error(&err))?;
    let modified = metadata
        .modified()
        .map_err(|err| BackendError::StateFileCorrupt(err.to_string()))?;
    Ok(StateFileStamp {
        modified,
        len: metadata.len(),
    })
}

/// Reads the LSP-written state file and parses it into a [`Report`].
fn read_state_report(state_file: &Path) -> Result<Report, BackendError> {
    let bytes = fs::read(state_file).map_err(|err| map_state_file_io_error(&err))?;
    serde_json::from_slice(&bytes).map_err(|err| BackendError::StateFileCorrupt(err.to_string()))
}

/// Lifts a state-file I/O error into a [`BackendError`], distinguishing
/// "LSP is not running" (file not found) from corruption.
fn map_state_file_io_error(err: &std::io::Error) -> BackendError {
    if err.kind() == ErrorKind::NotFound {
        BackendError::LspNotRunning
    } else {
        BackendError::StateFileCorrupt(err.to_string())
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

/// Maps the LSP IPC model shape into the MCP tool renderer's legacy
/// model row shape.
fn model_info_from_wire(model: WireEmbeddingModelInfo) -> OllamaModelInfo {
    OllamaModelInfo {
        name: model.model_id.clone(),
        bare_id: model.model_id,
        digest: model.model_version.unwrap_or_default(),
        size_bytes: 0,
        is_embedding_model: model.reachable && model.recommended,
    }
}

/// Entry point for the background watcher thread.
fn run_watcher(
    watch_dir: &Path,
    state_file: &Path,
    cached: &Arc<RwLock<Option<Arc<Report>>>>,
    cached_stamp: &Arc<RwLock<Option<StateFileStamp>>>,
    generation: &Arc<AtomicU64>,
    sender: &Arc<Mutex<Option<NotificationSender>>>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let handler = ChannelHandler { tx };
    let Ok(mut watcher) = recommended_watcher(handler) else {
        warn!("mcp_state_file_watcher_init_failed");
        return;
    };
    if watcher
        .watch(watch_dir, RecursiveMode::NonRecursive)
        .is_err()
    {
        warn!(dir = %watch_dir.display(), "mcp_state_file_watch_failed");
        return;
    }
    for _event in rx {
        reload_and_notify(state_file, cached, cached_stamp, generation, sender);
    }
}

/// Reloads the state file and notifies the client if a sender is set.
fn reload_and_notify(
    state_file: &Path,
    cached: &RwLock<Option<Arc<Report>>>,
    cached_stamp: &RwLock<Option<StateFileStamp>>,
    generation: &AtomicU64,
    sender: &Mutex<Option<NotificationSender>>,
) {
    let stamp = match state_file_stamp(state_file) {
        Ok(stamp) => stamp,
        Err(err) => {
            debug!(reason = %err, "mcp_state_file_reload_failed");
            return;
        }
    };
    let Ok(report) = read_state_report(state_file) else {
        warn!("mcp_state_file_parse_failed");
        return;
    };
    let shared = Arc::new(report);
    let Ok(mut guard) = cached.write() else {
        return;
    };
    *guard = Some(shared);
    drop(guard);
    let Ok(mut stamp_guard) = cached_stamp.write() else {
        return;
    };
    *stamp_guard = Some(stamp);
    let gen = generation.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
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
    /// Channel sender used to signal incoming filesystem events.
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
        input: FindSimilarInput<'_>,
        top_n: usize,
    ) -> Result<FindSimilarOutput, BackendError> {
        let wire_input = match input {
            FindSimilarInput::Range {
                path,
                start_byte,
                end_byte,
            } => {
                let resolved = crate::safety::resolve_within_root(&self.root, path)?;
                WireFindSimilarInput::OpenRange {
                    path: resolved,
                    start_byte,
                    end_byte,
                }
            }
            FindSimilarInput::Snippet { snippet, language } => WireFindSimilarInput::Snippet {
                snippet: snippet.to_owned(),
                language: language.to_owned(),
            },
        };
        let request = FindSimilarRequest {
            input: wire_input,
            max_results: Some(top_n),
        };
        let params = serde_json::to_value(&request)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc serialise: {err}")))?;
        let result = ipc_call(&self.ipc_socket, "duplicates/findSimilar", &params)?;
        let clusters: Vec<ReportCluster> = serde_json::from_value(
            result.get("clusters").cloned().unwrap_or(json!([])),
        )
        .map_err(|err| BackendError::StateFileCorrupt(format!("ipc clusters parse: {err}")))?;
        let below_min_nodes = result
            .get("below_min_nodes")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        Ok(FindSimilarOutput {
            clusters,
            below_min_nodes,
        })
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
        let result = ipc_call(&self.ipc_socket, "embedding/listModels", &json!({}))?;
        let models = serde_json::from_value::<Vec<WireEmbeddingModelInfo>>(result)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc models parse: {err}")))?;
        Ok(models.into_iter().map(model_info_from_wire).collect())
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
            incremental: true,
            cumulative_cache_stats: report.cache_stats,
        })
    }

    fn mark_changed(&self, _paths: &[PathBuf]) -> Result<(), BackendError> {
        let _refreshed = self.request_lsp_refresh()?;
        match self.reload_cache() {
            Ok(_) => {
                let gen = self.generation.load(Ordering::Relaxed);
                if let Ok(guard) = self.sender.lock() {
                    if let Some(s) = guard.as_ref() {
                        push_report_changed(s, gen);
                    }
                }
                Ok(())
            }
            Err(BackendError::LspNotRunning) => Ok(()),
            Err(err) => Err(err),
        }
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

/// Validates the compact `deslop.lsp.refreshReport` IPC response.
fn validate_refresh_result(result: &Value) -> Result<(), BackendError> {
    if result.get("command").and_then(Value::as_str) == Some("deslop.lsp.refreshReport") {
        return Ok(());
    }
    Err(BackendError::StateFileCorrupt(format!(
        "ipc refresh returned unexpected payload: {result}"
    )))
}
