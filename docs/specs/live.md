# Live analysis — in-process session in the LSP

`deslop-lsp` runs a persistent `AnalysisSession` that watches the workspace, re-analyses on every file change, and exposes the current in-memory report through a local IPC endpoint. `deslop-mcp` delegates to that endpoint — it runs no analysis of its own and does not read the warm-start state file. The CLI is unchanged: no watcher, no background threads, exits after one pass.

See also: [lsp.md](lsp.md), [mcp.md](mcp.md).

### [LIVE-PACKAGING] Crate + binary layout

The `live` module lives **inside `deslop-core`**, gated behind the `live` cargo feature. Only one binary links it:

- `crates/deslop-lsp` — JSON-RPC over stdio. Owns the `AnalysisSession`, the watcher, the scheduler, and all pipeline work. Writes the warm-start seed cache and exposes the IPC endpoint.

`crates/deslop-mcp` does **not** link the `live` feature. It is a pure transport adapter that delegates **every** read and compute call to the running LSP via the IPC socket ([LIVE-IPC-SOCKET]). It never reads `.deslop-cache/live-report.json` — that file is the LSP's private warm-start cache ([LIVE-SEED-CACHE]), not a wire contract.

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
        McpInner["IPC delegate\n(no analysis work)"]
    end

    CliProc(["deslop CLI process\n(one-shot batch)"])

    StateFile[(".deslop-cache/live-report.json")]
    IpcSocket[(".deslop-cache/deslop.sock\nor .deslop-cache/deslop.port")]
    DiskCache[(".deslop-cache/\nfingerprints + embeddings")]
    Workspace[(Workspace files)]
    Ollama[(Ollama /api/embed)]

    LspClient == "spawns · LSP stdio" ==> LspProc
    McpHost == "spawns · MCP stdio" ==> McpProc
    Agent == "spawns · MCP stdio" ==> McpProc
    CI == "spawns one-shot" ==> CliProc

    Workspace -- "file events (notify)" --> LspProc
    Workspace -- "walk + read" --> CliProc

    LspProc -- "warm-start seed" --> StateFile
    LspProc -- "read/write" --> DiskCache
    LspProc -- "listens" --> IpcSocket
    LspProc <-- "embed batches" --> Ollama

    McpProc -- "all reads · find-similar · listModels" --> IpcSocket

    CliProc -- "read/write" --> DiskCache
    CliProc <-- "embed batches" --> Ollama
```

### [LIVE-BINARY] The live binary
`deslop-lsp` is the single executable that links the `live` feature of
`deslop-core` and owns the running `AnalysisSession`, file watcher, and scheduler
([LIVE-PACKAGING]). It is the only process that performs live analysis: the LSP
client (VSIX, JetBrains, any editor) and the agent-facing MCP both delegate to it,
the MCP over the IPC socket ([LIVE-IPC-SOCKET]) and the editor over LSP stdio.
"The live binary" elsewhere in the specs (e.g. [lsp.md](lsp.md)) names this
process.

### [LIVE-LIFECYCLE] Session lifecycle

One `AnalysisSession` per workspace root, owned by `deslop-lsp`. On `initialize`:

1. Opens `.deslop-cache/` for the root (fingerprint cache + embedding cache).
2. Runs a full initial analysis (warm cache on second launch → cheap).
3. Writes the initial report to `.deslop-cache/live-report.json` ([LIVE-SEED-CACHE]) so the next LSP startup can warm-start.
4. Starts the IPC socket ([LIVE-IPC-SOCKET]) — the read surface for the MCP.
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

### [LIVE-SEED-CACHE] Warm-start seed cache

After the **initial** full pipeline pass and after every **cold-pass install** (the post-cache-seed background refresh), the LSP writes the current report to:

```
{workspace_root}/.deslop-cache/live-report.json
```

**Write is atomic:** write to `live-report.json.tmp`, then `rename()`. Readers always see a complete file or the previous version — never a partial write.

**Format:** canonical `Report` JSON — identical schema to `deslop --output report.json`.

**Use:** the file is an **LSP-private startup cache**, not an IPC channel. On the next LSP startup, [LIVE-CACHE-SEED] (`AnalysisSession::try_seeded_from_cache`) loads it so the editor sees clusters within milliseconds while the cold full pass runs in the background.

**Not written on:** per-keystroke incremental updates ([LIVE-SCHEDULER]) and embedding refresh commits — those used to spam the disk and contributed nothing to startup latency. The MCP no longer reads this file ([MCP-IPC-CLIENT]); it gets live state via the IPC socket. Stale-cache reads cannot leak hidden clusters because no one reads the cache except the LSP itself, post-restart, before its first cold pass overwrites it.

### [LIVE-STATE-FILE] State file (alias)
`[LIVE-STATE-FILE]` is an alias for the `.deslop-cache/live-report.json`
warm-start cache specified in full by [LIVE-SEED-CACHE]: written atomically on
the initial full pass and on each cold-pass install, holding canonical `Report`
JSON. It is the LSP's private startup cache and the on-disk path named in the
MCP's `LspNotRunning` recovery payload ([MCP-IPC-DISCOVERY]); it is not an IPC
channel and is not rewritten on incremental passes. Prefer the [LIVE-SEED-CACHE]
tag for new references.

### [LIVE-IPC-SOCKET] IPC endpoint

The LSP exposes its in-memory `latest_report` directly through a local IPC endpoint. **This is the only read path used by the MCP.** No on-disk cache is consulted on the read side.

- **Unix/macOS:** `{workspace_root}/.deslop-cache/deslop.sock` (Unix domain socket) — the default transport.
- **Windows:** TCP loopback per [LIVE-IPC-TCP] — Windows has no Unix sockets, so `deslop-lsp` binds `127.0.0.1` and publishes the endpoint in a discovery record.

The LSP creates the endpoint on startup and removes its on-disk artifacts on clean shutdown. The MCP connects on demand (lazy, not persistent). Protocol: line-delimited JSON-RPC 2.0 on both transports.

### [LIVE-IPC-TCP] TCP loopback transport

Where Unix domain sockets do not exist (Windows) — or when `deslop-lsp` is started with `--ipc-transport tcp` on any platform — the IPC server binds an OS-assigned TCP port on `127.0.0.1` and publishes a **discovery record** at `{workspace_root}/.deslop-cache/deslop.port`. The record is the `IpcEndpointFile` wire model (typeDiagram, `docs/models/live-ipc.td`): `{ "port": <u16>, "token": "<64-hex>" }`.

- **Token gate.** The token is fresh per LSP session (128 bits of OS entropy, hex-encoded). A TCP client must present it as the **first line** of every connection, before any JSON-RPC; the server drops mismatching connections without a response. This keeps the analysis server closed to other local processes and turns a stale record colliding with a foreign listener into a clean failure instead of a garbage exchange. Unix-socket connections present no token — the filesystem already permission-guards the socket (the record itself is written `0600` where Unix permissions exist).
- **Same protocol.** Past the token line, both transports carry identical line-delimited JSON-RPC ([LIVE-IPC-SOCKET] method table applies verbatim).
- **Platform-neutral by construction.** The TCP path is plain `std::net` compiled on every platform, and the E2E suite (`crates/deslop-mcp/tests/tcp_transport.rs`) forces it on Unix CI — the code Windows runs in production is the code CI tests everywhere.
- The transport choice lives in `deslop_core::live::transport::IpcMode` (`platform_default()`: Unix socket where available, otherwise TCP).

### [LIVE-WATCHER] File watcher

**The watcher runs only in `deslop-lsp`.** `deslop-mcp` watches only `.deslop-cache/live-report.json` (a single file) for change notifications — it never watches the workspace.

Use the `notify` crate (cross-platform, zero C deps). Watch the workspace root recursively, filtered by `LanguageParser::file_extensions()`. Debounce: **250 ms** of quiet after the last event, capped at **2 s** total accumulation so a formatter burst doesn't starve the scheduler.

Events matching `[EXCLUSION-CONFIG]` `exclude` patterns are dropped before debounce.

The LSP supplements the watcher with `textDocument/didChange` and `workspace/didChangeWatchedFiles` from the editor — belt-and-suspenders for in-buffer edits where the OS watcher may lag. Both paths converge on the same `AnalysisSession`.

### [LIVE-SCHEDULER] Re-analysis scheduler

After the watcher emits a coalesced changeset:

1. Calls `PipelineSession::update_files(changed: &[PathBuf]) -> Option<Report>`.
2. Pipeline reuses fingerprint and embedding caches.
3. Returns `None` when the changeset touched no analysed file ([LIVE-SCHEDULER-NOOP]); steps 4-5 are then skipped entirely.
4. Recomputes clustering + ranking over the updated fingerprint set.
5. Atomically swaps `latest_report`; bumps `generation`.
6. Broadcasts the new generation through `report_changed` so both LSP push notifications and IPC `report/subscribe` subscribers receive the same event ([LIVE-IPC-SOCKET]).
7. **Does NOT** rewrite `live-report.json`. The seed cache is per-cold-pass only ([LIVE-SEED-CACHE]).

Single-threaded per session. Consecutive queued changesets merge before dispatch.

Budget: ≤ 10 changed files, warm cache, 100 K-LOC → **< 500 ms**. Miss the budget → `tracing::warn!` with timing breakdown.

### [LIVE-SCHEDULER-NOOP] No-op pass early-out

A changeset whose every path is rejected before it can touch the corpus — an
unsupported extension, an `exclude` match, an ignore-rule match
([LIVE-WATCHER]), or a removal naming a file the corpus never held — leaves the
fingerprint set byte-identical. The report is therefore provably identical too,
so `update_files` returns `None` and the scheduler:

- does **not** run LSH, embedding, candidate-pair, clustering, or ranking;
- does **not** swap `latest_report`;
- does **not** bump `generation` or broadcast `report_changed`.

The last point is a correctness requirement, not just an optimisation: a
generation bump is a promise to every subscriber that the report changed, and
honouring it forces the panel, the diagnostics, and every MCP client to re-fetch
an identical snapshot.

Only a mutation to an analysed file re-renders. A watched config or ignore-rule
path always counts as a mutation ([LIVE-CONFIG-LIVE]) because it re-scopes
rendering itself — hide patterns and thresholds — not merely the file set.

Without this early-out, steps 4-6 ran on every filesystem event the watcher
delivered. One production LSP session spent **11h17m of CPU across 1086 passes**
on a 172-file workspace, re-deriving 4,972 clusters from 422,711 fingerprints
and 366,765 candidate pairs roughly every 30 seconds; 139 of 157 logged passes
published a byte-identical report (#299).

### [LIVE-CONFIG-LIVE] Live `.deslop.toml` reload
Editing `<root>/.deslop.toml` (or the explicit `--config` override) is a watched
change ([LIVE-WATCHER]): the watch set always includes the canonical and
as-given config paths. When such a file lands in a changeset, the scheduler
reloads the exclusion config and re-evaluates the whole live corpus against it —
dropping files the new `exclude` patterns now match and re-discovering files a
removed pattern re-admits — then bumps the generation like any other pass
([LIVE-SCHEDULER]). A malformed config is logged at `warn` and the previous
exclusion is kept; a typo never bricks the daemon or empties the report.

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

The `live` module exposes the `LiveApi` trait. The LSP holds a `LiveApi` impl and routes both LSP-transport requests and IPC dispatches to it. The MCP does **not** hold `LiveApi`; every MCP read becomes one IPC round-trip to the LSP-held `LiveService` ([LIVE-IPC-SOCKET]). Single source of truth — no second copy of analysis state exists in the MCP process.

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

### [LIVE-RESCAN-FRESHNESS] Single-call rescan freshness
`rescan` (the `deslop.lsp.refreshReport` IPC method) blocks until a fresh full
pass completes and returns that pass's report — never one from before the
refresh. The `paths` argument is informational; the LSP always runs a full
`refresh_full` under the session lock. The response's `generation` and `clusters`
fields are drawn from the same post-refresh generation, so one call surfaces the
post-edit state — eliminated clusters disappear and surviving clusters carry
post-edit offsets without a second call.

### [LIVE-CLUSTER-OFFSET-FRESHNESS] Cluster offset freshness on read
A `cluster/byId` read (and `report/forFile` / `report/forRange` /
`duplicates/findSimilar`) returns occurrence byte and line ranges that map onto
the current on-disk file, even when the agent has edited files without an explicit
`rescan` between reads. The session re-stats every occurrence path on each IPC
read ([LIVE-CLUSTER-OFFSET-FRESHNESS]) and synchronously re-analyses any file whose mtime is
newer than last observed, before resolving the cluster. A stale offset that
overshoots the post-edit file length would corrupt agent context, so this gate is
mandatory on every agent-facing read path, not just `rescan`.

### [LIVE-NOTIFICATIONS] Push notifications

The LSP pushes three notification types to LSP clients (VSIX, other editors):

- `report/changed` — fires after every pass with a non-empty delta. Payload: `{ generation: u64, summary: ChangeSummary }`. Must fire for pure removals — suppressing it is a bug.
- `analysis/state` — fires on `idle → running`, `running → idle`, and on scheduler errors.
- `embedding/progress` — fires around embedding refreshes. Payload: `{ phase, provider_id, model_id, done, total, message? }`.

The MCP **is** an IPC subscriber. It opens one long-lived `report/subscribe` connection over the socket and re-emits each `report/changed` notification to its own client as `notifications/deslop/reportChanged` ([MCP-NOTIFICATIONS]). It never reads `.deslop-cache/live-report.json` and never watches the workspace.

### [LIVE-REPORT-DISPLAY] Human-readable report display
On-screen cluster views (LSP cluster virtual documents, hover bubbles, tree rows)
are for humans, not AI: they render `line:column` locations and source snippets,
never raw byte offsets. The cluster markdown renderer takes a source lookup and,
when source is unavailable, prints the occurrence path alone and omits the snippet
block rather than leaking offset internals. The raw JSON report retains byte
offsets for AI consumers ([REPORTING-CONTEXT.md](REPORTING-CONTEXT.md)); the
display layer translates them.

### [LIVE-PERF-BUDGETS] Performance budgets

| Scenario | Budget |
|---|---|
| Cold start, empty cache, 100 K LOC | Same as `--incremental` CLI first-run. |
| Warm start, warm cache, 100 K LOC | < 2 s to `ready`. |
| Incremental re-analysis, ≤ 10 changed files | < 500 ms end-to-end. |
| `report/forFile`, 100 K-LOC report | < 50 ms. |
| `duplicates/findSimilar`, ≤ 200-node snippet | < 250 ms. |

Missed budgets → `tracing::warn!` with timing breakdown. Ratchet only.

**Rules inherited.**

No regex on source, no `unwrap`, no panics, `thiserror` for library errors, structured `tracing` only, 500-line file budget, coarse E2E tests only. E2E tests drive the real LSP binary over stdio with a fixture workspace and assert against rendered deltas — never reach into `AnalysisSession` internals.

### [MCP-IPC-DISCOVERY] Client endpoint discovery

The MCP resolves the endpoint per call: try the Unix socket where the platform has one; when it is absent or refuses, read the discovery record and dial loopback (presenting the token). When neither endpoint answers, every IPC call returns `LspNotRunning` naming **both** candidate paths so `--root` mismatches stay diagnosable ([Deslop#151]). A stale discovery record whose port no longer listens maps to the same `LspNotRunning`, never a hang.

**Single-shot methods** (one request → one response, connection closes):

| Method | MCP tool consumer |
|---|---|
| `report/get` | `report-get`, `report-query`, top-offenders bookkeeping |
| `report/forFile` | `report-for-file` |
| `report/forRange` | `report-for-range` |
| `cluster/byId` | `cluster-by-id` |
| `session/config` | `session-config` |
| `duplicates/findSimilar` | `find-similar` |
| `embedding/listModels` | `list-embedding-models` |
| `deslop.lsp.refreshReport` | `rescan` |

**Long-lived subscription**:

| Method | MCP behaviour |
|---|---|
| `report/subscribe` | One frame per generation bump until the subscriber disconnects. The MCP forwards each frame as `notifications/deslop/reportChanged` to its own client. |

If no endpoint is live ([MCP-IPC-DISCOVERY]), every IPC call returns `LspNotRunning` immediately. The MCP exposes that variant to its own client with an actionable message; it does **not** fall back to a second pipeline. CI / one-shot audits use the `deslop` CLI instead.

### [PERF-BUDGET-TYPE12] Batch CLI cold-pass budget (Type-1/2, no embeddings)
A cold-cache `deslop` batch run over a 100 K-LOC C# corpus with embeddings
disabled (structural + token LSH passes only) completes in **< 30 s** on a release
binary. This is the CLI counterpart to the daemon budgets above; the embedding
pass is excluded because Ollama latency dominates and is bounded separately
([FUSION-EMBED-PROVIDER]). The budget is validated **manually** against a release
build on a real corpus — coverage-instrumented `cargo test` triples runtime, so
the E2E suite carries only a lax anti-quadratic regression guard plus correctness
assertions (every file analysed, ≥ 1 ranked cluster). Ratchet only.
