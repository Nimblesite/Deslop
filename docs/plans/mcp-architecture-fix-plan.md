# MCP Architecture Fix — [LIVE-STATE-FILE] / [LIVE-IPC-SOCKET]

## The Problem

`deslop-mcp` runs its own `PipelineSessionBackend` with its own `PipelineSession`, embedding provider, and re-analysis logic. This is wrong:

- Two separate analysis engines on the same workspace → results can diverge
- MCP burns CPU on work the LSP already did
- MCP has CLI args (`--min-nodes`, `--embeddings`, `--embedding-model`, …) that imply a second configurable analysis session — that session must not exist

**The fix:** LSP is the sole analysis process. MCP is a dumb state reader.

## Architecture

1. LSP writes `{root}/.deslop-cache/live-report.json` after every scheduler pass (atomic rename).
2. LSP listens on `{root}/.deslop-cache/deslop.sock` for compute delegation.
3. MCP reads the state file, caches the parsed `Report` in memory, and watches the file for changes. Zero analysis work.
4. MCP delegates `find-similar` and `list-embedding-models` to the LSP socket. Returns `LspNotRunning` if the socket is absent.

Full spec: [live.md §[LIVE-STATE-FILE]](../specs/live.md#live-state-file), [live.md §[LIVE-IPC-SOCKET]](../specs/live.md#live-ipc-socket), [mcp.md](../specs/mcp.md).

## Phase 1 — LSP writes the state file [LIVE-STATE-FILE]

**Files:** `crates/deslop-lsp/src/file_watch.rs`, `crates/deslop-core/src/live/`

- After every scheduler pass with a new generation: write `Report` JSON to `live-report.json.tmp`, rename into place.
- Also write on initial `ready`.
- Log: `tracing::info!(path, generation, "state_file_written")`.

**Test:** modify a fixture file → assert `live-report.json` reflects the change within 500 ms.

## Phase 2 — LSP IPC socket [LIVE-IPC-SOCKET]

**Files:** `crates/deslop-lsp/src/ipc.rs` (new), `crates/deslop-lsp/src/backend.rs`

- On `initialize`: create `deslop.sock` (Unix) or named pipe (Windows).
- Accept line-delimited JSON-RPC 2.0. Supported methods: `duplicates/findSimilar`, `embedding/listModels`, `session/config`.
- Each connection: connect → request → response → close.
- On shutdown: remove the socket file.

**Test:** after LSP `initialize`, connect to socket and call each method; assert response matches the LSP's own report.

## Phase 3 — MCP refactor [MCP-STATE-FILE] / [MCP-REPORT-CACHE]

**Files:** `crates/deslop-mcp/src/` — major surgery.

### Delete

- `backend/pipeline.rs` (`PipelineSessionBackend`, `SessionState`)
- `backend/refresh.rs` (embedding refresh worker)
- All `PipelineSession`, `EmbeddingMode`, `OllamaProvider`, `StubProvider`, `EmbeddingProvider` usage
- CLI args: `--min-nodes`, `--incremental`, `--embeddings`, `--embedding-provider`, `--embedding-model`, `--embedding-endpoint`

### Add

`state.rs` — `StateFileReader`:

```rust
pub struct StateFileReader {
    state_file: PathBuf,
    ipc_socket: PathBuf,
    cached: Arc<RwLock<Option<Arc<Report>>>>,
}
```

- `report()` → reads state file on first call; returns `LspNotRunning` if absent.
- `ipc_call(method, params)` → connects to socket, sends JSON-RPC, returns response; returns `LspNotRunning` if socket absent.

`watcher.rs` — single-file `notify` watch on `live-report.json`:
- On `Modify`: call `state.load()`, push `notifications/resources/updated` + `notifications/deslop/reportChanged`.

### Tool routing

| Tool | Source |
|------|--------|
| `report-get`, `report-query`, `report-for-file`, `report-for-range`, `cluster-by-id` | `StateFileReader::report()` |
| `find-similar` | `ipc_call(duplicates/findSimilar)` |
| `list-embedding-models` | `ipc_call(embedding/listModels)` |
| `set-embedding-model` | write `.vscode/settings.json` + `ipc_call(embedding/setModel)` |
| `session-config` | static fields from state file; live fields from `ipc_call(session/config)` |

### Keep CLI args

`--root` and `--config` only.

## Phase 4 — MCP push notifications rewired [MCP-NOTIFICATIONS]

The existing `mark_changed` → internal `NotificationSender` path is deleted. Notifications now fire from the `watcher.rs` file-watch path (Phase 3). Rewrite `files_changed_pushes_resources_updated_and_report_changed_notifications` to:

1. Spawn LSP + MCP side-by-side on the same fixture root.
2. Modify a source file on disk.
3. Assert MCP pushes both notification frames within 500 ms.

## Phase 5 — MCP E2E tests [MCP-TESTING]

- **Snapshot tools**: pre-write a valid `live-report.json` fixture; spawn MCP standalone; assert tool responses.
- **Compute tools**: spawn LSP first (wait for `ready`), then spawn MCP on the same root; assert `find-similar` returns the expected cluster.
- **Notifications**: as Phase 4.
- Delete any MCP tests that reach into pipeline internals.

## Completion criteria

- `crates/deslop-mcp/` contains no `PipelineSession`, `EmbeddingMode`, `OllamaProvider`, or `StubProvider`.
- `deslop-mcp --help` shows only `--root` and `--config`.
- `make test` passes with LSP + MCP E2E tests running side-by-side.
- `find-similar` via MCP returns the same result as the LSP for the same byte range.
- MCP push notifications fire within 500 ms of a file change that triggers LSP re-analysis.
- Coverage threshold in `coverage-thresholds.json` does not regress.
