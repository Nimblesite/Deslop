# MCP agent workflow

## [MCP-START-WORK] Start in an unfamiliar repository

The contract is [MCP-TOOL-FILTERS-KINDS] in [the MCP specification](../specs/mcp.md). A query without `kinds` inherits the running engine's configured selection, normally identical and nearly identical. Explicit overrides affect one call. The shared page matcher, generated wire model, tool schemas, and real-process query tests implement that contract.

An agent starts with session metadata and a small duplicates summary, narrows by path and kind, then drills into a cluster and reads the source before editing. It calls `find-similar` before adding code and follows up with an ordinary query after editing. `rescan` is recovery for a suspected stale index, not a routine step. `merge-plan` is optional and language support must be checked before relying on its output.

## [MCP-START-RECOVERY] Recover without a working MCP connection

[The AI guide](../../site/src/docs/for-ai.md) and [repository instruction recipe](../snippets/agents-md-recipe.md) must describe the same fallback: read an existing canonical JSON report first; if none exists or it predates the relevant edits, generate one with the versioned bundled CLI. Cache processing is enabled by default, subject to `.deslop.toml`; never present a disk snapshot as a live snippet search or claim that an empty result from a failed tool proves uniqueness.

`tools/recovery.rs` implements this sequence in unavailable-engine error data, including absolute report and adjacent CLI paths, separate arguments, snapshot warnings, and the guide URL. `tests/query/recovery.rs` exercises the real binary without a running engine. The sidecar auto-spawn design remains proposed; recovery guidance does not claim to implement it.

## [MCP-REPORT-FOLLOWUP] SharpLsp findings remain acceptance work

The [SharpLsp report](https://github.com/Nimblesite/SharpLsp/blob/main/docs/reviews/DESLOP-MCP-REPORT.md) describes observed behavior, not a substitute for reproductions on the current engine. Verify each complaint with fixtures and rendered output before marking it resolved. Preserve source ranges, occurrence counts, kinds, ranking, configured totals, and engine provenance in assertions.

- Clone selection: covered by `crates/deslop-mcp/src/page/tests.rs` and `tests/query/kinds.rs`, including defaults, overrides, invalid inputs, and all cluster-returning tools.
- Similarity query order: `tests/query/pagination.rs::snippet_filter_selects_a_later_pair_before_applying_the_limit` pins [MCP-TOOL-FILTERS] against a snippet matching two ranked pairs, then selects the later pair with a one-row limit. The MCP collects candidates before filtering and pagination.
- Pagination: [MCP-PAGE-CONTINUATION] is implemented by `page/continuation.rs` and `tools/limits.rs`, with real-process budget paging in `tests/query/pagination.rs` and a unit boundary assertion for marker-inclusive byte limits. A smaller returned page is never evidence that the remaining clusters disappeared.
- Workspace buffers: [LIVE-WORKSPACE-SCOPE] rejects external open/edit/close paths before retaining text. `deslop-core/tests/live/scope.rs` pins both the typed rejection and the unchanged complete clusters and metrics. Pipeline ingestion already guards canonical root membership; live entry points now use their existing range-query guard too.
- Totals: literal-heavy data, overlapping extents, and MCP/CLI comparisons require current-engine fixtures. Query selection and configured repository totals are different scopes and must be labelled accordingly.
- Navigation: summaries show a representative occurrence; a path filter may match another member. Source reads and pair comparisons remain necessary before consolidation.
- Capability limits: helper applicability, behavioral-divergence explanations, tiny named definitions, and TypeScript merge planning are separate acceptance work. Do not advertise them as implemented merely because another tool returns a related cluster.

Keep this plan, the MCP specification, the AI guides, and the corresponding executable assertions aligned as each item ships.
