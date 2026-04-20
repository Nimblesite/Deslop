# Daemon — long-running analysis service

CodeDedup v1 is a batch CLI. The VSIX, the LSP shell, and the MCP shell all need a **live, watcher-driven, always-up-to-date report** that updates as the user (or an AI agent) edits files. This document specifies the shared daemon that every non-CLI shell runs on top of. The CLI pipeline stays unchanged — the daemon is a thin orchestration layer over [PIPELINE-INCREMENTAL] and the `update_files(changed)` entry point promised in [pipeline.md §13](pipeline.md).

See also: [lsp.md](lsp.md), [mcp.md](mcp.md), [vsix.md](vsix.md).

### [DAEMON-BINARY] Binary + crate layout

A new crate `codededup-daemon` owns the shared service. Two thin bins link it:

- `crates/codededup-lsp` — JSON-RPC over stdio (LSP transport).
- `crates/codededup-mcp` — JSON-RPC over stdio (Model Context Protocol transport).

Both bins stay under 100 LOC of glue — transport demux, dispatch, shutdown. All logic — state, watcher, scheduler, query API — lives in `codededup-daemon`, which in turn is a shell over `codededup-core`. Nothing in `codededup-core` moves; no pipeline code is duplicated.

Dependency chain:

```
codededup-lsp ─┐
               ├─► codededup-daemon ─► codededup-core
codededup-mcp ─┘
```

The CLI binary does **not** depend on the daemon crate. CLI runs stay zero-watcher, zero-socket, zero-background-thread — identical to v1.

### [DAEMON-LIFECYCLE] Session lifecycle

One daemon process per workspace root. The client (VSIX / LSP client / MCP client) launches it, sends an `initialize` frame with the workspace root + config (min-nodes, exclusion config path, embedding settings), and the daemon:

1. Opens the `.codededup-cache/` for that root (fingerprint cache + embedding cache).
2. Runs a full initial analysis with `--incremental` semantics on — usually a warm cache on second launch, so startup is cheap.
3. Starts a file watcher ([DAEMON-WATCHER]).
4. Starts the re-analysis scheduler ([DAEMON-SCHEDULER]).
5. Sends `ready` with the initial `Report`.

Shutdown is a graceful drain: stop accepting new edits, finish the current re-analysis, flush caches, exit. The daemon never writes outside `.codededup-cache/` and never modifies source files.

### [DAEMON-STATE] In-process state

The daemon keeps one `AnalysisSession` in memory:

```rust
struct AnalysisSession {
    root: PathBuf,
    config: PipelineConfig,
    registry: FileRegistry,        // from state.rs
    fingerprints: FingerprintStore,
    embeddings: EmbeddingStore,
    latest_report: Arc<Report>,    // immutable snapshot, swapped atomically
    generation: u64,               // monotonic; bumped every re-analysis
    subscribers: Vec<Subscriber>,  // LSP/MCP clients awaiting deltas
}
```

`latest_report` is an `Arc<Report>` swapped under a lock so readers get a consistent snapshot. `generation` lets a subscriber skip forward: *"I last saw generation 42, what changed since?"* This is the same version-cursor pattern an LSP uses for document syncs.

All mutable state is reachable from `AnalysisSession`. Nothing in `codededup-core` or `codededup-daemon` adds new process-global mutable state — [STATE-FILE-REGISTRY] is still the only blessed global, and it's owned per-session.

### [DAEMON-WATCHER] File watcher

Use the `notify` crate (cross-platform, already on the v2 roadmap per [PRINCIPLES-LONG-RUNNING-DAEMON]). Watch the workspace root recursively, filtered by the same extension set registered via the `LanguageParser` trait.

Events are debounced and coalesced: a burst of saves from a formatter or refactor tool must collapse into one re-analysis pass. Debounce window is **250 ms** of quiet after the last event, capped at **2 s** of total accumulation so a stream of edits doesn't starve the scheduler.

Events that cross `[EXCLUSION-CONFIG]` `exclude` patterns are dropped before debounce — the daemon never re-parses an excluded file.

### [DAEMON-SCHEDULER] Re-analysis scheduler

After the watcher emits a coalesced changeset, the scheduler:

1. Calls `codededup_core::update_files(changed: &[FileId]) -> ReportDelta` ([pipeline.md §13]).
2. Applies the delta to `fingerprints` and `embeddings`.
3. Recomputes clustering + ranking over the updated fingerprint set.
4. Atomically swaps `latest_report`; bumps `generation`.
5. Pushes a `ReportDelta` to every subscriber.

Re-analysis is single-threaded per session — one pass in flight at a time. If a new changeset lands while one is running, it's queued; consecutive queued changesets are merged before dispatch. This keeps the daemon CPU-bounded by the incremental cost of what actually changed, never by redundant re-runs.

Budget: a coalesced changeset of ≤ 10 files with a warm fingerprint cache must complete re-analysis in **< 500 ms** on a 100 K-LOC workspace. Miss the budget → `tracing::warn!` with the timing breakdown; the budget is a perf regression guard, not a correctness assertion.

### [DAEMON-DELTA] Report deltas

`ReportDelta` is the wire-shaped diff between two generations of `Report`. Subscribers consume deltas instead of full snapshots so update traffic stays small when one file changes in a repo with thousands of clusters.

```rust
struct ReportDelta {
    from_generation: u64,
    to_generation: u64,
    clusters_added: Vec<ReportCluster>,
    clusters_removed: Vec<ClusterId>,
    clusters_updated: Vec<ReportCluster>,  // same id, different occurrences/signals
    cache_stats: CacheStats,
    tool_version: String,
}
```

Cluster ids are stable across runs ([REPORTING-CONTEXT §"How to read the report format"]) — same clone, same id, even after an edit. That stability is what makes the delta shape useful: an IDE can keep its tree view mounted and just flip colours when a cluster's signals or occurrence set changes.

Clients that miss too many generations (or connect mid-session) ask for a full snapshot via `report/get`, then resume delta consumption at the snapshot's generation.

### [DAEMON-QUERY-API] Query API (shared by LSP + MCP)

The daemon exposes a small, stable query surface that both shells forward to their clients. This is the contract the VSIX UI and the AI agent both speak.

| Method | Input | Output | Purpose |
|---|---|---|---|
| `report/get` | `{}` | `Report` | Full current snapshot. |
| `report/delta` | `{ since_generation: u64 }` | `ReportDelta` or `null` | Pull changes since a known generation. |
| `report/forFile` | `{ path: String }` | `FileReport` | All clusters whose occurrences touch this file, byte-range sorted. |
| `report/forRange` | `{ path, start_byte, end_byte }` | `Vec<ReportCluster>` | Clusters overlapping the given byte range. Powers "is the code I'm editing a duplicate of something?" |
| `cluster/byId` | `{ id: ClusterId }` | `ReportCluster` | Fetch a cluster by stable id (for "jump to other occurrences"). |
| `duplicates/findSimilar` | `{ path, start_byte, end_byte }` | `Vec<ReportCluster>` | Agent-facing: "is this snippet I'm about to write already present elsewhere?" Runs the fingerprint + LSH + embedding passes on the snippet against the live index; no cache mutation. |
| `embedding/listModels` | `{}` | `Vec<EmbeddingModelInfo>` | Enumerates Ollama models available on the host (`/api/tags`) plus the built-in `stub` provider. Powers the VSIX model picker. |
| `embedding/setModel` | `{ provider_id, model_id, endpoint? }` | `EmbeddingProvenance` | Switches the live session to the selected model. Invalidates only the embedding layer ([FUSION-EMBED-PROVIDER]); structural + LSH caches stay warm. |
| `session/config` | `{}` | `SessionConfig` | min-nodes, languages active, embedding provenance, exclusion config path, `.codededup-cache/` path. |

All methods are synchronous request/response. **No subscribe/unsubscribe primitives** on the query API — deltas are pushed (see [DAEMON-NOTIFICATIONS]). Keeping read and push separate makes the transport layering identical for LSP and MCP.

### [DAEMON-NOTIFICATIONS] Push notifications

The daemon pushes two notification types:

- `report/changed` — fires after every scheduler pass. Payload: `{ generation: u64, summary: ChangeSummary }` where `ChangeSummary` is `{ clusters_added: usize, clusters_removed: usize, clusters_updated: usize, worst_weight: f64 }`. Subscribers that want the full delta call `report/delta`.
- `analysis/state` — fires on `idle → running`, `running → idle`, and on scheduler errors. Lets the VSIX render a live status indicator without polling.

Notifications are fire-and-forget; subscribers that fall behind never block the scheduler.

### [DAEMON-PERF-BUDGETS] Performance budgets

| Scenario | Budget |
|---|---|
| Cold start, empty cache, 100 K LOC | Same as `--incremental` CLI first-run (no new budget). |
| Warm start, warm cache, 100 K LOC | < 2 s to `ready`. |
| Incremental re-analysis of ≤ 10 changed files | < 500 ms end-to-end. |
| `report/forFile` on a 100 K-LOC report | < 50 ms (index lookup, not re-analysis). |
| `duplicates/findSimilar` on a ≤ 200-node snippet | < 250 ms (one parse + one LSH/ANN probe). |

All budgets are measured per [PERF-BUDGET-TYPE12] methodology: release build, not instrumented, warmed-up JIT equivalent (= second invocation). Missed budgets surface as `tracing::warn!` with a timing breakdown; budget regressions are tracked the same way coverage is — ratchet only, never regress.

### [DAEMON-NO-REGEX-NO-SHORTCUTS] Rules inherited

Everything from CLAUDE.md still applies inside the daemon: no regex on source, no `unwrap`, no panics, `thiserror` for library errors, structured `tracing` only, 500-line file budget, coarse E2E tests only. E2E for the daemon drives the real LSP/MCP binary over stdio with a fixture workspace and asserts against rendered deltas — never reaches into `AnalysisSession` internals.
