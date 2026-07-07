//! `LiveBackend` — [`McpBackend`] implementation that calls the LSP
//! IPC endpoint on every read ([MCP-WHY-LIVE], [MCP-IPC-CLIENT]).
//! Implements [PRINCIPLES-LIVE-IS-REACTIVE] for agent-facing reads.
//!
//! Single source of truth: the LSP's in-memory `latest_report`
//! ([LIVE-IPC-SOCKET]). The MCP never touches
//! `.deslop-cache/live-report.json` — that file is the LSP's private
//! warm-start cache ([LIVE-SEED-CACHE]), not a wire contract. Every
//! read tool call issues one JSON-RPC request over the published
//! transport — the Unix socket `.deslop-cache/deslop.sock`, or the
//! TCP loopback endpoint from `.deslop-cache/deslop.port` on Windows
//! and under `--ipc-transport tcp` ([LIVE-IPC-TCP],
//! [MCP-IPC-DISCOVERY]). When no endpoint is live, callers receive
//! [`BackendError::LspNotRunning`] — there is no fallback pipeline;
//! CI/one-shot use the `deslop` CLI.

use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, atomic::Ordering, Arc, Mutex},
};

use deslop_core::live::{transport::IpcStream, wire::ReportChangedNotification};

use deslop_core::{
    live::wire::{
        ChangeSummary, EmbeddingModelInfo as WireEmbeddingModelInfo,
        FindSimilarInput as WireFindSimilarInput, FindSimilarRequest, SessionConfig,
    },
    report::ReportCluster,
    EmbeddingSpec, Report,
};
use serde_json::{json, Value};
use tracing::debug;

use crate::{notify::push_report_changed, NotificationSender};

use super::{
    ipc::ipc_call, BackendError, FindSimilarInput, FindSimilarOutput, McpBackend, RescanProgress,
    SessionBackendConfig, SessionConfigSnapshot,
};

/// `McpBackend` implementation that delegates every read to the LSP
/// over the published IPC transport. No on-disk cache, no file
/// watcher, no duplicate analysis pipeline.
pub struct LiveBackend {
    /// Workspace root pinned at initialisation.
    root: PathBuf,
    /// Absolute Unix-socket path, kept for error reporting; the TCP
    /// discovery record lives beside it ([MCP-IPC-DISCOVERY]).
    ipc_socket: PathBuf,
    /// Last-seen LSP generation. Updated lazily after each IPC read so
    /// the synchronous `generation()` accessor stays cheap.
    generation: Arc<AtomicU64>,
    /// Shared notification sender wired by [`McpServer::run`].
    sender: Arc<Mutex<Option<NotificationSender>>>,
}

impl std::fmt::Debug for LiveBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LiveBackend")
            .field("root", &self.root)
            .field("ipc_socket", &self.ipc_socket)
            .finish_non_exhaustive()
    }
}

impl LiveBackend {
    /// Constructs the backend from `config`. Lightweight — no IPC traffic.
    ///
    /// # Errors
    ///
    /// Currently infallible; returns `Result` for symmetry with the
    /// previous backend signature so callers do not have to branch.
    pub fn initialise(config: SessionBackendConfig) -> Result<Self, BackendError> {
        let ipc_socket = deslop_core::live::transport::socket_path(&config.root);
        Ok(Self {
            root: config.root,
            ipc_socket,
            generation: Arc::new(AtomicU64::new(0)),
            sender: Arc::new(Mutex::new(None)),
        })
    }

    /// Issues an IPC `report/get` and decodes the result into `Report`.
    fn fetch_report(&self) -> Result<Arc<Report>, BackendError> {
        let result = ipc_call(&self.root, "report/get", &json!({}))?;
        let report: Report = serde_json::from_value(result)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc report parse: {err}")))?;
        Ok(Arc::new(report))
    }

    /// Issues `session/config` and decodes the response.
    fn fetch_session_config(&self) -> Result<SessionConfig, BackendError> {
        let result = ipc_call(&self.root, "session/config", &json!({}))?;
        let config: SessionConfig = serde_json::from_value(result).map_err(|err| {
            BackendError::StateFileCorrupt(format!("ipc session_config parse: {err}"))
        })?;
        Ok(config)
    }
}

/// Connects to the subscribe endpoint over whichever transport the
/// LSP published, sends the initial JSON-RPC request, blocks for the
/// ack frame, and returns the buffered reader alongside the LSP's
/// current generation. Returning the reader (not the underlying
/// stream) preserves any bytes the LSP wrote between the ack and the
/// next user call, which a separate `try_clone`'d stream would lose
/// to `BufReader`'s internal buffer.
fn connect_subscribe_and_read_ack(
    root: &Path,
) -> Result<(BufReader<IpcStream>, u64), BackendError> {
    use std::io::Write;
    let mut stream = super::ipc::connect(root)?;
    let payload = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "report/subscribe",
        "params": {},
    }))
    .unwrap_or_default();
    let io_error =
        |err: std::io::Error| BackendError::StateFileCorrupt(format!("ipc subscribe: {err}"));
    stream.write_all(&payload).map_err(io_error)?;
    stream.write_all(b"\n").map_err(io_error)?;
    stream.flush().map_err(io_error)?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let _bytes = reader.read_line(&mut line).map_err(io_error)?;
    let generation = parse_subscribe_ack_generation(&line).unwrap_or(0);
    Ok((reader, generation))
}

/// Spawns the background thread that drains broadcast notifications
/// from `reader` and forwards them as MCP `reportChanged` frames.
fn spawn_subscribe_reader(
    reader: BufReader<IpcStream>,
    sender: Arc<Mutex<Option<NotificationSender>>>,
    generation: Arc<AtomicU64>,
) {
    let _thread = std::thread::spawn(move || {
        let mut reader = reader;
        let mut line = String::new();
        while reader.read_line(&mut line).is_ok() && !line.is_empty() {
            if let Some(notification) = parse_report_changed(&line) {
                generation.store(notification.generation, Ordering::Relaxed);
                let Ok(guard) = sender.lock() else {
                    return;
                };
                if let Some(s) = guard.as_ref() {
                    push_report_changed(s, notification.generation);
                }
            }
            line.clear();
        }
        debug!("mcp_subscribe_loop_exited");
    });
}

/// Parses one `report/changed` notification frame, returning `None`
/// when the line is not a recognisable notification.
fn parse_report_changed(line: &str) -> Option<ReportChangedNotification> {
    let frame: Value = serde_json::from_str(line.trim()).ok()?;
    if frame.get("method").and_then(Value::as_str) != Some("report/changed") {
        return None;
    }
    let params = frame.get("params").cloned()?;
    serde_json::from_value(params).ok()
}

/// Reads the `generation` field from the `report/subscribe` ack so
/// the MCP can sync its counter without an extra IPC round-trip.
fn parse_subscribe_ack_generation(line: &str) -> Option<u64> {
    let frame: Value = serde_json::from_str(line.trim()).ok()?;
    frame
        .get("result")
        .and_then(|result| result.get("generation"))
        .and_then(Value::as_u64)
}

impl McpBackend for LiveBackend {
    fn root(&self) -> &Path {
        &self.root
    }

    fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    fn report_get(&self) -> Result<Arc<Report>, BackendError> {
        self.fetch_report()
    }

    fn report_for_file(&self, path: &Path) -> Result<Vec<ReportCluster>, BackendError> {
        let resolved = crate::safety::resolve_within_root(&self.root, path)?;
        let result = ipc_call(&self.root, "report/forFile", &json!({ "path": resolved }))?;
        let file_report: deslop_core::live::wire::FileReport = serde_json::from_value(result)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc forFile parse: {err}")))?;
        Ok(file_report.clusters)
    }

    fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Result<Vec<ReportCluster>, BackendError> {
        let resolved = crate::safety::resolve_within_root(&self.root, path)?;
        let result = ipc_call(
            &self.root,
            "report/forRange",
            &json!({
                "path": resolved,
                "start_byte": start_byte,
                "end_byte": end_byte,
            }),
        )?;
        let clusters: Vec<ReportCluster> = serde_json::from_value(result)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc forRange parse: {err}")))?;
        Ok(clusters)
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
        let result = ipc_call(&self.root, "duplicates/findSimilar", &params)?;
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
        let result = cluster_scoped_ipc(&self.root, "cluster/byId", id)?;
        let cluster: ReportCluster = serde_json::from_value(result)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc cluster parse: {err}")))?;
        Ok(cluster)
    }

    fn merge_plan(&self, id: &str) -> Result<deslop_core::wire_generated::MergePlan, BackendError> {
        let result = cluster_scoped_ipc(&self.root, "merge/plan", id)?;
        let plan: deslop_core::wire_generated::MergePlan =
            serde_json::from_value(result).map_err(|err| {
                BackendError::StateFileCorrupt(format!("ipc merge-plan parse: {err}"))
            })?;
        Ok(plan)
    }

    fn list_embedding_models(&self) -> Result<Vec<WireEmbeddingModelInfo>, BackendError> {
        let result = ipc_call(&self.root, "embedding/listModels", &json!({}))?;
        let models = serde_json::from_value::<Vec<WireEmbeddingModelInfo>>(result)
            .map_err(|err| BackendError::StateFileCorrupt(format!("ipc models parse: {err}")))?;
        Ok(models)
    }

    fn set_embedding_model(
        &self,
        _provider_id: &str,
        _model_id: &str,
        _endpoint: Option<&str>,
    ) -> Result<EmbeddingSpec, BackendError> {
        Err(BackendError::LspNotRunning {
            socket_path: self.ipc_socket.clone(),
        })
    }

    fn session_config(&self) -> Result<SessionConfigSnapshot, BackendError> {
        let config = self.fetch_session_config()?;
        Ok(SessionConfigSnapshot {
            root: config.workspace_root,
            min_nodes: config.min_nodes,
            languages: config.languages,
            embedding_provenance: config.embedding_provenance,
            incremental: config.incremental,
            cumulative_cache_stats: deslop_core::report::CacheStats::default(),
        })
    }

    fn mark_changed(&self, _paths: &[PathBuf]) -> Result<RescanProgress, BackendError> {
        let result = match ipc_call(&self.root, "deslop.lsp.refreshReport", &json!({})) {
            Ok(result) => result,
            Err(BackendError::LspNotRunning { .. }) => {
                return Ok(empty_rescan_progress(
                    self.generation.load(Ordering::Relaxed),
                ))
            }
            Err(err) => return Err(err),
        };
        let progress = refresh_progress_from_result(&result)?;
        self.generation
            .store(progress.generation, Ordering::Relaxed);
        if let Ok(guard) = self.sender.lock() {
            if let Some(s) = guard.as_ref() {
                push_report_changed(s, progress.generation);
            }
        }
        Ok(progress)
    }

    fn set_notification_sender(&self, sender: NotificationSender) {
        let Ok(mut guard) = self.sender.lock() else {
            return;
        };
        *guard = Some(sender);
        drop(guard);
        // Synchronously open the subscribe socket so the initial
        // generation is recorded before any tool call returns. The
        // subscribe ack carries the LSP's current generation; the
        // same BufReader that consumed it is then handed to the
        // background reader thread so any `report/changed` frames the
        // LSP wrote between ack and reader spawn are not lost in a
        // discarded buffer ([MCP-IPC-CLIENT]).
        match connect_subscribe_and_read_ack(&self.root) {
            Ok((reader, initial_generation)) => {
                self.generation.store(initial_generation, Ordering::Relaxed);
                spawn_subscribe_reader(
                    reader,
                    Arc::clone(&self.sender),
                    Arc::clone(&self.generation),
                );
            }
            Err(error) => debug!(reason = %error, "mcp_subscribe_connect_failed"),
        }
    }
}

/// Converts the compact `deslop.lsp.refreshReport` IPC response into
/// the progress fields exposed by MCP `rescan`.
fn refresh_progress_from_result(result: &Value) -> Result<RescanProgress, BackendError> {
    if result.get("command").and_then(Value::as_str) != Some("deslop.lsp.refreshReport") {
        return Err(BackendError::StateFileCorrupt(format!(
            "ipc refresh returned unexpected payload: {result}"
        )));
    }
    Ok(RescanProgress {
        generation: required_u64(result, "generation")?,
        summary: ChangeSummary {
            clusters_added: required_usize(result, "clustersAdded")?,
            clusters_removed: required_usize(result, "clustersRemoved")?,
            clusters_updated: required_usize(result, "clustersUpdated")?,
            worst_weight: 0.0,
        },
    })
}

/// Reads a required unsigned IPC field and converts it to `usize`.
fn required_usize(result: &Value, field: &str) -> Result<usize, BackendError> {
    let value = required_u64(result, field)?;
    usize::try_from(value).map_err(|_| {
        BackendError::StateFileCorrupt(format!("ipc refresh field {field} overflows usize"))
    })
}

/// Reads a required unsigned IPC field as `u64`.
fn required_u64(result: &Value, field: &str) -> Result<u64, BackendError> {
    result.get(field).and_then(Value::as_u64).ok_or_else(|| {
        BackendError::StateFileCorrupt(format!(
            "ipc refresh missing unsigned integer field {field}: {result}"
        ))
    })
}

/// Builds a zero-summary fallback for cache-only rescans.
fn empty_rescan_progress(generation: u64) -> RescanProgress {
    RescanProgress {
        generation,
        summary: ChangeSummary::default(),
    }
}

/// Issues a cluster-scoped IPC call, mapping the LSP's unknown-id
/// JSON-RPC error onto the stable `UnknownCluster` code — the generic
/// IPC client folds every RPC error into `StateFileCorrupt`, but the
/// MCP wire contract requires the stable code ([MCP-TESTING]).
fn cluster_scoped_ipc(
    root: &std::path::Path,
    method: &str,
    id: &str,
) -> Result<serde_json::Value, BackendError> {
    match ipc_call(root, method, &json!({ "id": id })) {
        Ok(value) => Ok(value),
        Err(BackendError::StateFileCorrupt(message))
            if message.contains("unknown cluster id") || message.contains("no cluster with id") =>
        {
            Err(BackendError::UnknownCluster(id.to_owned()))
        }
        Err(error) => Err(error),
    }
}
