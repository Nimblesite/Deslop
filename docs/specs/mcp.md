# MCP shell — feeding the agent in real time

Thin Model Context Protocol shell over [LIVE-BINARY]. Lets an AI coding agent (Claude Code, Claude Desktop, Cursor, Continue, or anything that speaks MCP) **ask live questions of the running dedup analysis**: *"as I'm working on this file, which ranges I'm about to touch are already duplicated elsewhere?"* and *"before I write this block, is something like it already in the repo?"*

Crate: `crates/codededup-mcp`. Transport: JSON-RPC 2.0 over stdio per MCP spec. Stays under 100 LOC of glue; all logic lives in `codededup-daemon`.

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
| `report-get` | `{}` | `Report` (canonical JSON) | Fetch the current full duplication report. Worst offenders first. Call this at session start, or when you want a full picture. |
| `report-for-file` | `{ path }` | `FileReport` | All clone clusters whose occurrences touch this file. Call before editing to see what's already a duplicate here. |
| `report-for-range` | `{ path, start_byte, end_byte }` | `[Cluster]` | Clusters overlapping the byte range you're about to edit. Call before a refactor — tells you if the range is part of a larger clone family. |
| `find-similar` | `{ path?, start_byte?, end_byte?, snippet?, language? }` | `[Cluster]` | **Before you write a new block, call this.** Give either a byte range on an open file or a snippet + language. Returns existing clusters similar to the input via the full structural + LSH + embedding passes. Prevents you from introducing new clones. See [MCP-TOOL-FINDSIMILAR]. |
| `cluster-by-id` | `{ id }` | `Cluster` | Fetch a cluster by its stable 16-char id (the one shown in report text and LSP diagnostics). |
| `list-embedding-models` | `{}` | `[EmbeddingModelInfo]` | Enumerate Ollama models installed on the host plus the `stub` provider. Use before switching models. |
| `set-embedding-model` | `{ provider_id, model_id, endpoint? }` | `EmbeddingProvenance` | Switch the live embedding model. Invalidates only the embedding layer; structural + LSH caches stay warm. |
| `session-config` | `{}` | `SessionConfig` | Min-nodes, active languages, embedding provenance, exclusion config path, cache root. |

All tools are pure reads except `set-embedding-model`. No tool writes source files; no tool modifies the workspace. That guarantee is part of the agent-facing contract.

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

### [MCP-RESOURCES] Resources

MCP resources are read-only documents the agent can open by URI. Two canonical resources:

| Resource URI | Contents |
|---|---|
| `codededup://report` | The current report, canonical JSON. The agent can `resources/read` it directly instead of calling `report-get` as a tool. Resource reads are the MCP-idiomatic way to pull a document-sized payload. |
| `codededup://schema` | The `schema_doc` block from the report ([OUTPUT-SCHEMA-JSON]). Same markdown as the LSP virtual doc. An agent new to CodeDedup reads this once per session to learn the schema. |

Resources are listed via `resources/list`. Their content refreshes every time `resources/read` is called — the daemon always returns the latest snapshot.

### [MCP-NOTIFICATIONS] Notifications (server → client)

The MCP spec supports server-initiated notifications. We use two:

- `notifications/resources/updated` — fired against `codededup://report` after every scheduler pass. Clients that subscribed to the resource re-read it.
- `notifications/codededup/reportChanged` — custom namespace, carries a `{ generation, summary }` payload mirroring the LSP `report/changed`. Agents that keep a cursor on the report generation consume this directly.

Notifications are not ordered relative to tool calls; an agent issuing a tool call mid-re-analysis gets a snapshot that's either pre- or post-pass, never partial. The `generation` field on the response lets the agent reconcile.

### [MCP-AGENT-PROMPT-GUIDANCE] Tool descriptions are prompt engineering

The tool descriptions above are the user-visible contract between the daemon and the agent. They're written for the agent's planner, not for a developer. Three rules they follow:

1. **Tell the agent when to call the tool.** *"Before you write a new block, call this."* — `find-similar`. Agents don't infer usage patterns reliably from schemas; they need the use-case spelled out.
2. **Tell the agent what the result means, not just its shape.** *"Worst offenders first."* — `report-get`. *"Clusters overlapping the byte range you're about to edit."* — `report-for-range`.
3. **Point to related tools.** *"Use before switching models."* — `list-embedding-models`. This surfaces the natural call chain.

These descriptions are written in one place (the MCP shell's `tools/list` response) and reused verbatim in [vsix.md] docs and the `codededup/schema` virtual document so humans and agents read the same contract.

### [MCP-ACTIONABLE] Every result is actionable

Because the canonical JSON report already embeds `interpretation`, `action_hints`, and stable cluster ids, every tool result is self-describing. The agent doesn't need a second round-trip to understand what a cluster means — `interpretation: "Type-3 near-miss, review before merging — loop vs recursion"` is already in the payload. This is the same contract [PRINCIPLES-AUDIENCE-AGENT] nailed down for the CLI; the MCP inherits it unchanged.

### [MCP-SAFETY] Safety + scope

- **Read-only by default.** Eight tools, one writes, and the write is scoped to the embedding-provider selection. Nothing the agent can call modifies source code.
- **No arbitrary command execution.** No `exec`, no `shell`, no `write-file` tool. If the agent wants to refactor, it uses its own edit tools on the source files; the MCP only tells it *where* to refactor.
- **No secrets in payloads.** The report already refuses to embed source; snippets echoed back via `find-similar` are normalised AST summaries, not raw source.
- **One workspace per MCP session.** The `initialize` handshake pins the workspace root; later tool calls can't traverse outside it. `report-for-file` rejects paths that resolve outside the root.

### [MCP-TESTING] E2E tests

`crates/codededup-mcp/tests/cli.rs` drives the real MCP binary over stdio with the MCP JSON-RPC frames:

- `initialize` + `tools/list` returns the eight tools above with matching schemas.
- `tools/call report-for-file` on a fixture returns the expected cluster.
- `tools/call find-similar` with a known snippet returns the matching cluster with fused score above threshold.
- `tools/call find-similar` with unparseable input returns `UnparseableInputError`.
- `tools/call set-embedding-model` followed by `tools/call session-config` shows the new provenance.
- `resources/read codededup://report` returns valid canonical JSON; a follow-up edit triggers `notifications/resources/updated`.

No mocking of the MCP framing — test frames are raw JSON-RPC over a pipe.
