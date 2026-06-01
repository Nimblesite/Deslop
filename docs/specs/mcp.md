# MCP shell — feeding the agent in real time

Thin Model Context Protocol shell that delegates **every** read and compute call to the running `deslop-lsp` over its IPC socket ([LIVE-IPC-SOCKET], [MCP-IPC-CLIENT]). The MCP binary runs **no analysis work** — no watcher on the workspace, no `PipelineSession`, no embeddings, no on-disk cache. All analysis runs in the single long-running LSP process; the MCP serves whatever the LSP's in-memory `latest_report` says **right now**. There is no on-disk staleness window between MCP and LSP.

Crate: `crates/deslop-mcp`. Transport: JSON-RPC 2.0 over stdio per MCP spec. Under 100 LOC of glue.

### [MCP-EXTERNAL-CLIENT] External MCP client wiring

External MCP clients — Claude Code (CLI), Claude Desktop, Codex, Cursor,
Continue — run outside the VS Code host process and do not inherit the
extension's bundled `PATH`. They must be configured with an **absolute path
into the unpacked VSIX**:

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>/bin/<platform>/deslop-mcp
```

This is the same binary the VS Code extension launches for its in-process MCP
host, so the agent sees identical analysis whether it talks to Copilot Chat
inside VS Code or to Codex over its own stdio MCP. Per
[DEPLOY-EXTERNAL-MCP-CONSUMER] this is the only supported wiring outside of
the brew/scoop PATH form. Pointing a client at `target/release/deslop-mcp` or
a `cargo install` artifact silently drifts the agent's analysis off the
shipright-versioned wire contract and is forbidden by the rules in
[CLAUDE.md](../../CLAUDE.md).

Wiring examples per client live in the [root README](../../README.md#use-deslop-from-an-ai-agent-mcp)
and the paste-ready [`docs/snippets/agents-md-recipe.md`](../snippets/agents-md-recipe.md).

### [MCP-WHY-LIVE] Why the agent sees a live report

An agent that runs the CLI once at the start of a session sees a stale report after its first edit. The LSP re-analyses on every file change. The MCP reads the LSP's in-memory `latest_report` over the IPC socket on every tool call — same `LiveService` lock the LSP itself reads. The agent always works against a report that reflects the current source, with **no on-disk caching layer** in between to drift out of sync. No duplicate CPU, no extra watcher, no divergent analysis state, no race window where a hidden cluster can resurface in the MCP after the LSP has dropped it.

### [MCP-CAPABILITIES] MCP server capabilities

The MCP surface splits into **tools**, **resources**, and **notifications**.

### [MCP-TOOLS] Tools

Each tool has a JSON schema and an agent-readable description. Descriptions are written for an LLM reader — the agent reads the tool list and decides when to call each one.

**Snapshot tools** (each delegates one IPC round-trip to the LSP — `LspNotRunning` if the socket is absent):

| Tool | Inputs | Output | Description |
|---|---|---|---|
| `top-offenders` | `{ n?, max_occurrences? }` | `TopOffenders` | Fetch the worst duplicate clusters with full occurrences, interpretation, signals, bucket, and score. Start here when choosing what to fix. `max_occurrences` (default 15) caps total occurrences across returned clusters per [MCP-OCCURRENCE-BUDGET]. |
| `rescan` | `{ paths?, n?, max_occurrences? }` | `RescanPayload` | Ask the running LSP to execute `deslop.lsp.refreshReport` over IPC, then return fresh top offenders plus `generation` and `summary` change counts. If the LSP socket is absent, returns the last known generation with an empty summary. Same `max_occurrences` budget as `top-offenders`. Use when watcher lag or stale ranges are suspected. |
| `report-get` | `{ offset, limit }` (both required) | `ReportPage` | Fetch one page of the current duplication report. Worst offenders first. Call at session start; follow with `cluster-by-id` to drill in. Both `offset` and `limit` are required — the agent sizes its own context window. |
| `report-query` | `{ offset, limit, language?, bucket?, path_contains?, min_score?, min_size? }` | `ReportPage` | Filtered lookup. Use instead of `report-get` when you can describe what you're looking for. |
| `schema-doc` | `{}` | `SchemaDocPayload` | One-shot schema markdown. Call once when learning field meanings; report pages omit `schema_doc` by default to avoid repeated context bloat. |
| `report-for-file` | `{ path, max_occurrences? }` | `FileReport` | All clone clusters touching this file. Call before editing to see what's already duplicated here. `max_occurrences` (default 15) per [MCP-OCCURRENCE-BUDGET]. |
| `report-for-range` | `{ path, start_byte, end_byte, max_occurrences? }` | `[Cluster]` | Clusters overlapping the byte range you're about to edit. Same budget. |
| `cluster-by-id` | `{ id }` | `Cluster` | Fetch a cluster by its stable 16-char id. The only tool that returns full occurrence lists — `report-get` and `report-query` omit them to keep pages slim. |

**Compute tools** (delegate to LSP via [LIVE-IPC-SOCKET] — requires LSP running):

| Tool | Inputs | Output | Description |
|---|---|---|---|
| `find-similar` | `{ path?, start_byte?, end_byte?, snippet?, language?, top_n?, max_occurrences? }` | `[Cluster]` | **Before you write a new block, call this.** Runs the full structural + LSH + embedding passes on the input against the live index. Prevents introducing new clones. Returns `LspNotRunning` if LSP is absent. Same budget as `top-offenders`. See [MCP-TOOL-FINDSIMILAR]. |
| `list-embedding-models` | `{}` | `[EmbeddingModelInfo]` | Enumerate Ollama models on the host. Delegates to LSP via IPC; returns `LspNotRunning` if the socket is absent. |
| `set-embedding-model` | `{ provider_id, model_id, endpoint?, user_initiated: true }` | `EmbeddingProvenance` | Switch the live embedding model after a human-initiated request. Writes shared workspace settings and notifies the LSP via IPC. The agent must not use this autonomously. See [MCP-EMBEDDING-CONSENT]. |

**Session tool**:

| Tool | Inputs | Output | Description |
|---|---|---|---|
| `session-config` | `{}` | `SessionConfig` | min-nodes, active languages, embedding provenance, exclusion config path, cache root. One IPC round-trip; `LspNotRunning` if the socket is absent. |

All tools are source-read-only except `set-embedding-model`; `rescan` never edits source, but it may trigger the LSP's full refresh command before reloading MCP's cache and emitting report-change notifications. `set-embedding-model` requires `user_initiated: true` and may only be set after a human asked for the switch.

### [MCP-OCCURRENCE-BUDGET] Per-call total-occurrence budget

Every tool that ships full `ReportCluster` shapes — `top-offenders`, `rescan`,
`report-for-file`, `report-for-range`, `find-similar` — accepts an optional
`max_occurrences` parameter (default **15**) that bounds the **total** number
of occurrences across **all** returned clusters. The budget is the
fix for [issue #136](https://github.com/Nimblesite/Deslop/issues/136): an
unbounded `top-offenders` response on a real workspace ships 50+ occurrences
per cluster (each carrying byte ranges + paths), and that's enough to
crash some MCP clients (e.g. Codex's `rmcp_client`). No tool result is
allowed to be large enough to break the agent.

**Algorithm.** Walk the candidate clusters worst-first. Track running
`used` occurrences. For each cluster:

- If `used >= max_occurrences`: **drop this cluster and every following
  cluster.**
- Else if `cluster.occurrences.len() <= remaining_budget`: include the
  cluster fully, advance `used`.
- Else: include the cluster with `occurrences` truncated to the
  `remaining_budget`, set `occurrences_truncated = true`, mark
  `used = max_occurrences`. Stop after this cluster.

A budget that is exactly consumed by a cluster's full occurrence list
does **not** set `occurrences_truncated`; truncation only fires when a
cluster's tail was actually dropped.

**`total_occurrences`.** Every payload reports the **unfiltered**
occurrence count across every cluster the tool would have considered.
Agents read this to know how much was filtered. For
`top-offenders` / `rescan` it sums across the entire report; for
`report-for-file` / `report-for-range` it sums across the clusters
matching the path/range; for `find-similar` it sums across the matched
clusters.

**Per-cluster `occurrences_total`.** Already on `ReportCluster` from the
live-wire truncation pass. The budget keeps it accurate per cluster:
when the tail is dropped, `occurrences_total` reflects the cluster's
true count, not the truncated array length.

**`cluster-by-id`** is the escape hatch. It returns the full cluster
without applying the budget (the agent specifically asked for one
cluster), capped only by the existing live-wire `LIVE_WIRE_OCCURRENCE_CAP`
of 100. Agents that need every occurrence of a clipped cluster call
`cluster-by-id` with the cluster's stable id.

Tool descriptions in `crates/deslop-mcp/src/tools/mod.rs` lead with the
budget so an LLM reading `tools/list` sees the contract before the first
call. Tests `issue_136_top_offenders_max_occurrences_caps_response_and_reports_total`
and `issue_136_top_offenders_default_max_occurrences_is_fifteen` in
`crates/deslop-mcp/tests/cli.rs` lock the behaviour.

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
  generation, metrics, files_analysed,
  min_nodes, embedding_provenance, cache_stats, action_hints,
  total_clusters, page: { offset, limit, returned },
  clusters: [ClusterSummary, ...]
}
```

`schema_doc` is intentionally absent from `ReportPage`; agents that need the
large markdown guide call `schema-doc` or read `deslop://schema` once.

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
| `deslop://report` | Current report, canonical JSON. Each `resources/read` issues one fresh `report/get` IPC call to the LSP. |
| `deslop://schema` | The `schema_doc` block from the report. An agent new to Deslop reads this once to learn the schema. |

Content refreshes on every `resources/read` — always whatever the LSP's in-memory `latest_report` says right now.

### [MCP-IPC-CLIENT] IPC client (single source of truth)

Every read tool issues exactly one JSON-RPC request over the LSP socket. The MCP holds **no on-disk cache** and **no in-memory `Report` cache** — caching layers are exactly what create the staleness window the IPC architecture exists to eliminate. Per-call cost is one Unix-socket round-trip (sub-millisecond on localhost), bounded entirely by the LSP's `LiveService` lock contention. If the socket is missing or the LSP exits mid-call, every read returns `LspNotRunning`; the MCP does **not** fall back to a second pipeline. CI / one-shot audits are the `deslop` CLI's job, not the MCP's.

### [MCP-NOTIFICATIONS] Notifications (server → client)

On startup, the MCP opens a single long-lived `report/subscribe` connection on the LSP socket ([LIVE-IPC-SOCKET]). The LSP keeps that connection open and writes one `report/changed` notification frame per generation bump. For each frame the MCP pushes two MCP notifications under one mutex lock:

- `notifications/resources/updated` — standard MCP; payload `{ uri: "deslop://report" }`.
- `notifications/deslop/reportChanged` — custom; payload `{ generation: <u64> }`.

Both frames always arrive consecutively on the wire. Agents reconcile against the `generation` cursor. There is no file watcher; the MCP never observes the filesystem.

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
- `tools/call schema-doc` returns the same markdown as `resources/read deslop://schema`; `report-get` and `report-query` omit inline `schema_doc`.
- `tools/call cluster-by-id` returns the full `Cluster` with `occurrences[]`.
- `tools/call report-for-file` returns the expected cluster.
- `tools/call find-similar` with a known snippet returns the matching cluster above threshold (requires LSP running alongside the test).
- `tools/call find-similar` with unparseable input returns `UnparseableInputError`.
- `tools/call set-embedding-model` followed by `session-config` shows the new provenance.
- `resources/read deslop://report` returns valid canonical JSON; a follow-up file-change triggers `notifications/resources/updated`.
