# MCP agent workflow

## [MCP-START-WORK] Start in an unfamiliar repository

The contract is [MCP-TOOL-FILTERS-KINDS] in [the MCP specification](../specs/mcp.md). A query without `kinds` inherits the running engine's configured selection, normally identical and nearly identical. Explicit overrides affect one call. The shared page matcher, generated wire model, tool schemas, and real-process query tests implement that contract.

[MCP-TOOL-FILTERS] uses the same five filter arguments on `duplicates`, `find-similar`, and `rescan`. `tests/query/kinds.rs::every_result_list_applies_shared_filter_arguments_before_pagination` checks their public schemas, each language, kind, severity, path and size selection, empty selections, combined filters and selection before a one-row limit against complete ranked fixture clusters. Every combination of matching and rejecting predicates must preserve AND semantics across fields, union semantics within selections, complete occurrence totals and per-call isolation. Malformed filter values return `InvalidParams`; they never silently become an omitted filter, and invalid rescan arguments are rejected before refreshing the index.

An agent starts with session metadata and a small duplicates summary, narrows by path and kind, then drills into a cluster and reads the source before editing. It calls `find-similar` before adding code and follows up with an ordinary query after editing. `rescan` is recovery for a suspected stale index, not a routine step. `merge-plan` is optional and language support must be checked before relying on its output.

## [MCP-START-RECOVERY] Recover without a working MCP connection

[LIVE-IPC-SOCKET], [LIVE-IPC-TCP] and [MCP-IPC-DISCOVERY] require long workspace roots to select authenticated TCP automatically, with the same report as the CLI. ASCII, Unicode and space-containing roots must preserve full and paginated reports across independent MCP reconnects. [LSP-IPC] requires an editor error when IPC startup fails. The shared transport, LSP backend and MCP connection diagnostics implement these contracts; `crates/deslop-mcp/tests/transport/workspace_paths.rs` and `crates/deslop-lsp/tests/notifications.rs` pin them through the real binaries.

[The AI guide](../../site/src/docs/for-ai.md) and [repository instruction recipe](../snippets/agents-md-recipe.md) must describe the same fallback: read an existing canonical JSON report first; if none exists or it predates the relevant edits, generate one with the versioned bundled CLI. Cache processing is enabled by default, subject to `.deslop.toml`; never present a disk snapshot as a live snippet search or claim that an empty result from a failed tool proves uniqueness.

`tools/recovery.rs` implements this sequence in unavailable-engine error data, including absolute report and adjacent CLI paths, separate arguments, snapshot warnings, and the guide URL. `tests/query/recovery.rs` exercises the real binary without a running engine. The sidecar auto-spawn design remains proposed; recovery guidance does not claim to implement it.

[MCP-WIRE-FRAMING] also requires the shared companion-LSP header reader to return `UnexpectedEof` when a child exits before its handshake completes. `crates/deslop-test-support/src/lib.rs::read_lsp_content_length` implements this; `src/protocol/tests.rs::closed_child_stdout_returns_unexpected_eof` pins the typed error after the child has exited.

## [MCP-REPORT-FOLLOWUP] SharpLsp findings remain acceptance work

The [SharpLsp report](https://github.com/Nimblesite/SharpLsp/blob/main/docs/reviews/DESLOP-MCP-REPORT.md) describes observed behavior, not a substitute for reproductions on the current engine. Verify each complaint with fixtures and rendered output before marking it resolved. Preserve source ranges, occurrence counts, kinds, ranking, configured totals, and engine provenance in assertions.

- Clone selection: covered by `crates/deslop-mcp/src/page/tests.rs` and `tests/query/kinds.rs`, including defaults, overrides, invalid inputs, and all cluster-returning tools.
- Similarity query order: `tests/query/pagination.rs::snippet_filter_selects_a_later_pair_before_applying_the_limit` pins [MCP-TOOL-FILTERS] against a snippet matching two ranked pairs, then selects the later pair with a one-row limit. The MCP collects candidates before filtering and pagination.
- Pagination: [MCP-PAGE-CONTINUATION] is implemented by `page/continuation.rs` and `tools/limits.rs`, with real-process budget paging in `tests/query/pagination.rs` and a unit boundary assertion for marker-inclusive byte limits. A smaller returned page is never evidence that the remaining clusters disappeared.
- Workspace buffers: [LIVE-WORKSPACE-SCOPE] rejects external open/edit/close paths before retaining text. `deslop-core/tests/live/scope.rs` pins the typed rejection, new unsaved sources, and full report restoration after a buffer's parent directory is removed. Corpus updates and guards use the buffer map's canonical path identity; the one-resolution removal contract remains asserted.
- Totals: literal-heavy data, overlapping extents, and MCP/CLI comparisons require current-engine fixtures. Query selection and configured repository totals are different scopes and must be labelled accordingly.
- Navigation: summaries show a representative occurrence; a path filter may match another member. Source reads and pair comparisons remain necessary before consolidation.
- Capability limits: helper applicability, behavioral-divergence explanations, tiny named definitions, and TypeScript merge planning are separate acceptance work. Do not advertise them as implemented merely because another tool returns a related cluster.

Keep this plan, the MCP specification, the AI guides, and the corresponding executable assertions aligned as each item ships.
