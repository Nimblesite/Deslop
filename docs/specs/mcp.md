# MCP shell — feeding the agent in real time

Thin Model Context Protocol shell that reads the live report written by `deslop-lsp`. The MCP binary runs **no analysis work** — no watcher on the workspace, no `PipelineSession`, no embeddings. All analysis runs in the single long-running LSP process. The MCP reads the state file the LSP writes after every pass ([LIVE-STATE-FILE]) and, for compute-heavy operations, delegates via the LSP IPC socket ([LIVE-IPC-SOCKET]).

Crate: `crates/deslop-mcp`. Transport: JSON-RPC 2.0 over stdio per MCP spec. Under 100 LOC of glue.

### [MCP-WHY-LIVE] Why the agent sees a live report

An agent that runs the CLI once at the start of a session sees a stale report after its first edit. The LSP re-analyses on every file change and writes the new report to `.deslop-cache/live-report.json`. The MCP reads that file on every tool call — so the agent always works against a report that reflects the current source. No duplicate CPU, no extra watcher, no divergent analysis state.

### [MCP-CAPABILITIES] MCP server capabilities

The MCP surface splits into **tools**, **resources**, and **notifications**.

### [MCP-TOOLS] Tools

Each tool has a JSON schema and an agent-readable description. Descriptions are written for an LLM reader — the agent reads the tool list and decides when to call each one.

**Snapshot tools** (read from `.deslop-cache/live-report.json` — no LSP required):

| Tool | Inputs | Output | Description |
|---|---|---|---|
| `report-get` | `{ offset, limit }` (both required) | `ReportPage` | Fetch one page of the current duplication report. Worst offenders first. Call at session start; follow with `cluster-by-id` to drill in. Both `offset` and `limit` are required — the agent sizes its own context window. |
| `report-query` | `{ offset, limit, language?, bucket?, path_contains?, min_score?, min_size? }` | `ReportPage` | Filtered lookup. Use instead of `report-get` when you can describe what you're looking for. |
| `report-for-file` | `{ path }` | `FileReport` | All clone clusters touching this file. Call before editing to see what's already duplicated here. |
| `report-for-range` | `{ path, start_byte, end_byte }` | `[Cluster]` | Clusters overlapping the byte range you're about to edit. |
| `cluster-by-id` | `{ id }` | `Cluster` | Fetch a cluster by its stable 16-char id. The only tool that returns full occurrence lists — `report-get` and `report-query` omit them to keep pages slim. |

**Compute tools** (delegate to LSP via [LIVE-IPC-SOCKET] — requires LSP running):

| Tool | Inputs | Output | Description |
|---|---|---|---|
| `find-similar` | `{ path?, start_byte?, end_byte?, snippet?, language? }` | `[Cluster]` | **Before you write a new block, call this.** Runs the full structural + LSH + embedding passes on the input against the live index. Prevents introducing new clones. Returns `LspNotRunning` if LSP is absent. See [MCP-TOOL-FINDSIMILAR]. |
| `list-embedding-models` | `{}` | `[EmbeddingModelInfo]` | Enumerate Ollama models on the host. Delegates to LSP; falls back to state file embedding provenance if LSP is absent. |
| `set-embedding-model` | `{ provider_id, model_id, endpoint?, user_initiated: true }` | `EmbeddingProvenance` | Switch the live embedding model after a human-initiated request. Writes shared workspace settings and notifies the LSP via IPC. The agent must not use this autonomously. See [MCP-EMBEDDING-CONSENT]. |

**Session tool**:

| Tool | Inputs | Output | Description |
|---|---|---|---|
| `session-config` | `{}` | `SessionConfig` | min-nodes, active languages, embedding provenance, exclusion config path, cache root. Read from state file; live fields from LSP if reachable. |

All tools are pure reads except `set-embedding-model`. That tool requires `user_initiated: true` and may only be set after a human asked for the switch.

### [MCP-EMBEDDING-CONSENT] Embedding model consent

Follows [LIVE-EMBEDDING-CONSENT]. Default startup serves deterministic-only reports. An agent or host must call `list-embedding-models`, present the choice to the user, then call `set-embedding-model` with `user_initiated: true`. If no explicit human request, the only valid MCP behaviour is to leave the current model unchanged.

`set-embedding-model` writes only `deslop.embedding.*` workspace settings, never source files. If the MCP cannot write those settings, it must fail the switch instead of silently diverging from the LSP.

### [MCP-TOOL-FINDSIMILAR] `find-similar` — the keystone tool

Input variants:

1. **Range on an open file:** `{ path, start_byte, end_byte }` — the LSP already has this parsed; the query is a cache lookup + LSH + ANN probe.
2. **Snippet + language:** `{ snippet, language }` — for code the agent is *about to write*. The LSP parses in-memory, fingerprints, probes, and returns matches. Nothing is written to the cache.

Both delegate to the LSP via [LIVE-IPC-SOCKET]. Budget: < 250 ms ([LIVE-PERF-BUDGETS]).

Output: top-N clusters (default N=5) by fused score with signals, interpretation, action hints, and occurrences.

Edge cases:

- Empty snippet → empty result.
- Unparseable snippet → `UnparseableInputError` with tree-sitter error range.
- Language not registered → `UnsupportedLanguageError` listing registered languages.
- Snippet below `min-nodes` after normalisation → empty result with `below_min_nodes: true`.
- LSP not running → `LspNotRunning` error.

### [MCP-TOOL-REPORT-PAGINATION] `report-get` — slim, paginated, agent-sized

The canonical report is unbounded. `report-get` returns a **slim page** and forces the agent to pick its own page size.

**Required inputs:** `offset` (zero-based cluster index), `limit` (max clusters in this page). Omitting either is `InvalidParams`.

**Output (`ReportPage`):**

```text
{
  report_schema_version, schema_doc, generation, metrics, files_analysed,
  min_nodes, embedding_provenance, cache_stats, action_hints,
  total_clusters, page: { offset, limit, returned },
  clusters: [ClusterSummary, ...]
}
```

**`ClusterSummary`** (not `Cluster` — `members[]` and `occurrences[]` are omitted):

```text
{
  id, bucket, bucket_type, score, size_nodes, size_loc,
  occurrence_count, language,
  first_occurrence: { path, start_byte, end_byte }
}
```

`total_clusters` lets the agent decide whether to keep paging. Agents that want the top 10 ignore it; exhaustive audits page until `offset + returned >= total_clusters`.

### [MCP-TOOL-REPORT-QUERY] `report-query` — targeted lookup

Same slim page shape as [MCP-TOOL-REPORT-PAGINATION], plus filter knobs:

**Required:** `offset`, `limit`. **Optional filters** (combine with AND): `language`, `bucket`, `path_contains`, `min_score`, `min_size`. Filtering happens before pagination; `total_clusters` reflects the post-filter count.

Output echoes the filter inputs so transcripts are reproducible. Use `report-get` for the headline scan; `report-query` when the agent has a hypothesis. For "does this snippet already exist?" use `find-similar`.

### [MCP-RESOURCES] Resources

| Resource URI | Contents |
|---|---|
| `deslop://report` | Current report, canonical JSON (the state file contents). |
| `deslop://schema` | The `schema_doc` block from the report. An agent new to Deslop reads this once to learn the schema. |

Content refreshes on every `resources/read` — always the latest state file.

### [MCP-REPORT-CACHE] In-process report cache

The MCP parses `.deslop-cache/live-report.json` once on first access and caches the result in memory (`Arc<Report>` behind a lock). It watches the file for modification events (single-file `notify` watch). On change it re-reads and replaces the cached value before pushing notifications. Every tool call reads the in-memory cache — no repeated file I/O. This is the only state the MCP process holds; it never holds pipeline state or cluster fingerprints.

### [MCP-NOTIFICATIONS] Notifications (server → client)

The MCP watches `.deslop-cache/live-report.json` for modification events (single-file `notify` watch). On change it invalidates the cache ([MCP-REPORT-CACHE]) and pushes two frames under one mutex lock:

- `notifications/resources/updated` — standard MCP; payload `{ uri: "deslop://report" }`.
- `notifications/deslop/reportChanged` — custom; payload `{ generation: <u64> }`.

Both frames always arrive consecutively on the wire. Agents reconcile against the `generation` cursor.

### [MCP-AGENT-PROMPT-GUIDANCE] Tool descriptions are prompt engineering

Three rules the descriptions follow:

1. **Tell the agent when to call the tool.** `find-similar`: *"Before you write a new block, call this."*
2. **Tell the agent what the result means.** `report-get`: *"Worst offenders first."*
3. **Point to related tools.** `list-embedding-models`: *"Use before switching models."*

Descriptions are written once (the `tools/list` response) and reused in [vsix.md] docs and `deslop://schema`.

### [MCP-SAFETY] Safety + scope

- **Read-only by default.** All tools are pure reads except `set-embedding-model`, which writes only the embedding provider setting.
- **No arbitrary command execution.** No `exec`, no `shell`, no `write-file`.
- **No secrets in payloads.** Snippets echoed by `find-similar` are normalised AST summaries, not raw source.
- **One workspace per MCP session.** `initialize` pins the workspace root; `report-for-file` rejects paths outside it.

### [MCP-TESTING] E2E tests

`crates/deslop-mcp/tests/cli.rs` drives the real MCP binary over stdio with raw JSON-RPC frames. No mocking.

- `initialize` + `tools/list` returns the tools above with matching schemas and no `null` capability values.
- `tools/call report-get` requires both `offset` and `limit`; omitting either returns `InvalidParams`.
- `tools/call report-get` with a non-trivial fixture returns a `ReportPage` under the byte budget, with `total_clusters >= page.returned` and the `ClusterSummary` shape (no `members[]`, no `occurrences[]`).
- `tools/call report-get` past the end returns empty `clusters[]`, `page.returned == 0`.
- `tools/call report-query` honours each filter independently and in combination; echoed `filters` reflect inputs.
- `tools/call cluster-by-id` returns the full `Cluster` with `occurrences[]`.
- `tools/call report-for-file` returns the expected cluster.
- `tools/call find-similar` with a known snippet returns the matching cluster above threshold (requires LSP running alongside the test).
- `tools/call find-similar` with unparseable input returns `UnparseableInputError`.
- `tools/call set-embedding-model` followed by `session-config` shows the new provenance.
- `resources/read deslop://report` returns valid canonical JSON; a follow-up file-change triggers `notifications/resources/updated`.
