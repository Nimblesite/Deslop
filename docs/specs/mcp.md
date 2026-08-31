# MCP shell — feeding the agent in real time

Thin Model Context Protocol shell that delegates **every** read and compute call to the running `deslop-lsp` over its IPC socket ([LIVE-IPC-SOCKET], [MCP-IPC-CLIENT]). The MCP binary runs **no analysis work** — no watcher on the workspace, no `PipelineSession`, no embeddings, no on-disk cache. All analysis runs in the single long-running LSP process; the MCP serves whatever the LSP's in-memory `latest_report` says **right now**. There is no on-disk staleness window between MCP and LSP.

Crate: `crates/deslop-mcp`. Transport: JSON-RPC 2.0 over stdio per MCP spec. Under 100 LOC of glue.

**External MCP client wiring.**

External MCP clients — Claude Code (CLI), Claude Desktop, Codex, Cursor,
Continue — run outside the VS Code host process and do not inherit the
extension's bundled `PATH`. They must be configured with an **absolute path
into the unpacked VSIX**:

```
~/.vscode/extensions/nimblesite.deslop-live-<VERSION>-<platform>/bin/<platform>/deslop-mcp
```

This is the same binary the VS Code extension launches for its in-process MCP
host, so the agent sees identical analysis whether it talks to Copilot Chat
inside VS Code or to Codex over its own stdio MCP. Per
[DEPLOY-EXTERNAL-MCP-CONSUMER] this is the only supported wiring outside of
the release-locked PATH forms (brew/scoop, the published fail-closed curl
installer). Pointing a client at `target/release/deslop-mcp` or
a `cargo install` artifact silently drifts the agent's analysis off the
shipright-versioned wire contract and is forbidden by the rules in
[CLAUDE.md](../../CLAUDE.md).

Wiring examples per client live in the [root README](../../README.md#use-deslop-from-an-ai-agent-mcp)
and the paste-ready [`docs/snippets/agents-md-recipe.md`](../snippets/agents-md-recipe.md).

### [MCP-WHY-LIVE] Why the agent sees a live report

An agent that runs the CLI once at the start of a session sees a stale report after its first edit. The LSP re-analyses on every file change. The MCP reads the LSP's in-memory `latest_report` over the IPC socket on every tool call — same `LiveService` lock the LSP itself reads. The agent always works against a report that reflects the current source, with **no on-disk caching layer** in between to drift out of sync. No duplicate CPU, no extra watcher, no divergent analysis state, no race window where a hidden cluster can resurface in the MCP after the LSP has dropped it.

### [MCP-CAPABILITIES] MCP server capabilities

The MCP surface splits into **tools**, **resources**, and **notifications**.

### [MCP-WIRE-FRAMING] JSON-RPC stdio framing

The MCP stdio transport carries one JSON-RPC 2.0 message per line: each frame is
UTF-8 JSON terminated by a single `\n`, written under one mutex so responses and
server→client notifications never interleave. One frame in produces zero frames
out for a notification (no `id`) or exactly one for a request. Error frames are
strict JSON-RPC envelopes — `{ "jsonrpc": "2.0", "id": <n>, "error": { "code",
"message", "data"? } }` with no extra top-level or error keys — using the
canonical reserved codes: ParseError (-32700) for malformed JSON, InvalidRequest
(-32600) for a wrong `jsonrpc` version, MethodNotFound (-32601) for an unknown
method, and InvalidParams (-32602) for bad arguments. The strict shape is what
keeps stricter clients (Codex's `rmcp_client`) from tripping on an unexpected
envelope.

### [MCP-TOOLS] Seven core analysis tools

> **Normative cutover.** The server exposes the seven core analysis tools below. The retired twelve-tool analysis-query surface is deleted wholesale; it is neither a compatibility alias nor an alternate mode. `duplicates` owns report queries, `session` owns session and embedding-model management, and `compare-pair` is the only pair-evidence tool. Orthogonal refactor tools remain governed by [AUTOFIX-MERGE-MCP] and [AUTOFIX-EXTRACT-AI-MCP-TOOLS]; they do not create another report or pair-evidence path.

**Exactly seven core analysis tools** ([DECISION-MCP-SURFACE]). Three calls carry the analysis product: `find-similar` before writing code, `duplicates` when fixing duplication, and `compare-pair` when inspecting one exact relation. The other four provide drill-in, freshness, session, and schema support. `tools/list` returns these seven plus the separately specified refactor tools that are implemented in the current build.

| Tool | Role | One-line description shape |
|---|---|---|
| `find-similar` | **Prevention keystone.** Call BEFORE writing new code. | see [MCP-TOOL-FINDSIMILAR] |
| `duplicates` | **The one report tool.** Mass-ranked clusters, worst-first; scope by path/range; filter by language or path ([MCP-TOOL-FILTERS]); `detail` picks slim or full payloads. | "Duplicate clusters ranked by mass. Start here when fixing duplicates." |
| `compare-pair` | Evidence for two explicitly identified occurrences. | "Compare exactly two occurrences and return their pair admission evidence." |
| `cluster-by-id` | Escape hatch for one full cluster, no occurrence budget. | "Full cluster record by stable id (shown in report text and LSP diagnostics)." |
| `rescan` | Force the LSP's full refresh, then return a fresh filtered `duplicates` page plus `generation` + change summary. Use when watcher lag is suspected. | accepts the same filter and pagination params as `duplicates`, plus `paths?` |
| `session` | Session metadata + embedding-model management in one tool: `action = "get"` (default) \| `"list-embedding-models"` \| `"set-embedding-model"`. | see [MCP-TOOL-SESSION] |
| `schema-doc` | One-shot schema markdown for clients with weak resource support. | "One-shot report schema markdown. Call once for field meanings; report pages omit it by default." |

All seven analysis tools are source-read-only except `session`'s `set-embedding-model` action ([MCP-EMBEDDING-CONSENT]); `rescan` never edits source, but it triggers the LSP's full refresh before returning. Refactor-tool mutation boundaries are specified under [AUTOFIX-*]. Every result is subject to the global payload cap; cap-truncation `next_action` hints name `duplicates`.

### [MCP-TOOL-FILTERS] The shared filter block

Every cluster-returning tool (`duplicates`, `rescan`, `find-similar`) accepts one identical filter
block, AND-combined, applied **before** pagination and before the occurrence budget. One schema
builder, one wire `filters` echo type, one matching function — never per-tool copies (DRY hard
rule). This is the agent-side spelling of [FACET-MODEL]; same wire vocabulary as every UI surface.

```text
languages?:      [enum]   // the core language registry (the #170/#198 anti-drift fix)
path_contains?:  string
severities?:     [enum]   // engine-stamped mass severity only
min_size?:       integer
```

Array params with empty/absent = no filtering; a one-element array is the single-value form. The
Language values derive from the canonical parser registry. Pair classification, pair evidence, literal kind, category, and confidence are not cluster filters. Filtered tools echo the applied filters so transcripts are reproducible; `total_clusters` reflects the post-filter count.

The shared pagination block rides beside it on `duplicates` and `rescan`:

```text
limit?:           integer >= 1, default 5
offset?:          integer >= 0, default 0
detail?:          "full" (default) | "summary"   // full = ReportCluster + occurrence budget;
                                                 // summary = slim ClusterSummary rows
max_occurrences?: integer >= 1, default 15        // detail = "full" only ([MCP-OCCURRENCE-BUDGET])
```

Every page preserves the engine's mass-descending, cluster-id-ascending order. The MCP has no alternate score, occurrence-count, size, or client-local sort mode.

`rescan` additionally accepts `paths?: [string]` — workspace-relative files the caller just
changed, scoping the forced refresh to those files; absent = full refresh. Paths outside the
pinned workspace root are rejected per [MCP-SAFETY].

### [MCP-TOOL-DUPLICATES] `duplicates` — one tool, every report view

Inputs: the filter block + shape block above, plus an optional scope:

- no scope — whole-workspace ranked list;
- `path` — clusters whose occurrences touch that file;
- `path` + `start_byte` + `end_byte` — clusters overlapping the byte range (range params require
  `path` and each other; violations are `InvalidParams`).

Output (`DuplicatesPage` — the one page wire type, whatever the scope or detail):

```text
{
  generation, tool_version, files_analysed, min_nodes, clusters_hidden,
  embedding_provenance, cache_stats, metrics,
  total_clusters, total_occurrences,
  page: { offset, limit, returned },
  filters: { …echo of applied filter block… },
  clusters: [ClusterSummary…] | [ReportCluster…]   // per `detail`
}
```

`ClusterSummary` (slim — no `occurrences[]`):
`{ id, mass, size_nodes, occurrence_count, language, first_occurrence: { path, start_byte, end_byte, start_line, end_line } }`.
Line numbers accompany byte offsets because humans reason in lines. The summary's `language` derives
from the canonical occurrence path via the **core parser registry's** extension map — the single
source shared with the HTML renderer, so every registered language (Dart included, #164) reports
its real id, never `"unknown"`.

`metrics` carries the headline repo totals on every page. Its **`per_file` breakdown is opt-in**
via `include_per_file: true` and empty otherwise: it holds one row per analysed file, so on a
workspace of a thousand files — or a few hundred deeply nested ones — that block alone exceeds the
whole [MCP-RESULT-SIZE-CAP] budget before a single cluster is added, which made every page overflow
([issue #286](https://github.com/Nimblesite/Deslop/issues/286)). Every `per_file` path is rendered
relative to the scan root, the same form occurrence paths use, so a report never mixes the two and
never carries the user's home directory.

`schema_doc` is intentionally absent from every page; agents call `schema-doc` or read
`deslop://schema` once. `total_clusters` lets the agent decide whether to keep paging; exhaustive
audits page until `offset + returned >= total_clusters`.

### [MCP-OCCURRENCE-BUDGET] Per-call total-occurrence budget

Every call that ships full `ReportCluster` shapes — `duplicates` / `rescan` with
`detail: "full"`, and `find-similar` — accepts `max_occurrences` (default **15**) bounding the
**total** occurrences across **all** returned clusters. The budget is the fix for
[issue #136](https://github.com/Nimblesite/Deslop/issues/136): an unbounded full-detail response on
a real workspace ships 50+ occurrences per cluster, enough to crash some MCP clients (e.g. Codex's
`rmcp_client`). No tool result is allowed to be large enough to break the agent.

**Algorithm.** Walk the candidate clusters worst-first. Track running `used` occurrences. For each
cluster:

- If `used >= max_occurrences`: **drop this cluster and every following cluster.**
- Else if `cluster.occurrences.len() <= remaining_budget`: include the cluster fully, advance `used`.
- Else: include the cluster with `occurrences` truncated to the `remaining_budget`, set
  `occurrences_truncated = true`, mark `used = max_occurrences`. Stop after this cluster.

A budget exactly consumed by a cluster's full occurrence list does **not** set
`occurrences_truncated`; truncation only fires when a tail was actually dropped.

**`total_occurrences`.** Every payload reports the **un-budgeted** occurrence count across the
full post-filter cluster set (before pagination and before the budget) so agents know how much
exists beyond what this page shipped.
**`page.returned`** counts clusters actually present in `clusters[]`, i.e. after any budget drop.
**Per-cluster `occurrences_total`** stays accurate when a tail is dropped.

**`cluster-by-id`** is the escape hatch: the full cluster, no budget (the agent asked for exactly one), capped at the live-wire occurrence cap of 100 per call. It accepts `offset?` (integer ≥ 0, default 0) over the occurrence list, so a large clone component is fully enumerable by paging until fewer than 100 come back. Dedicated literal findings are not addressable as clusters.

Tool descriptions lead with the budget so an LLM reading `tools/list` sees the contract before the
first call. The budget behaviour is pinned by the issue-136 tests named in [MCP-TESTING].

### [MCP-RESULT-SIZE-CAP] Defensive tool-result size cap

Every `tools/call` result envelope is bounded at a fixed wire budget of **200 KB**
(well under the ~512 KB most JSON-RPC clients tolerate, and below the smaller
ceiling at which Codex's `rmcp_client` crashes — issue #136). This is the outer,
last-resort guard layered on top of the per-call occurrence budget
([MCP-OCCURRENCE-BUDGET]). When a serialized payload exceeds the cap the
dispatcher drops clusters from the tail of the inner `clusters[]` array until it
fits, then stamps the response with `truncated: true`, a human-readable
`truncated_reason`, `truncated_at_bytes`, and a `next_action` pointer to the
paginated report tool; payloads with no `clusters` array — **and payloads still
over budget once every cluster has been dropped** — degrade to a stub carrying
the same markers. Draining the cluster array is not by itself success: reporting
it as one shipped an oversized payload stamped `truncated: true`, which is the
one outcome this cap exists to prevent
([issue #286](https://github.com/Nimblesite/Deslop/issues/286)). Every truncation emits a `tracing::warn!` so operators
can size their corpora. A companion budget keeps the whole `tools/list` payload
≤16 KB with each description ≤200 chars so long-form rationale stays in the
`deslop://schema` resource.

### [MCP-EMBEDDING-CONSENT] Embedding model consent

Follows [LIVE-EMBEDDING-CONSENT]. Default startup serves deterministic-only reports. An agent or host must call `session { action: "list-embedding-models" }`, present the choice to the user, then call `session { action: "set-embedding-model", … }` with `user_initiated: true`. If no explicit human request, the only valid MCP behaviour is to leave the current model unchanged.

The set action writes only `deslop.embedding.*` workspace settings, never source files. If the MCP cannot write those settings, it must fail the switch instead of silently diverging from the LSP.

### [MCP-TOOL-FINDSIMILAR] `find-similar` — the keystone tool

Input variants:

1. **Range on an open file:** `{ path, start_byte, end_byte }` — the LSP already has this parsed; the query is a cache lookup + LSH + ANN probe.
2. **Snippet + language:** `{ snippet, language }` — for code the agent is *about to write*. The LSP parses in-memory, fingerprints, probes, and returns matches. Nothing is written to the cache.

Both delegate to the LSP via [LIVE-IPC-SOCKET]. Budget: < 250 ms ([LIVE-PERF-BUDGETS]).

The tool description keeps its prevention framing: **"Call BEFORE writing new code to PREVENT duplication."** `find-similar` returns clone clusters only. Dedicated literal findings remain a separate top-level collection in the canonical report available through `deslop://report`; they are never exposed as cluster filters or cluster fields.

`find-similar` accepts the [MCP-TOOL-FILTERS] block and the uniform `limit` plus `max_occurrences` params.

Output: top-`limit` clusters in report order by duplicated mass ([RANK-MASS-SUM](pipeline.md#rank-mass-sum)), carrying occurrences, mass, and the filter echo. Pair signals, classifications, and explanations are absent; `compare-pair` owns them.

Edge cases:

- Empty snippet → empty result.
- Unparseable snippet → `UnparseableInputError` with tree-sitter error range.
- Language not registered → `UnsupportedLanguageError` listing registered languages.
- Snippet below `min-nodes` after normalisation → empty result with `below_min_nodes: true`.
- LSP not running → `LspNotRunning` error.

### [MCP-TOOL-COMPARE-PAIR] `compare-pair` — exact endpoint evidence

Input is `PairComparisonParams { left, right }`; each endpoint is `{ path, start_byte, end_byte }`, and the endpoints must be distinct occurrences within the pinned workspace. A cluster id is invalid input because the server never chooses comparison endpoints from a component.

Output is `PairComparison { left, right, evidence }`. The response echoes both endpoints and returns only that relation's structural similarity, token Jaccard, embedding cosine, content agreement, rename consistency, literal fraction, fused admission score, content-gate applicability and result, final admission result, optional pair classification, and engine-authored explanation. It contains no cluster mass. No value is cached or copied onto a cluster.

The server recomputes or retrieves the endpoint-keyed pair record through `pair/compare`. Reversing endpoint order preserves the symmetric measurements and admission result while the echoed endpoint order follows the request. Replacing either endpoint asks a different question and cannot reuse evidence from the first pair.

### [MCP-TOOL-SESSION] `session` — metadata + embedding management

One tool, three actions, replacing three plumbing tools:

- `{ }` or `{ action: "get" }` — root, min-nodes, active languages, incremental flag, embedding
  provenance, cache stats, generation counter.
- `{ action: "list-embedding-models" }` — enumerate available models (one IPC round-trip).
- `{ action: "set-embedding-model", provider_id, model_id, endpoint?, user_initiated: true }` —
  switch the live model. The schema keeps `user_initiated` as a **required const-`true`** property
  for this action, and the handler rejects its absence — the [MCP-EMBEDDING-CONSENT] invariant
  carries over verbatim. The agent must never call this autonomously.

### [MCP-RESOURCES] Resources

| Resource URI | Contents |
|---|---|
| `deslop://report` | Current report, canonical JSON. Each `resources/read` issues one fresh `report/get` IPC call to the LSP. |
| `deslop://schema` | The `schema_doc` block from the report. An agent new to Deslop reads this once to learn the schema. |

Content refreshes on every `resources/read` — always whatever the LSP's in-memory `latest_report` says right now.

### [MCP-IPC-CLIENT] IPC client (single source of truth)

Every read tool issues exactly one JSON-RPC request over the LSP's IPC endpoint. The MCP holds **no on-disk cache** and **no in-memory `Report` cache** — caching layers are exactly what create the staleness window the IPC architecture exists to eliminate. Per-call cost is one local IPC round-trip (Unix socket, or TCP loopback per [LIVE-IPC-TCP]; sub-millisecond either way), bounded entirely by the LSP's `LiveService` lock contention. If the socket is missing or the LSP exits mid-call, every read returns `LspNotRunning`; the MCP does **not** fall back to a second pipeline. CI / one-shot audits are the `deslop` CLI's job, not the MCP's.

### [MCP-NOTIFICATIONS] Notifications (server → client)

On startup, the MCP opens a single long-lived `report/subscribe` connection on the LSP socket ([LIVE-IPC-SOCKET]). The LSP keeps that connection open and writes one `report/changed` notification frame per generation bump. For each frame the MCP pushes two MCP notifications under one mutex lock:

- `notifications/resources/updated` — standard MCP; payload `{ uri: "deslop://report" }`.
- `notifications/deslop/reportChanged` — custom; payload `{ generation: <u64> }`.

Both frames always arrive consecutively on the wire. Agents reconcile against the `generation` cursor. There is no file watcher; the MCP never observes the filesystem.

### [MCP-AGENT-PROMPT-GUIDANCE] Tool descriptions are prompt engineering

Three rules the descriptions follow:

1. **Tell the agent when to call the tool.** `find-similar`: *"Before you write a new block, call this."* `duplicates`: *"Start here when fixing duplicates."*
2. **Tell the agent what the result means.** `duplicates`: *"Worst offenders first."*
3. **Keep cluster and pair vocabulary separate.** `duplicates` documents membership and mass filters; `compare-pair` documents pair classifications and evidence.

A small tool list is itself prompt engineering: every extra tool is a description the agent must read and a wrong choice it can make.

### [MCP-SAFETY] Safety + scope

- **Read-only by default.** All tools are pure reads except `session`'s `set-embedding-model` action, which writes only the embedding provider setting.
- **No arbitrary command execution.** No `exec`, no `shell`, no `write-file`.
- **No secrets in payloads.** Snippets echoed by `find-similar` are normalised AST summaries, not raw source. Literal-family `literal_value` / `constant_value` fields are capped at 80 chars ([LITERAL-WIRE]) and never logged.
- **One workspace per MCP session.** `initialize` pins the workspace root; path-scoped `duplicates` calls reject paths outside it.

### [MCP-TESTING] E2E tests

`crates/deslop-mcp/tests/cli.rs` drives the real MCP binary over stdio with raw JSON-RPC frames. No mocking.

- `initialize` + `tools/list` returns exactly the seven core analysis tools with matching schemas, plus only the separately specified refactor tools implemented in that build, and no `null` capability values.
- `duplicates {}` returns 5 full clusters, budget applied, `total_clusters` + `total_occurrences` populated.
- `duplicates { detail: "summary" }` pages: slim rows contain no `occurrences[]` or pair evidence, language is populated, and paging past the end returns empty `clusters[]` with `returned == 0`.
- Each cluster filter is honoured independently and in combination; language values come from the parser registry; pair classifications and evidence fields are rejected as cluster filters.
- Scope: `duplicates { path }` returns the expected cluster; `{ path, start_byte, end_byte }` honours the range; range params without `path` → `InvalidParams`.
- Cluster results remain in engine mass order; no tool-local sort can replace that order.
- `cluster-by-id` returns the full clone cluster with `occurrences[]`, membership, canonical extent, mass, and rank fields only.
- `rescan` returns `generation` + change summary + a filtered page; the issue-135/137/153 freshness behaviours hold (stale-watcher recovery, fresh generation, no stale ranges).
- `session {}` returns the config; `list-embedding-models` action lists; `set-embedding-model` without `user_initiated: true` is rejected; with it, a follow-up `session {}` shows the new provenance.
- `schema-doc` returns the same markdown as `resources/read deslop://schema`; pages omit inline `schema_doc`.
- `find-similar` keystone cases remain: snippet match, unparseable input, below-min-nodes, and limit handling.
- `compare-pair` requires two concrete endpoints and returns only that pair's `S`, `J`, `E`, `A`, `R`, literal fraction, admission result, and classification.
- `resources/read deslop://report` returns valid canonical JSON; a follow-up file-change triggers `notifications/resources/updated`.
