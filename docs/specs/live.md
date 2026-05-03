# Live analysis — in-process session in the LSP

`deslop-lsp` runs a persistent `AnalysisSession` that watches the workspace, re-analyses on every file change, and writes the current report to a shared state file on disk. `deslop-mcp` reads that state file — it runs no analysis of its own. The CLI is unchanged: no watcher, no background threads, exits after one pass.

See also: [lsp.md](lsp.md), [mcp.md](mcp.md).

### [LIVE-PACKAGING] Crate + binary layout

The `live` module lives **inside `deslop-core`**, gated behind the `live` cargo feature. Only one binary links it:

- `crates/deslop-lsp` — JSON-RPC over stdio. Owns the `AnalysisSession`, the watcher, the scheduler, and all pipeline work. Writes the state file and the IPC socket.

`crates/deslop-mcp` does **not** link the `live` feature. It reads the LSP's output via the state file ([LIVE-STATE-FILE]) and delegates compute operations via the IPC socket ([LIVE-IPC-SOCKET]).

`crates/deslop` (CLI) does not link `live` either — zero watcher, zero background threads.

```mermaid
flowchart LR
    CI(["CI / terminal"])

    subgraph VSCode["VS Code process"]
        direction TB
        UI["Deslop VSIX (bubble · tree · webview · status bar)"]
        LspClient["LSP client"]
        McpHost["Bundled MCP host"]
        UI --> LspClient
    end

    subgraph AgentHost["AI agent host (Claude Code · Cursor · Continue)"]
        Agent["Agent + MCP client"]
    end

    subgraph LspProc["deslop-lsp process"]
        LspInner["AnalysisSession · watcher · scheduler · LiveApi\n(deslop-core live feature linked in)"]
    end

    subgraph McpProc["deslop-mcp process"]
        McpInner["State-file reader + in-memory cache\nIPC delegate for compute ops\n(no analysis work)"]
    end

    CliProc(["deslop CLI process\n(one-shot batch)"])

    StateFile[(".deslop-cache/live-report.json")]
    IpcSocket[(".deslop-cache/deslop.sock")]
    DiskCache[(".deslop-cache/\nfingerprints + embeddings")]
    Workspace[(Workspace files)]
    Ollama[(Ollama /api/embed)]

    LspClient == "spawns · LSP stdio" ==> LspProc
    McpHost == "spawns · MCP stdio" ==> McpProc
    Agent == "spawns · MCP stdio" ==> McpProc
    CI == "spawns one-shot" ==> CliProc

    Workspace -- "file events (notify)" --> LspProc
    Workspace -- "walk + read" --> CliProc

    LspProc -- "atomic write after every pass" --> StateFile
    LspProc -- "read/write" --> DiskCache
    LspProc -- "listens" --> IpcSocket
    LspProc <-- "embed batches" --> Ollama

    McpProc -- "reads (cached in-memory)" --> StateFile
    McpProc -- "find-similar · listModels" --> IpcSocket

    CliProc -- "read/write" --> DiskCache
    CliProc <-- "embed batches" --> Ollama
```

### [LIVE-LIFECYCLE] Session lifecycle

One `AnalysisSession` per workspace root, owned by `deslop-lsp`. On `initialize`:

1. Opens `.deslop-cache/` for the root (fingerprint cache + embedding cache).
2. Runs a full initial analysis (warm cache on second launch → cheap).
3. Writes the initial report to `.deslop-cache/live-report.json` ([LIVE-STATE-FILE]).
4. Starts the IPC socket ([LIVE-IPC-SOCKET]).
5. Starts the file watcher ([LIVE-WATCHER]).
6. Starts the re-analysis scheduler ([LIVE-SCHEDULER]).
7. Sends `ready` with the initial `Report` to the LSP client.

Shutdown: stop accepting new edits, finish the current pass, flush caches, remove the IPC socket, exit. The session never writes outside `.deslop-cache/` and never modifies source files.

### [LIVE-PROFILING] CPU repro evidence

When `deslop-lsp` appears pegged at 100% CPU, capture both diagnosis channels:

1. Run the VS Code command `Deslop: Reveal CPU Report` and attach the markdown output.
2. Restart the extension host with `DESLOP_PROFILE_DIR=~/Desktop`, reproduce the spike, then zip and attach the generated `deslop-lsp-*-firefox-profile.json` file.

The profile path is compiled behind the `deslop-lsp` `profiling` cargo feature and is active only when `DESLOP_PROFILE_DIR` is set. The file is Firefox processed-profile JSON and can be opened at `https://profiler.firefox.com/` for stack inspection.

### [LIVE-EMBEDDING-CONSENT] Explicit live embedding consent

A fresh live session starts with structural + token/LSH signals only. The embedding pass is opt-in at the model boundary. The user selects a model from `embedding/listModels`; `embedding/setModel` is the consent boundary: the selected provider/model is recorded, the embedding cache is invalidated, and embedding work is queued immediately. Agent surfaces must not call this boundary autonomously or infer a preferred model.

Embedding refreshes are always low priority with bounded batches and yield states between them so the LSP transport, watcher, and editor remain responsive. `latest_report` serves the last complete structural/token report until the embedding-enhanced generation is ready.

Progress is observable: `queued`, `starting`, `running`, `complete`, `failed`.

A user-approved model switch from either live surface writes the shared workspace embedding settings (`.vscode/settings.json` keys `deslop.embedding.*`). The MCP must not hold a successful model change in process memory only — it writes the settings file so LSP picks it up on config reload.

### [LIVE-STATE] In-process state

```rust
pub struct AnalysisSession {
    pipeline: PipelineSession,
    latest_report: Arc<Report>,
    generation: u64,
    subscribers: Vec<Subscriber>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
}
```

`PipelineSession` carries the file registry, per-file fingerprints, normalised trees, and source bytes ([PIPELINE-INCREMENTAL]). `AnalysisSession` adds orchestration state: the current snapshot, the generation counter, and the subscriber list. All mutable state is reachable from `AnalysisSession`. [STATE-FILE-REGISTRY] is still the only blessed process-global.

### [LIVE-STATE-FILE] Shared state file

After every scheduler pass that produces a new generation, the LSP writes the current report to:

```
{workspace_root}/.deslop-cache/live-report.json
```

**Write is atomic:** write to `live-report.json.tmp`, then `rename()`. Readers always see a complete file or the previous version — never a partial write.

**Format:** canonical `Report` JSON — identical schema to `deslop --output report.json`. The `generation` field lets readers detect that a newer version is available. This file is also what the `deslop://report` MCP resource serves.

If the LSP is not running, the MCP serves the last written state — stale but complete for its generation.

### [LIVE-IPC-SOCKET] IPC socket

Compute-heavy operations that cannot be answered from the state file are delegated to the running LSP via a local socket:

- **Unix/macOS:** `{workspace_root}/.deslop-cache/deslop.sock` (Unix domain socket)
- **Windows:** `\\.\pipe\deslop-{workspace_root_hash}` (named pipe)

The LSP creates the socket on startup and removes it on clean shutdown. The MCP connects on demand (lazy, not persistent). The IPC protocol is JSON-RPC 2.0 using a subset of the `LiveApi` methods:

| Method | Used by MCP for |
|---|---|
| `duplicates/findSimilar` | `find-similar` tool |
| `embedding/listModels` | `list-embedding-models` tool |
| `session/config` | `session-config` tool (live fields) |

If the socket is absent (LSP not running), `find-similar` returns `LspNotRunning`. `list-embedding-models` and `session-config` fall back to the state file where possible.

### [LIVE-WATCHER] File watcher

**The watcher runs only in `deslop-lsp`.** `deslop-mcp` watches only `.deslop-cache/live-report.json` (a single file) for change notifications — it never watches the workspace.

Use the `notify` crate (cross-platform, zero C deps). Watch the workspace root recursively, filtered by `LanguageParser::file_extensions()`. Debounce: **250 ms** of quiet after the last event, capped at **2 s** total accumulation so a formatter burst doesn't starve the scheduler.

Events matching `[EXCLUSION-CONFIG]` `exclude` patterns are dropped before debounce.

The LSP supplements the watcher with `textDocument/didChange` and `workspace/didChangeWatchedFiles` from the editor — belt-and-suspenders for in-buffer edits where the OS watcher may lag. Both paths converge on the same `AnalysisSession`.

### [LIVE-SCHEDULER] Re-analysis scheduler

After the watcher emits a coalesced changeset:

1. Calls `PipelineSession::update_files(changed: &[PathBuf]) -> Report`.
2. Pipeline reuses fingerprint and embedding caches.
3. Recomputes clustering + ranking over the updated fingerprint set.
4. Atomically swaps `latest_report`; bumps `generation`.
5. Writes the new report to `.deslop-cache/live-report.json` ([LIVE-STATE-FILE]).
6. Pushes a `ReportDelta` to every LSP subscriber.

Single-threaded per session. Consecutive queued changesets merge before dispatch.

Budget: ≤ 10 changed files, warm cache, 100 K-LOC → **< 500 ms**. Miss the budget → `tracing::warn!` with timing breakdown.

### [LIVE-DELTA] Report deltas

`ReportDelta` is the wire diff between two generations. LSP subscribers consume deltas instead of full snapshots so update traffic stays small.

```rust
pub struct ReportDelta {
    pub from_generation: u64,
    pub to_generation: u64,
    pub clusters_added: Vec<ReportCluster>,
    pub clusters_removed: Vec<String>,
    pub clusters_updated: Vec<ReportCluster>,
    pub cache_stats: CacheStats,
    pub tool_version: String,
}
```

Cluster ids are stable across runs ([REPORTING-CONTEXT §"How to read the report format"]). Clients that miss generations ask for a full snapshot via `report/get`, then resume delta consumption at the snapshot's generation.

### [LIVE-QUERY-API] Query API (LSP-internal)

The `live` module exposes the `LiveApi` trait. The LSP holds a `LiveApi` impl and routes LSP-transport requests to it. The MCP does **not** hold `LiveApi` — it reads the state file for snapshot queries and connects via the IPC socket for compute operations.

| Method | Input | Output | Purpose |
|---|---|---|---|
| `report/get` | `{}` | `Report` | Full current snapshot. |
| `report/delta` | `{ since_generation: u64 }` | `ReportDelta \| null` | Pull changes since a known generation. |
| `report/forFile` | `{ path }` | `FileReport` | Clusters touching this file. |
| `report/forRange` | `{ path, start_byte, end_byte }` | `Vec<ReportCluster>` | Clusters overlapping the byte range. |
| `cluster/byId` | `{ id }` | `ReportCluster` | Fetch by stable id. |
| `duplicates/findSimilar` | `{ path, start_byte, end_byte }` or `{ snippet, language }` | `Vec<ReportCluster>` | Parse + fingerprint + LSH + embedding against the live index. No cache mutation. |
| `embedding/listModels` | `{}` | `Vec<EmbeddingModelInfo>` | Enumerate available Ollama models. |
| `embedding/setModel` | `{ provider_id, model_id, endpoint? }` | `EmbeddingProvenance \| null` | Switch the live embedding model; write workspace settings. |
| `session/config` | `{}` | `SessionConfig` | min-nodes, languages, embedding provenance, exclusion config, cache dir. |

### [LIVE-NOTIFICATIONS] Push notifications

The LSP pushes three notification types to LSP clients (VSIX, other editors):

- `report/changed` — fires after every pass with a non-empty delta. Payload: `{ generation: u64, summary: ChangeSummary }`. Must fire for pure removals — suppressing it is a bug.
- `analysis/state` — fires on `idle → running`, `running → idle`, and on scheduler errors.
- `embedding/progress` — fires around embedding refreshes. Payload: `{ phase, provider_id, model_id, done, total, message? }`.

The MCP is not an LSP subscriber. It detects report changes by watching `.deslop-cache/live-report.json` for modification and pushes its own MCP notifications ([MCP-NOTIFICATIONS]).

### [LIVE-PERF-BUDGETS] Performance budgets

| Scenario | Budget |
|---|---|
| Cold start, empty cache, 100 K LOC | Same as `--incremental` CLI first-run. |
| Warm start, warm cache, 100 K LOC | < 2 s to `ready`. |
| Incremental re-analysis, ≤ 10 changed files | < 500 ms end-to-end. |
| `report/forFile`, 100 K-LOC report | < 50 ms. |
| `duplicates/findSimilar`, ≤ 200-node snippet | < 250 ms. |

Missed budgets → `tracing::warn!` with timing breakdown. Ratchet only.

### [LIVE-NO-REGEX-NO-SHORTCUTS] Rules inherited

No regex on source, no `unwrap`, no panics, `thiserror` for library errors, structured `tracing` only, 500-line file budget, coarse E2E tests only. E2E tests drive the real LSP binary over stdio with a fixture workspace and assert against rendered deltas — never reach into `AnalysisSession` internals.
