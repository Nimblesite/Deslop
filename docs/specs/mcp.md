# MCP shell — feeding the agent in real time

Thin Model Context Protocol shell over [LIVE-BINARY]. Lets an AI coding agent (Claude Code, Claude Desktop, Cursor, Continue, or anything that speaks MCP) **ask live questions of the running dedup analysis**: *"as I'm working on this file, which ranges I'm about to touch are already duplicated elsewhere?"* and *"before I write this block, is something like it already in the repo?"*

Crate: `crates/deslop-mcp`. Transport: JSON-RPC 2.0 over stdio per MCP spec. Stays under 100 LOC of glue; all live-analysis logic lives in `deslop-core::live` (feature-gated).

The MCP shell and the LSP shell ([lsp.md](lsp.md)) are peers — same daemon, different framing. The LSP is for the editor UI; the MCP is for the agent in the loop.

### [MCP-WHY-LIVE] Why live, not batch

An agent that runs the CLI once at the start of a session sees a stale report after its first edit. A long session with a stale report means the agent is acting on duplication data that no longer reflects the code it's been writing. The live MCP is the fix: the report **re-runs incrementally on every save** ([LIVE-SCHEDULER]), and the agent can subscribe to change notifications or pull the latest snapshot at any point.

Concretely: an agent working on a feature can, after every significant edit, call `find-similar` with the snippet it's about to write, see what's already in the repo, and refactor into a shared helper instead of introducing a new clone. Before the MCP existed, the agent had no cheap way to know.

### [MCP-CAPABILITIES] MCP server capabilities

The MCP surface splits into **tools**, **resources**, and **notifications**.

### [MCP-TOOLS] Tools

Each tool has a JSON schema and an agent-readable description. The descriptions are written for an LLM reader, not a human — the agent reads the tool list and decides when to call each one.

| Tool | Inputs | Output | Description (agent-facing) |
|---|---|---|---|
| `report-get` | `{ offset, limit }` (both required) | `ReportPage` (slim summary; see [MCP-TOOL-REPORT-PAGINATION]) | Fetch one page of the current duplication report. Worst offenders first. Returns headline metrics + a slim cluster summary slice. Call this at session start; follow up with `cluster-by-id` for any cluster you want to drill into. **Both `offset` and `limit` are required** — the agent must size its own context window. |
| `report-query` | `{ offset, limit, language?, bucket?, path_contains?, min_score?, min_size? }` | `ReportPage` (same slim summary shape) | Targeted, filterable lookup over the report. Same slim shape as `report-get` but lets the agent narrow by language, clone bucket, file substring, score floor, or subtree-size floor. Use this instead of `report-get` whenever you can describe what you're looking for. See [MCP-TOOL-REPORT-QUERY]. |
| `report-for-file` | `{ path }` | `FileReport` | All clone clusters whose occurrences touch this file. Call before editing to see what's already a duplicate here. |
| `report-for-range` | `{ path, start_byte, end_byte }` | `[Cluster]` | Clusters overlapping the byte range you're about to edit. Call before a refactor — tells you if the range is part of a larger clone family. |
| `find-similar` | `{ path?, start_byte?, end_byte?, snippet?, language? }` | `[Cluster]` | **Before you write a new block, call this.** Give either a byte range on an open file or a snippet + language. Returns existing clusters similar to the input via the full structural + LSH + embedding passes. Prevents you from introducing new clones. See [MCP-TOOL-FINDSIMILAR]. |
| `cluster-by-id` | `{ id }` | `Cluster` | Fetch a cluster by its stable 16-char id (the one shown in report text and LSP diagnostics). This is the only tool that returns full member lists + occurrence ranges — `report-get` and `report-query` deliberately omit them to keep the page slim. |
| `list-embedding-models` | `{}` | `[EmbeddingModelInfo]` | Enumerate Ollama models installed on the host plus the `stub` provider. Use before switching models; a fresh MCP live session does not run embeddings automatically. |
| `set-embedding-model` | `{ provider_id, model_id, endpoint?, user_initiated: true }` | `EmbeddingProvenance` | Explicitly select the live embedding model after a human initiated the change. The agent must not use this as an autonomous model preference or upgrade mechanism. This persists the same `deslop.embedding.*` workspace settings the VSIX/LSP reads, starts low-priority embedding work, invalidates only the embedding layer, and leaves structural + LSH results available while embeddings refresh. |
| `session-config` | `{}` | `SessionConfig` | Min-nodes, active languages, embedding provenance, exclusion config path, cache root. |

All tools are pure reads except `set-embedding-model`. That tool is not an agent-autonomous preference knob: it requires `user_initiated: true`, which the agent may only send after the user explicitly asked for the model switch. The agent must not infer consent from model availability, performance goals, or a previous session. It writes only the shared workspace embedding settings (`.vscode/settings.json` keys under `deslop.embedding.*`) so the VSIX/LSP and MCP converge on the same selected provider/model; it never edits source files.

### [MCP-EMBEDDING-CONSENT] Embedding model consent

The MCP server is a live protocol surface, so it follows [LIVE-EMBEDDING-CONSENT]. Default startup is deterministic-only (`--embeddings off`): agents can read reports, query ranges, and find deterministic duplicates without causing local model work. An agent or host must call `list-embedding-models`, present the choice to the user when the MCP client has a human in the loop, then call `set-embedding-model` with the selected provider/model and `user_initiated: true`. If there is no explicit human request to change the embedding model, the only valid MCP behaviour is to leave the current model setting unchanged.

After `set-embedding-model`, embedding refresh starts immediately at low priority with bounded batches and yield states between them. Report tools continue to serve the latest complete report while the embedding-enhanced generation is being prepared. The selected model must be persisted through the same VSIX/LSP workspace settings, not only in MCP memory, so both live surfaces remain reactive to one another across restarts and config reloads. If MCP cannot write those settings, it must fail the switch instead of silently diverging from the LSP.

### [MCP-TOOL-FINDSIMILAR] `find-similar` — the keystone tool

This is the tool that changes how the agent works. Input variants:

1. **Range on an open file:** `{ path, start_byte, end_byte }` — the daemon already has this parsed and fingerprinted; the query is a cache lookup plus an LSH + ANN probe. Budget: < 250 ms ([LIVE-PERF-BUDGETS]).
2. **Snippet + language:** `{ snippet, language }` — for code the agent is *about to write*. The daemon parses the snippet in-memory with the matching `LanguageParser`, normalises it, fingerprints it, probes LSH + ANN, and returns matching clusters. Nothing is written to the cache. Budget: < 250 ms.

Output: the top-N clusters (default N=5) by fused score, each with signals, interpretation, action hints, and occurrences. The agent decides whether to refactor by reading the same fields a human would.

Edge cases:

- Empty snippet → empty result, no error.
- Unparseable snippet → `UnparseableInputError` with the tree-sitter error range; the agent can retry with a corrected snippet.
- Language not registered → `UnsupportedLanguageError` listing the registered languages.
- Snippet smaller than `min-nodes` after normalisation → returns empty with a `below_min_nodes: true` field so the agent knows the query was too small to match, not that no clone exists.

No silent no-ops — every outcome is explicit per CLAUDE.md.

### [MCP-TOOL-REPORT-PAGINATION] `report-get` — slim, paginated, agent-sized

The full canonical report is unbounded — a real-world workspace produces megabytes of JSON. Returning that in one frame would blow out an agent's context window every time. `report-get` therefore returns a **slim page** and forces the agent to pick its own page size.

**Required inputs:**

- `offset` — non-negative integer, zero-based cluster index to start at.
- `limit` — non-negative integer, max clusters in this page. The server trusts the agent to pick a sensible value; **no implicit default** — omitting either field is an `InvalidParams` error. The agent owns its context budget.

**Output (`ReportPage`):**

```text
{
  report_schema_version,
  schema_doc,
  generation,
  metrics: { analysed_loc, duplicated_loc, duplication_percent, duplicated_files, clusters_total, threshold },
  files_analysed,
  min_nodes,
  embedding_provenance,
  cache_stats,
  action_hints,
  total_clusters,        // length of the underlying clusters[] BEFORE pagination
  page: { offset, limit, returned },
  clusters: [ClusterSummary, ...]
}
```

**`ClusterSummary` shape** (deliberately *not* `Cluster`):

```text
{
  id,                    // stable 16-char id; pass to cluster-by-id for the full record
  bucket,                // Identical | NearlyIdentical | LooselySimilar | SameBehavior
  bucket_type,           // Type-1..Type-4 (academic dual label)
  score,                 // fused ranking score (worst-first sort key)
  size_nodes,            // representative subtree size
  size_loc,              // spanned LOC across all occurrences
  occurrence_count,      // number of locations
  language,
  first_occurrence: { path, start_byte, end_byte }   // single representative location only; bytes are native, agent converts to lines on demand
}
```

`members[]` and the full `occurrences[]` array are **omitted** from the page — they live behind `cluster-by-id`. This is the load-bearing constraint: the page must stay small even when a single cluster has hundreds of members.

**Why required, not defaulted:** the spec frames the agent as the planner ([MCP-AGENT-PROMPT-GUIDANCE]). A planner that doesn't know how big a page it wants doesn't know how to use the tool. Forcing the parameter makes the agent state its budget explicitly and makes the call self-describing in transcripts.

**`total_clusters`** lets the agent decide whether to keep paging or stop. Agents that just want the top 10 ignore it; agents producing exhaustive audits page until `offset + returned >= total_clusters`.

### [MCP-TOOL-REPORT-QUERY] `report-query` — targeted lookup over the report

`report-get` is for "show me the worst stuff." `report-query` is for "show me the worst stuff matching *X*." Same slim page shape, plus filter knobs:

**Required inputs:** `offset`, `limit` (same contract as [MCP-TOOL-REPORT-PAGINATION]).

**Optional filter inputs** (all combine with logical AND):

- `language` — one of the registered language ids (`csharp`, `rust`, `python`, …). Cluster matches if its language equals this value.
- `bucket` — one of the canonical [CLONE-BUCKETS] labels. Cluster matches if its bucket equals this value.
- `path_contains` — case-sensitive substring match against any occurrence path on the cluster (workspace-relative). Cluster matches if any occurrence path contains the substring.
- `min_score` — float, fused-score floor (inclusive).
- `min_size` — integer, `size_nodes` floor (inclusive).

**Filtering happens before pagination.** `total_clusters` reflects the count *after* filters apply, so paging is consistent with the filter set.

**Output:** identical to `ReportPage` from `report-get`, plus an echo of the filter inputs so transcripts are reproducible:

```text
{
  ...ReportPage fields...,
  filters: { language, bucket, path_contains, min_score, min_size }   // null fields omitted
}
```

**When to use which:** `report-get` for the headline scan, `report-query` whenever the agent has a hypothesis ("show me LooselySimilar Rust clusters in `crates/deslop-core/src/`"). The query tool is *not* a search engine — there is no full-text snippet search, no regex, no AST query. For "I'm about to write this — does it already exist?" use `find-similar`, which has the full LSH + embedding pipeline behind it.

### [MCP-RESOURCES] Resources

MCP resources are read-only documents the agent can open by URI. Two canonical resources:

| Resource URI | Contents |
|---|---|
| `deslop://report` | The current report, canonical JSON. The agent can `resources/read` it directly instead of calling `report-get` as a tool. Resource reads are the MCP-idiomatic way to pull a document-sized payload. |
| `deslop://schema` | The `schema_doc` block from the report ([OUTPUT-SCHEMA-JSON]). Same markdown as the LSP virtual doc. An agent new to Deslop reads this once per session to learn the schema. |

Resources are listed via `resources/list`. Their content refreshes every time `resources/read` is called — the daemon always returns the latest snapshot.

### [MCP-NOTIFICATIONS] Notifications (server → client)

The MCP spec supports server-initiated notifications. Two are pushed unconditionally whenever the analysis report changes:

- `notifications/resources/updated` — standard MCP notification; payload `{ uri: "deslop://report" }`. Clients subscribed to the resource re-read it on receipt.
- `notifications/deslop/reportChanged` — custom namespace; payload `{ generation: <u64> }`. Mirrors the LSP `deslop/reportChanged` notification. Agents that reconcile against a generation cursor consume this directly without polling tool calls.

**Both notifications are pushed in the same write under one mutex lock**, so they always arrive consecutively on the wire with no interleaving.

**Two trigger points** — notifications fire from both paths:

1. **`notifications/deslop/filesChanged` handler** — after `mark_changed` updates the session state, the server pushes both frames *synchronously* before returning to the read loop. The client can call `read_frame()` twice immediately after sending the notification.
2. **Embedding refresh thread** — when a background model-swap completes, the worker thread pushes the same two frames through the shared `NotificationSender` without waiting for the next client message.

**`NotificationSender` is the shared write handle** — `Arc<Mutex<Box<dyn Write + Send>>>`. `McpServer::run` constructs it from the writer, wires it into the backend via `McpBackend::set_notification_sender`, then both the synchronous request loop and background threads use it under the same mutex so frames never interleave.

Notifications are not ordered relative to tool calls; an agent issuing a tool call mid-re-analysis gets a snapshot that's either pre- or post-pass, never partial. The `generation` field on the notification and response lets the agent reconcile.

### [MCP-AGENT-PROMPT-GUIDANCE] Tool descriptions are prompt engineering

The tool descriptions above are the user-visible contract between the daemon and the agent. They're written for the agent's planner, not for a developer. Three rules they follow:

1. **Tell the agent when to call the tool.** *"Before you write a new block, call this."* — `find-similar`. Agents don't infer usage patterns reliably from schemas; they need the use-case spelled out.
2. **Tell the agent what the result means, not just its shape.** *"Worst offenders first."* — `report-get`. *"Clusters overlapping the byte range you're about to edit."* — `report-for-range`.
3. **Point to related tools.** *"Use before switching models."* — `list-embedding-models`. This surfaces the natural call chain.

These descriptions are written in one place (the MCP shell's `tools/list` response) and reused verbatim in [vsix.md] docs and the `deslop/schema` virtual document so humans and agents read the same contract.

### [MCP-ACTIONABLE] Every result is actionable

Because the canonical JSON report already embeds `interpretation`, `action_hints`, and stable cluster ids, every tool result is self-describing. The agent doesn't need a second round-trip to understand what a cluster means — `interpretation: "Type-3 near-miss, review before merging — loop vs recursion"` is already in the payload. This is the same contract [PRINCIPLES-AUDIENCE-AGENT] nailed down for the CLI; the MCP inherits it unchanged.

### [MCP-SAFETY] Safety + scope

- **Read-only by default.** Eight tools, one writes, and the write is scoped to the embedding-provider selection. Nothing the agent can call modifies source code.
- **No arbitrary command execution.** No `exec`, no `shell`, no `write-file` tool. If the agent wants to refactor, it uses its own edit tools on the source files; the MCP only tells it *where* to refactor.
- **No secrets in payloads.** The report already refuses to embed source; snippets echoed back via `find-similar` are normalised AST summaries, not raw source.
- **One workspace per MCP session.** The `initialize` handshake pins the workspace root; later tool calls can't traverse outside it. `report-for-file` rejects paths that resolve outside the root.

### [MCP-TESTING] E2E tests

`crates/deslop-mcp/tests/cli.rs` drives the real MCP binary over stdio with the MCP JSON-RPC frames:

- `initialize` + `tools/list` returns the nine tools above with matching schemas.
- `initialize` capabilities are MCP-spec valid — every advertised capability key maps to an object, never `null`. Capabilities the server does not implement are **omitted**, not nulled. (Regression guard: a `prompts: null` / `logging: null` payload was rejected by Claude Desktop's MCP picker with `expected: object, received: null`. The test asserts no capability value is `null`.)
- `tools/call report-get` requires both `offset` and `limit`; omitting either returns `InvalidParams`.
- `tools/call report-get` with a non-trivial fixture returns a `ReportPage` whose serialised size is below a hard byte budget (the budget exists so a pathological cluster count cannot blow up the agent's context). The page contains `total_clusters >= page.returned`, and every cluster carries the `ClusterSummary` shape (no `members[]`, no full `occurrences[]`).
- `tools/call report-get` with `offset` past the end returns an empty `clusters[]`, `page.returned == 0`, and `total_clusters` unchanged.
- `tools/call report-query` honours `language`, `bucket`, `path_contains`, `min_score`, `min_size` independently and in combination; the echoed `filters` object reflects the inputs.
- `tools/call cluster-by-id` with an id discovered via `report-get` returns the full `Cluster` (with `members[]` and `occurrences[]`).
- `tools/call report-for-file` on a fixture returns the expected cluster.
- `tools/call find-similar` with a known snippet returns the matching cluster with fused score above threshold.
- `tools/call find-similar` with unparseable input returns `UnparseableInputError`.
- `tools/call set-embedding-model` followed by `tools/call session-config` shows the new provenance.
- `resources/read deslop://report` returns valid canonical JSON; a follow-up edit triggers `notifications/resources/updated`.
- `notifications/deslop/filesChanged` on a real path change **immediately** pushes `notifications/resources/updated` then `notifications/deslop/reportChanged` before the server returns to its read loop. The test calls `read_frame()` twice right after `notify()` and asserts both frames arrive without waiting for a subsequent request.

No mocking of the MCP framing — test frames are raw JSON-RPC over a pipe.
